use serde::Deserialize;

use super::constant::{LISTENER_DEFAULT_IP_ADDRESS, LISTENER_DEFAULT_PORT};
use tls_config::TlsConfig;

pub mod tls_config;

/// verbose logs
#[derive(Clone, Deserialize)]
#[non_exhaustive]
pub struct ListenerConfig {
    pub ip_address: String,
    pub port: u16,
    pub tls: Option<TlsConfig>,
}

impl ListenerConfig {
    /// validate. `Result<(), String>`, not `bool`, for the same reason
    /// `TlsConfig::validate` changed (RFC 074 S-08) — the caller needs
    /// the specific reason, not just pass/fail.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(tls) = self.tls.as_ref() {
            tls.validate()?;
        }
        Ok(())
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
