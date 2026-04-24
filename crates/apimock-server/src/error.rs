//! Errors produced by server-level operations.
//!
//! See `apimock_routing::error` for the rationale on per-crate error
//! types. `ServerError` wraps `ConfigError` via `#[from]` because
//! server startup calls `Config::new` on the user's behalf.

use std::{io, path::PathBuf};

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, thiserror::Error)]
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

#[derive(Debug, Clone, Copy)]
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
