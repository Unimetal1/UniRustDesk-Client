use hbb_common::{
    anyhow::{ensure, Context},
    config::{self, Config, RELAY_PORT, RENDEZVOUS_PORT},
    protobuf::Message as _,
    rendezvous_proto::{self, RendezvousMessage, RequestRelay},
    sodiumoxide::{self, crypto::secretbox},
    tokio, ResultType, Stream,
};
pub use hbb_common::socket_client::check_port;
pub use librustdesk::common::{get_key, secure_tcp};

#[path = "../src/cert_auth.rs"]
mod cert_auth;
#[path = "../src/platform/windows/certificate_auth.rs"]
pub(crate) mod certificate_auth;
mod platform {
    pub(crate) use crate::certificate_auth;
}
mod lang {
    pub fn translate(message: String) -> String {
        librustdesk::flutter_ffi::translate(message, "en".into()).0
    }
}

#[tokio::main]
async fn main() -> ResultType<()> {
    sodiumoxide::init().map_err(|_| hbb_common::anyhow::anyhow!("sodium initialization failed"))?;
    ensure!(option_env!("CNG_SERVER_ADDRESS").is_none(), "Build this test without CNG_SERVER_ADDRESS");
    let host = std::env::var("RUSTDESK_TEST_HOST")?;
    let public_key = std::env::var("RUSTDESK_TEST_PUBLIC_KEY")?;
    *config::APP_NAME.write().unwrap() = "RustDesk-136-Compatibility-Test".into();
    config::OVERWRITE_SETTINGS.write().unwrap().extend([
        ("custom-rendezvous-server".into(), host.clone()),
        ("key".into(), public_key.clone()),
    ]);
    let mut id = cert_auth::connect(&host, false, 5_000).await?;
    let mut nat = RendezvousMessage::new();
    nat.set_test_nat_request(rendezvous_proto::TestNatRequest::new());
    id.send(&nat).await?;
    let answer = id.next_timeout(5_000).await.context("ID server timeout")??;
    ensure!(RendezvousMessage::parse_from_bytes(&answer)?.has_test_nat_response(), "ID protocol mismatch");
    println!("PASS actual client CNG authentication and encrypted ID request");
    drop(id);

    let udp = tokio::net::UdpSocket::bind((host.as_str(), 0)).await?;
    let peer_id = format!("136{}", std::process::id());
    let mut registration = RendezvousMessage::new();
    registration.set_register_pk(rendezvous_proto::RegisterPk {
        id: peer_id.clone(),
        uuid: vec![19; 16].into(),
        pk: sodiumoxide::crypto::sign::gen_keypair().0.as_ref().to_vec().into(),
        ..Default::default()
    });
    udp.send_to(&registration.write_to_bytes()?, (host.as_str(), RENDEZVOUS_PORT as u16)).await?;
    let mut buffer = [0; 65536];
    let (length, _) = hbb_common::timeout(5_000, udp.recv_from(&mut buffer)).await??;
    let reply = RendezvousMessage::parse_from_bytes(&buffer[..length])?;
    ensure!(reply.has_register_pk_response(), "Missing registration response");
    ensure!(reply.register_pk_response().result.enum_value() == Ok(rendezvous_proto::register_pk_response::Result::OK), "Registration rejected");
    registration.set_register_peer(rendezvous_proto::RegisterPeer {
        id: peer_id.clone(),
        ..Default::default()
    });
    udp.send_to(&registration.write_to_bytes()?, (host.as_str(), RENDEZVOUS_PORT as u16)).await?;
    let (length, _) = hbb_common::timeout(5_000, udp.recv_from(&mut buffer)).await??;
    let reply = RendezvousMessage::parse_from_bytes(&buffer[..length])?;
    ensure!(reply.has_register_peer_response() && !reply.register_peer_response().request_pk, "Peer address registration failed");
    println!("PASS receiver UDP registration with historical protocol");

    let session = hbb_common::uuid::Uuid::new_v4().to_string();
    let mut relay_request = RendezvousMessage::new();
    relay_request.set_request_relay(RequestRelay {
        id: peer_id,
        uuid: session.clone(),
        licence_key: public_key.clone(),
        ..Default::default()
    });
    let mut id = cert_auth::connect(&host, false, 5_000).await?;
    id.send(&relay_request).await?;
    let (length, _) = hbb_common::timeout(5_000, udp.recv_from(&mut buffer)).await??;
    let request = RendezvousMessage::parse_from_bytes(&buffer[..length])?;
    ensure!(request.has_request_relay() && request.request_relay().uuid == session, "Relay negotiation mismatch");
    println!("PASS authenticated ID server forwards relay request to receiver");

    let mut receiver = Stream::new(check_port(&host, RELAY_PORT), None, 5_000).await?;
    let mut receiver_request = RendezvousMessage::new();
    receiver_request.set_request_relay(RequestRelay {
        uuid: session,
        licence_key: public_key,
        ..Default::default()
    });
    receiver.send(&receiver_request).await?;
    let mut technician = cert_auth::connect(&host, true, 5_000).await?;
    technician.send(&relay_request).await?;
    let session_key = secretbox::gen_key();
    receiver.set_key(session_key.clone());
    technician.set_key(session_key);
    for size in [64, 4096, 1024 * 1024] {
        let payload: Vec<u8> = (0..size).map(|index| (index % 251) as u8).collect();
        technician.send_raw(payload.clone()).await?;
        let received = receiver.next_timeout(10_000).await.context("Receiver timeout")??;
        ensure!(received.as_ref() == payload, "Technician to receiver payload mismatch");
        receiver.send_raw(payload.clone()).await?;
        let received = technician.next_timeout(10_000).await.context("Technician timeout")??;
        ensure!(received.as_ref() == payload, "Receiver to technician payload mismatch");
    }
    println!("PASS actual client transport/session encryption: bidirectional 64 B, 4 KiB, 1 MiB");
    ensure!(Config::get_option("key") == std::env::var("RUSTDESK_TEST_PUBLIC_KEY")?, "Test configuration changed");
    Ok(())
}
