//! TLS certificate loading and hot-reload support (RFC 020).
//!
//! # Two TLS setup modes
//!
//! | Mode | When used | Cert changes |
//! |---|---|---|
//! | `with_single_cert` | startup (static) | require restart |
//! | `ReloadableCertResolver` | startup with hot-reload | soft reload via `reload_certs` |
//!
//! # Outcome C (RFC 020)
//!
//! - `TlsCertFile` / `TlsKeyFile` changes are `SoftReload` (no listener rebind).
//! - `TlsEnabled` toggle is still `HardRestart` (changes the listener type).
//! - In-progress TLS handshakes that started before a reload complete with
//!   the old cert; new handshakes use the new cert atomically.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;

use crate::error::{ServerError, ServerResult, TlsKind};

// ── PEM loaders (unchanged from pre-5.11) ───────────────────────────────

/// Load TLS/SSL certificates (leaf + any intermediates) from a PEM file.
pub fn load_certs(file_path: &str) -> ServerResult<Vec<CertificateDer<'static>>> {
    let path = PathBuf::from(file_path);
    let iter = CertificateDer::pem_file_iter(file_path).map_err(|e| ServerError::TlsLoad {
        kind: TlsKind::Certificate,
        path: path.clone(),
        reason: e.to_string(),
    })?;

    let mut certs = Vec::new();
    for (idx, item) in iter.enumerate() {
        let cert = item.map_err(|e| ServerError::TlsLoad {
            kind: TlsKind::Certificate,
            path: path.clone(),
            reason: format!("failed to parse certificate #{}: {}", idx + 1, e),
        })?;
        certs.push(cert);
    }

    if certs.is_empty() {
        return Err(ServerError::TlsLoad {
            kind: TlsKind::Certificate,
            path,
            reason: "no certificates found in PEM file".to_owned(),
        });
    }

    Ok(certs)
}

/// Load a TLS/SSL private key from a PEM file.
pub fn load_private_key(file_path: &str) -> ServerResult<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(file_path).map_err(|e| ServerError::TlsLoad {
        kind: TlsKind::PrivateKey,
        path: PathBuf::from(file_path),
        reason: e.to_string(),
    })
}

// ── CertifiedKey builder ─────────────────────────────────────────────────

/// Error returned when `ReloadableCertResolver::reload_from_paths` fails.
#[derive(Debug, Clone)]
pub struct TlsReloadError {
    pub reason: String,
}

impl fmt::Display for TlsReloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TLS cert reload failed: {}", self.reason)
    }
}

impl std::error::Error for TlsReloadError {}

