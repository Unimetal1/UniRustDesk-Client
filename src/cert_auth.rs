use hbb_common::{
    anyhow::anyhow,
    config::{Config, RELAY_PORT, RENDEZVOUS_PORT},
    socket_client::connect_tcp_local,
    tokio, ResultType, Stream,
};

pub async fn connect(server: &str, use_relay_server: bool, timeout_ms: u64) -> ResultType<Stream> {
    let server_host = hostname_without_port(server);
    let configured_host = hostname_without_port(&Config::get_rendezvous_server());
    if !server_host.eq_ignore_ascii_case(&configured_host) {
        return Err(anyhow!("Certificate authentication requires the configured ID server hostname"));
    }

    let (port, server_role) = if use_relay_server {
        (RELAY_PORT + 2, 1)
    } else {
        (RENDEZVOUS_PORT + 2, 0)
    };
    let address = crate::check_port(&server_host, port);
    let mut stream = connect_tcp_local(address, None, timeout_ms).await?;
    let server_key = crate::get_key(true).await;
    crate::secure_tcp(&mut stream, &server_key).await?;
    if !stream.is_secured() {
        return Err(anyhow!("Server did not establish an encrypted connection"));
    }

    let mut expected_prefix = b"RustDesk technician v1\0".to_vec();
    expected_prefix.push(server_role);
    let challenge = match stream.next_timeout(15_000).await {
        Some(result) => result?,
        None => return Err(anyhow!("Certificate challenge missing")),
    };
    if challenge.len() != expected_prefix.len() + 32 || !challenge.starts_with(&expected_prefix) {
        return Err(anyhow!("Invalid certificate challenge"));
    }

    #[cfg(target_os = "windows")]
    let signing_result = tokio::task::spawn_blocking(move || {
        crate::platform::certificate_auth::sign_challenge(&challenge)
    }).await?;
    #[cfg(not(target_os = "windows"))]
    let signing_result: ResultType<Vec<u8>> = Err(anyhow!("Technician authentication requires Windows CNG"));
    let signature = signing_result?;
    stream.send_raw(signature).await?;

    let reply = match stream.next_timeout(15_000).await {
        Some(result) => result?,
        None => return Err(anyhow!("Technician certificate rejected")),
    };
    if reply.as_ref() != b"OK" {
        return Err(anyhow!("Technician certificate rejected"));
    }
    Ok(stream)
}

fn hostname_without_port(server: &str) -> String {
    if let Ok(address) = server.parse::<std::net::SocketAddr>() {
        return address.ip().to_string();
    }
    if let Some((hostname, port)) = server.rsplit_once(':') {
        if !hostname.contains(':') && port.parse::<u16>().is_ok() {
            return hostname.to_owned();
        }
    }
    server.trim_start_matches('[').trim_end_matches(']').to_owned()
}
