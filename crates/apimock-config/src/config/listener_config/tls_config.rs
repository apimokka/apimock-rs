use std::path::Path;

use serde::Deserialize;

/// tls/ssl connection
#[derive(Clone, Deserialize)]
#[non_exhaustive]
pub struct TlsConfig {
    pub key: String,
    pub cert: String,
    pub port: Option<u16>,
    /// RFC 074 S-07: how long an incomplete TLS handshake may hold a
    /// connection before it's dropped. `None` uses
    /// [`TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS`](super::super::constant::TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS).
    pub handshake_timeout_seconds: Option<u64>,
    /// RFC 074 S-07: maximum concurrent HTTPS connections. Beyond this,
    /// new connections wait for a slot rather than being refused —
    /// see `Server::serve_https`. `None` uses
    /// [`TLS_DEFAULT_MAX_CONNECTIONS`](super::super::constant::TLS_DEFAULT_MAX_CONNECTIONS).
    pub max_connections: Option<usize>,
}

impl TlsConfig {
    /// Validate that the cert/key files exist. RFC 074 S-08: this used
    /// to return a bare `bool`, losing the specific missing path by the
    /// time the caller could report anything — the same defect RFC 065
    /// fixed for `ServiceConfig::validate` (see that method's doc
    /// comment). A malformed-but-present PEM is not caught here — that
    /// needs an actual parse, which belongs to `apimock-server`'s
    /// `tls::load_certs`/`load_private_key`, not this crate. Existence
    /// is what this layer can and should check before ever handing the
    /// path to the parser.
    pub fn validate(&self) -> Result<(), String> {
        if !Path::new(self.key.as_str()).exists() {
            return Err(format!("tls private key is missing: {}", self.key));
        }
        if !Path::new(self.cert.as_str()).exists() {
            return Err(format!("tls certificate is missing: {}", self.cert));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tls_config(cert: &str, key: &str) -> TlsConfig {
        TlsConfig {
            cert: cert.to_owned(),
            key: key.to_owned(),
            port: None,
            handshake_timeout_seconds: None,
            max_connections: None,
        }
    }

    /// RFC 074 S-08 acceptance: "a missing certificate file: same [as
    /// malformed PEM — exits, names the file]". This is the "names the
    /// file" half — `Config::validate` (one layer up) is what turns
    /// this `Err` into the fatal `ConfigError::Validation` that stops
    /// startup before any listener binds.
    #[test]
    fn validate_names_a_missing_cert_file() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("key.pem");
        std::fs::write(&key_path, b"irrelevant for this test").unwrap();
        let missing_cert = dir.path().join("does_not_exist_cert.pem");

        let result =
            tls_config(missing_cert.to_str().unwrap(), key_path.to_str().unwrap()).validate();

        let err = result.expect_err("missing cert file must fail validation");
        assert!(
            err.contains(missing_cert.to_str().unwrap()),
            "error must name the missing cert file: {err}"
        );
    }

    /// Same as above, for the key — checked first, so it must not be
    /// masked by a coincidentally-also-missing cert.
    #[test]
    fn validate_names_a_missing_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("cert.pem");
        std::fs::write(&cert_path, b"irrelevant for this test").unwrap();
        let missing_key = dir.path().join("does_not_exist_key.pem");

        let result =
            tls_config(cert_path.to_str().unwrap(), missing_key.to_str().unwrap()).validate();

        let err = result.expect_err("missing key file must fail validation");
        assert!(
            err.contains(missing_key.to_str().unwrap()),
            "error must name the missing key file: {err}"
        );
    }

    #[test]
    fn validate_succeeds_when_both_files_exist() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("cert.pem");
        let key_path = dir.path().join("key.pem");
        std::fs::write(&cert_path, b"irrelevant for this test").unwrap();
        std::fs::write(&key_path, b"irrelevant for this test").unwrap();

        let result = tls_config(cert_path.to_str().unwrap(), key_path.to_str().unwrap()).validate();

        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }
}
