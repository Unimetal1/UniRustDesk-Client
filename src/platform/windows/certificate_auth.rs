use hbb_common::{
    anyhow::{anyhow, Context},
    sodiumoxide::crypto::hash::sha256,
    ResultType,
};
use schannel::{cert_store::CertStore, ncrypt_key::NcryptKey, RawPointer};
use windows::{core::w, Win32::Security::Cryptography::*};

pub(crate) fn sign_challenge(challenge: &[u8]) -> ResultType<Vec<u8>> {
    let store_flags = CERT_OPEN_STORE_FLAGS(CERT_SYSTEM_STORE_LOCAL_MACHINE)
        | CERT_STORE_READONLY_FLAG
        | CERT_STORE_OPEN_EXISTING_FLAG;
    // unsafe obejmuje wywołania WinAPI i wskaźniki — podobnie jak P/Invoke w C#.
    let store = unsafe {
        let store_handle = CertOpenStore(
            CERT_STORE_PROV_SYSTEM_W,
            CERT_QUERY_ENCODING_TYPE::default(),
            None,
            store_flags,
            Some(w!("MY").as_ptr().cast()),
        )
        .context(crate::lang::translate(
            "service-certificate-store-error".to_owned(),
        ))?;
        // CertStore zamknie uchwyt przy wyjściu z funkcji, tak jak using / SafeHandle.
        CertStore::from_ptr(store_handle.0)
    };

    let mut selected_certificate = None;
    for certificate in store.certs() {
        // Porównujemy całe CN, żeby nie wybrać np. certyfikatu RustDesk-Inny.
        let common_name = unsafe {
            let certificate_context = certificate.as_ptr().cast();
            let common_name_oid = Some(szOID_COMMON_NAME.as_ptr().cast());
            let name_length = CertGetNameStringW(
                certificate_context,
                CERT_NAME_ATTR_TYPE,
                0,
                common_name_oid,
                None,
            );
            let mut name_buffer = vec![0u16; name_length as usize];
            CertGetNameStringW(
                certificate_context,
                CERT_NAME_ATTR_TYPE,
                0,
                common_name_oid,
                Some(&mut name_buffer),
            );
            String::from_utf16_lossy(&name_buffer)
                .trim_end_matches('\0')
                .to_owned()
        };
        if common_name == "RustDesk" && certificate.is_time_valid()? {
            if selected_certificate.is_some() {
                return Err(anyhow!(
                    "{}",
                    crate::lang::translate("service-certificate-multiple".to_owned())
                ));
            }
            selected_certificate = Some(certificate);
        }
    }
    let certificate = match selected_certificate {
        Some(value) => value,
        None => {
            return Err(anyhow!(
                "{}",
                crate::lang::translate("service-certificate-missing".to_owned())
            ))
        }
    };

    let key_flags = CRYPT_ACQUIRE_ONLY_NCRYPT_KEY_FLAG
        | CRYPT_ACQUIRE_COMPARE_KEY_FLAG
        | CRYPT_ACQUIRE_SILENT_FLAG;
    let mut key_handle = HCRYPTPROV_OR_NCRYPT_KEY_HANDLE::default();
    let mut must_free_key = windows::core::BOOL::default();
    unsafe {
        CryptAcquireCertificatePrivateKey(
            certificate.as_ptr().cast(),
            key_flags,
            None,
            &mut key_handle,
            None,
            Some(&mut must_free_key),
        )
        .context(crate::lang::translate(
            "service-certificate-private-key-error".to_owned(),
        ))?;
    }
    // Windows wskazuje, kto ma zwolnić uchwyt. Nie wolno zamknąć pożyczonego uchwytu.
    // Zmienna musi żyć do końca funkcji: jej automatyczne sprzątanie zastępuje using.
    let _owned_key = if must_free_key.as_bool() {
        Some(unsafe { NcryptKey::from_ptr(key_handle.0 as *mut _) })
    } else {
        None
    };

    let challenge_hash = sha256::hash(challenge);
    let padding = BCRYPT_PKCS1_PADDING_INFO {
        pszAlgId: w!("SHA256"),
    };
    let padding_pointer = &padding as *const BCRYPT_PKCS1_PADDING_INFO;
    let mut signature = vec![0u8; 1024];
    let mut signature_length = 0;
    // Podpis powstaje w CNG. Klucz prywatny nie jest eksportowany, a jego ACL pozostaje bez zmian.
    unsafe {
        NCryptSignHash(
            NCRYPT_KEY_HANDLE(key_handle.0),
            Some(padding_pointer.cast()),
            &challenge_hash.0,
            Some(&mut signature),
            &mut signature_length,
            NCRYPT_PAD_PKCS1_FLAG,
        )?;
    }
    signature.truncate(signature_length as usize);
    if signature.len() < 384 || signature.len() > 1024 {
        return Err(anyhow!("RSA key must be 3072 to 8192 bits"));
    }
    Ok(signature)
}
