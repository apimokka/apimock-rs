use std::path::Path;

use serde::Deserialize;

/// tls/ssl connection
#[derive(Clone, Deserialize)]
pub struct TlsConfig {
    pub key: String,
    pub cert: String,
    pub port: Option<u16>,
}

impl TlsConfig {
    /// validate
    pub fn validate(&self) -> bool {
        if !Path::new(self.key.as_str()).exists() {
            log::error!("tls private key is missing: {}", self.key);
            return false;
        }
        if !Path::new(self.cert.as_str()).exists() {
            log::error!("tls certificate is missing: {}", self.cert);
            return false;
        }
        true
    }
}