/// Build a `CertifiedKey` from DER-encoded cert chain and private key.
///
/// Returns an error if the key cannot be parsed by the active crypto backend.
fn make_certified_key(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<CertifiedKey, TlsReloadError> {
    let signing_key =
        rustls::crypto::ring::sign::any_supported_type(&key).map_err(|e| TlsReloadError {
            reason: format!("unsupported private key type: {}", e),
        })?;
    Ok(CertifiedKey::new(certs, signing_key))
}

// ── ReloadableCertResolver ───────────────────────────────────────────────

/// A [`ResolvesServerCert`] implementation that supports atomic in-place
/// certificate rotation without restarting the listener (RFC 020).
///
/// # Usage
///
/// 1. Build with [`ReloadableCertResolver::new`] at server startup.
/// 2. Pass `Arc::clone(&resolver)` to the server loop and keep one Arc in
///    `ServerHandle::cert_reloader`.
/// 3. Call [`reload_from_paths`] when the GUI applies a `TlsCertFile` or
///    `TlsKeyFile` change.  The swap is atomic: in-progress handshakes
///    complete with the old cert; new handshakes use the new cert.
///
/// [`reload_from_paths`]: ReloadableCertResolver::reload_from_paths
#[derive(Debug)]
pub struct ReloadableCertResolver {
    inner: RwLock<Arc<CertifiedKey>>,
}

impl ReloadableCertResolver {
    /// Create a new resolver from DER-encoded cert and key material.
    pub fn new(
        certs: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<Self, TlsReloadError> {
        let ck = make_certified_key(certs, key)?;
        Ok(Self {
            inner: RwLock::new(Arc::new(ck)),
        })
    }

    /// Reload certificates from PEM files on disk.
    ///
    /// If loading or parsing fails, the old certificate remains active and
    /// this method returns an error describing the failure.
    pub fn reload_from_paths(&self, cert_path: &str, key_path: &str) -> Result<(), TlsReloadError> {
        let certs = load_certs(cert_path).map_err(|e| TlsReloadError {
            reason: e.to_string(),
        })?;
        let key = load_private_key(key_path).map_err(|e| TlsReloadError {
            reason: e.to_string(),
        })?;
        let new_ck = make_certified_key(certs, key)?;

        let mut guard = self.inner.write().map_err(|_| TlsReloadError {
            reason: "cert RwLock poisoned".to_owned(),
        })?;
        *guard = Arc::new(new_ck);
        log::info!("TLS certificate reloaded successfully from {}", cert_path);
        Ok(())
    }
}

impl ResolvesServerCert for ReloadableCertResolver {
    fn resolve(&self, _client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        self.inner.read().ok().map(|g| Arc::clone(&*g))
    }
}

// ── ServerConfig builder helpers ──────────────────────────────────────────

/// Build a static (non-reloadable) `ServerConfig` for the common case.
///
/// Used when the caller doesn't need hot-reload (e.g. integration test
/// environments, or once TLS-toggle is still `HardRestart`).
pub fn build_server_config_static(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<ServerConfig, String> {
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("failed to build rustls ServerConfig: {}", e))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

/// Build a `ServerConfig` backed by a [`ReloadableCertResolver`].
///
/// Returns the config and an `Arc` to the resolver so the caller can later
/// call `reload_from_paths` without locking the entire config.
pub fn build_server_config_reloadable(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<(ServerConfig, Arc<ReloadableCertResolver>), TlsReloadError> {
    let resolver = Arc::new(ReloadableCertResolver::new(certs, key)?);
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::clone(&resolver) as Arc<dyn ResolvesServerCert>);
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok((config, resolver))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_pem_file(path: &str, content: &str) {
        std::fs::write(path, content).unwrap();
    }

    // Minimal self-signed ECDSA P-256 cert + key, generated with:
    //   openssl ecparam -genkey -name P-256 -noout -out key.pem
    //   openssl req -new -x509 -key key.pem -out cert.pem -days 3650 -subj "/CN=test"
    const TEST_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\n\
MIIBczCCARmgAwIBAgIUNNKjB+m5H6ZCjEPHNFEL5GYW3/UwCgYIKoZIzj0EAwIw\n\
DzENMAsGA1UEAwwEdGVzdDAeFw0yNjA1MjIwMjQ0NTZaFw0zNjA1MTkwMjQ0NTZa\n\
MA8xDTALBgNVBAMMBHRlc3QwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQTo544\n\
m3Yk+4kNlcFXR8RL5rtGVqrZohzvanN7oUiIYXzpofwYNBLqLg9AOZPeiX32aizX\n\
wqEBuYMV4B6gBj1Ho1MwUTAdBgNVHQ4EFgQUxoL28LxPMcYmwNAvUCIaaZp02xAw\n\
HwYDVR0jBBgwFoAUxoL28LxPMcYmwNAvUCIaaZp02xAwDwYDVR0TAQH/BAUwAwEB\n\
/zAKBggqhkjOPQQDAgNIADBFAiEA2IO7sD+CIM4OWZkF0SMCmrnus/xQbNFBICXg\n\
YNQ/K+oCIGlsqHA+PmxwUknuDDS5dQF26iNztRz2PY4diIfWxLNi\n\
-----END CERTIFICATE-----\n";

    const TEST_KEY_PEM: &str = "-----BEGIN EC PRIVATE KEY-----\n\
MHcCAQEEIBK3C/2yAvhbvjxP7f5aCgVZN9udnXStns0xKk7LQ3RnoAoGCCqGSM49\n\
AwEHoUQDQgAEE6OeOJt2JPuJDZXBV0fES+a7Rlaq2aIc72pze6FIiGF86aH8GDQS\n\
6i4PQDmT3ol99mos18KhAbmDFeAeoAY9Rw==\n\
-----END EC PRIVATE KEY-----\n";

    #[test]
    fn load_certs_returns_error_for_missing_file() {
        let result = load_certs("/nonexistent/cert.pem");
        assert!(result.is_err());
    }

    #[test]
    fn load_private_key_returns_error_for_missing_file() {
        let result = load_private_key("/nonexistent/key.pem");
        assert!(result.is_err());
    }

    #[test]
    fn reloadable_resolver_init_and_reload_bad_path_keeps_old_cert() {
        let cert_path = "/tmp/apimock_test_cert.pem";
        let key_path = "/tmp/apimock_test_key.pem";
        write_pem_file(cert_path, TEST_CERT_PEM);
        write_pem_file(key_path, TEST_KEY_PEM);

        let certs = load_certs(cert_path).expect("load test cert");
        let key = load_private_key(key_path).expect("load test key");
        let resolver = ReloadableCertResolver::new(certs, key).expect("build resolver");

        // Reload from bad paths → error, resolver must not crash.
        let result = resolver.reload_from_paths("/no/cert.pem", "/no/key.pem");
        assert!(result.is_err(), "expected error for missing paths");

        // The resolver's inner cert is still readable (lock not poisoned).
        let guard = resolver.inner.read().unwrap();
        drop(guard);
    }

    #[test]
    fn reloadable_resolver_reload_from_same_files_succeeds() {
        let cert_path = "/tmp/apimock_test_cert2.pem";
        let key_path = "/tmp/apimock_test_key2.pem";
        write_pem_file(cert_path, TEST_CERT_PEM);
        write_pem_file(key_path, TEST_KEY_PEM);

        let certs = load_certs(cert_path).expect("load test cert");
        let key = load_private_key(key_path).expect("load test key");
        let resolver = ReloadableCertResolver::new(certs, key).expect("build resolver");

        // Re-loading from the same valid files should succeed.
        let result = resolver.reload_from_paths(cert_path, key_path);
        assert!(
            result.is_ok(),
            "reload from same valid files must succeed: {:?}",
            result
        );
    }

    #[test]
    fn build_server_config_reloadable_returns_resolver() {
        let cert_path = "/tmp/apimock_test_cert3.pem";
        let key_path = "/tmp/apimock_test_key3.pem";
        write_pem_file(cert_path, TEST_CERT_PEM);
        write_pem_file(key_path, TEST_KEY_PEM);

        let certs = load_certs(cert_path).unwrap();
        let key = load_private_key(key_path).unwrap();
        let result = build_server_config_reloadable(certs, key);
        assert!(
            result.is_ok(),
            "build_server_config_reloadable failed: {:?}",
            result
        );
        let (_config, resolver) = result.unwrap();
        // Resolver should be usable after config is built.
        let reload = resolver.reload_from_paths(cert_path, key_path);
        assert!(reload.is_ok());
    }
}
