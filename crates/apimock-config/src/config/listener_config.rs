use serde::Deserialize;

use super::constant::{LISTENER_DEFAULT_IP_ADDRESS, LISTENER_DEFAULT_PORT};
use tls_config::TlsConfig;

mod tls_config;

/// verbose logs
#[derive(Clone, Deserialize)]
pub struct ListenerConfig {
    pub ip_address: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

impl ListenerConfig {
    /// validate
    pub fn validate(&self) -> bool {
        if let Some(tls) = self.tls.as_ref() {
            if !tls.validate() {
                return false;
            }
        }
        true
    }
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            ip_address: LISTENER_DEFAULT_IP_ADDRESS.to_owned(),
            port: LISTENER_DEFAULT_PORT,
            tls: None,
        }
    }
}
