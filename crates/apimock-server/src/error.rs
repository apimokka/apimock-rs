//! Errors produced by server-level operations.
//!
//! See `apimock_routing::error` for the rationale on per-crate error
//! types. `ServerError` wraps `ConfigError` via `#[from]` because
//! server startup calls `Config::new` on the user's behalf.
//!
//! # `#[non_exhaustive]` and `kind()` (RFC 041)
//!
//! `ServerError` is `#[non_exhaustive]` and gains `kind()` /
//! `ServerErrorKind`, one variant per `ServerError` variant — no
//! delegation into `ConfigErrorKind` for the wrapped `Config` variant,
//! same reasoning as `WorkspaceError`'s in `apimock_config::error`. No
//! variant here carries a `toml::de::Error`, so nothing in this enum
//! needed boxing — `apimock_config::error`'s module doc has the full
//! reasoning for the two variants (elsewhere) that did.

use std::{io, path::PathBuf};

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// TLS certificate or private key failed to load.
    #[error("TLS material load failed ({kind}) at `{path}`: {reason}")]
    TlsLoad {
        kind: TlsKind,
        path: PathBuf,
        reason: String,
    },

    /// Listener address failed to resolve or bind.
    #[error("invalid listener address `{addr}`: {reason}")]
    ListenerAddress { addr: String, reason: String },

    /// A middleware file listed in config was missing on disk.
    #[error("middleware script not found: `{path}`")]
    MiddlewareMissing { path: PathBuf },

    /// A middleware file was found but failed to compile.
    #[error("failed to compile middleware `{path}`: {reason}")]
    MiddlewareCompile { path: PathBuf, reason: String },

    /// Catch-all for plain I/O that doesn't have a more specific variant.
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),

    /// Forwarded from the config crate when server startup triggers
    /// config loading.
    #[error(transparent)]
    Config(#[from] apimock_config::ConfigError),
}

/// `ServerError`'s failure class.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerErrorKind {
    TlsLoad,
    ListenerAddress,
    MiddlewareMissing,
    MiddlewareCompile,
    Io,
    Config,
}

impl ServerError {
    pub fn kind(&self) -> ServerErrorKind {
        match self {
            ServerError::TlsLoad { .. } => ServerErrorKind::TlsLoad,
            ServerError::ListenerAddress { .. } => ServerErrorKind::ListenerAddress,
            ServerError::MiddlewareMissing { .. } => ServerErrorKind::MiddlewareMissing,
            ServerError::MiddlewareCompile { .. } => ServerErrorKind::MiddlewareCompile,
            ServerError::Io(_) => ServerErrorKind::Io,
            ServerError::Config(_) => ServerErrorKind::Config,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum TlsKind {
    Certificate,
    PrivateKey,
}

impl std::fmt::Display for TlsKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsKind::Certificate => f.write_str("certificate"),
            TlsKind::PrivateKey => f.write_str("private key"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 041 § 6: kind() — one assertion per variant.
    #[test]
    fn server_error_kind_matches_every_variant() {
        assert_eq!(
            ServerError::TlsLoad {
                kind: TlsKind::Certificate,
                path: PathBuf::from("x"),
                reason: "x".to_owned(),
            }
            .kind(),
            ServerErrorKind::TlsLoad
        );
        assert_eq!(
            ServerError::ListenerAddress {
                addr: "x".to_owned(),
                reason: "x".to_owned(),
            }
            .kind(),
            ServerErrorKind::ListenerAddress
        );
        assert_eq!(
            ServerError::MiddlewareMissing {
                path: PathBuf::from("x"),
            }
            .kind(),
            ServerErrorKind::MiddlewareMissing
        );
        assert_eq!(
            ServerError::MiddlewareCompile {
                path: PathBuf::from("x"),
                reason: "x".to_owned(),
            }
            .kind(),
            ServerErrorKind::MiddlewareCompile
        );
        assert_eq!(
            ServerError::Io(io::Error::other("x")).kind(),
            ServerErrorKind::Io
        );
        assert_eq!(
            ServerError::Config(apimock_config::ConfigError::Validation {
                reason: "x".to_owned()
            })
            .kind(),
            ServerErrorKind::Config
        );
    }
}
