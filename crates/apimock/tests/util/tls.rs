use std::{fs, path::PathBuf};

use rcgen::{date_time_ymd, CertificateParams, DistinguishedName, KeyPair, SanType};

use super::file::file_path_from_cargo_manifest_dir;

use super::constant::tls::{CERT_FILE_PATH, KEY_FILE_PATH};

/// generate certificates for https connection as pem files
pub fn generate_tls_credentials() {
    let mut params: CertificateParams = Default::default();
    params.not_before = date_time_ymd(1975, 1, 1);
    params.not_after = date_time_ymd(4096, 1, 1);
    params.distinguished_name = DistinguishedName::new();
    params.subject_alt_names = vec![SanType::DnsName(
        "localhost".try_into().expect("failed to generate dns name"),
    )];

    let key_pair = KeyPair::generate().expect("failed to generate key pair by rcgen");
    let cert = params
        .self_signed(&key_pair)
        .expect("failed to sign by self by rcgen");

    let pem_serialized = cert.pem();

    fs::write(cert_file_path(), pem_serialized.as_bytes()).expect("failed to write cert.pem");
    fs::write(key_file_path(), key_pair.serialize_pem().as_bytes())
        .expect("failed to write key.pem");
}

/// check if certificates are ready
pub fn tls_credentials_are_ready() -> bool {
    cert_file_path().exists() && key_file_path().exists()
}

/// cert file path from project root dir
pub fn cert_file_path() -> PathBuf {
    file_path_from_cargo_manifest_dir(CERT_FILE_PATH)
}

/// private key file path from project root dir
pub fn key_file_path() -> PathBuf {
    file_path_from_cargo_manifest_dir(KEY_FILE_PATH)
}
