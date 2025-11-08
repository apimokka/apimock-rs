use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};

/// load tls/ssl cert
pub fn load_certs(file_path: &str) -> Vec<CertificateDer<'static>> {
    CertificateDer::pem_file_iter(file_path)
        .expect("cannot open certificate file")
        .map(|result| result.unwrap())
        .collect()
}

/// load tls/ssl private key
pub fn load_private_key(file_path: &str) -> PrivateKeyDer<'static> {
    PrivateKeyDer::from_pem_file(file_path).expect("cannot read private key file")
}
