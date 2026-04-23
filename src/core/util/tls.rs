use std::path::PathBuf;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

use crate::core::error::{AppError, AppResult, TlsKind};

/// Load TLS/SSL certificates (leaf + any intermediates) from a PEM file.
///
/// # Why this returns `AppResult` instead of panicking
///
/// In 4.6.x this function used `.expect(...)` and panicked on any read
/// error. Because TLS paths come from user config, a typo was enough to
/// abort the process with a backtrace. Returning `AppResult` lets the
/// caller surface a single readable line and exit cleanly.
pub fn load_certs(file_path: &str) -> AppResult<Vec<CertificateDer<'static>>> {
    let path = PathBuf::from(file_path);

    // Collecting eagerly means a PEM parse error in the middle of the
    // file surfaces before we try to hand the cert chain to rustls —
    // the later rustls error would be much less descriptive.
    let iter = CertificateDer::pem_file_iter(file_path).map_err(|e| AppError::TlsLoad {
        kind: TlsKind::Certificate,
        path: path.clone(),
        reason: e.to_string(),
    })?;

    let mut certs = Vec::new();
    for (idx, item) in iter.enumerate() {
        let cert = item.map_err(|e| AppError::TlsLoad {
            kind: TlsKind::Certificate,
            path: path.clone(),
            reason: format!("failed to parse certificate #{}: {}", idx + 1, e),
        })?;
        certs.push(cert);
    }

    if certs.is_empty() {
        return Err(AppError::TlsLoad {
            kind: TlsKind::Certificate,
            path,
            reason: "no certificates found in PEM file".to_owned(),
        });
    }

    Ok(certs)
}

/// Load a TLS/SSL private key from a PEM file.
///
/// Accepts any PEM-encoded key format that `rustls-pki-types` understands
/// (PKCS#8, PKCS#1, SEC1). We don't narrow the accepted format because
/// rustls itself is tolerant and picking one here would just make the
/// error message less obvious ("wrong format" vs. the real "file missing").
pub fn load_private_key(file_path: &str) -> AppResult<PrivateKeyDer<'static>> {
    PrivateKeyDer::from_pem_file(file_path).map_err(|e| AppError::TlsLoad {
        kind: TlsKind::PrivateKey,
        path: PathBuf::from(file_path),
        reason: e.to_string(),
    })
}
