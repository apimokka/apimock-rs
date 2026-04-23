//! Application-level error type.
//!
//! # Why a dedicated `AppError`
//!
//! Earlier versions of `apimock` signalled failures in several different
//! ways across the codebase:
//!
//! - raw `panic!` / `expect` / `unwrap` in startup paths (config loading,
//!   TLS setup, Rhai compilation, etc.),
//! - `Result<_, String>` for some fallible helpers,
//! - implicit success by returning a default value on error.
//!
//! Those styles each had drawbacks. Panics make the process abort with a
//! backtrace that is hard to read for an end user who merely mistyped a
//! path in `apimock.toml`. `String` errors lose type information and
//! force the caller to format the message themselves. Default-on-error
//! silently hides real problems from operators.
//!
//! `AppError` consolidates all *startup-time* failures (parsing config,
//! compiling Rhai scripts, reading TLS material, etc.) into one typed
//! enum. Callers use `?` to propagate, and the binary entry point prints
//! a single readable line — no backtrace spam for expected user errors.
//!
//! # What is *not* here
//!
//! Per-request failures (a response-body serialization error, a missing
//! file on disk for a single HTTP request) deliberately do **not** become
//! `AppError`. Those are translated into HTTP 4xx/5xx responses by the
//! response helpers in `core::server::response`. Turning every such case
//! into a typed error would couple the server loop to the startup error
//! model without any real benefit — the server has to produce an HTTP
//! response either way.
//!
//! In short: `AppError` is the error model for "the process failed to
//! start" or "the operator gave us bad config". The HTTP layer keeps its
//! own, separate error flow.

use std::{io, path::PathBuf};

/// Result alias used throughout the startup and configuration layers.
pub type AppResult<T> = Result<T, AppError>;

/// All fatal, startup-time errors that `apimock` produces.
///
/// Every variant carries enough context (path, reason) to produce a
/// single-line message without the caller having to add more text.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The config TOML file could not be read from disk.
    #[error("failed to read config file `{path}`: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// The config TOML file was read, but could not be parsed.
    ///
    /// The `canonical` field, when present, is the canonicalized absolute
    /// path — helpful when the user passed a relative path and isn't sure
    /// which file on disk was actually opened.
    #[error("invalid TOML in `{path}`{canonical_display}: {source}", canonical_display = match canonical {
        Some(p) => format!(" ({})", p.display()),
        None => String::new(),
    })]
    ConfigParse {
        path: PathBuf,
        canonical: Option<PathBuf>,
        #[source]
        source: toml::de::Error,
    },

    /// A rule-set file listed in `service.rule_sets` could not be read.
    #[error("failed to read rule set file `{path}`: {source}")]
    RuleSetRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// A rule-set file was read, but could not be parsed as TOML.
    #[error("invalid rule set TOML in `{path}`{canonical_display}: {source}", canonical_display = match canonical {
        Some(p) => format!(" ({})", p.display()),
        None => String::new(),
    })]
    RuleSetParse {
        path: PathBuf,
        canonical: Option<PathBuf>,
        #[source]
        source: toml::de::Error,
    },

    /// A configured middleware (Rhai) file was missing on disk.
    #[error("middleware script not found: `{path}`")]
    MiddlewareMissing { path: PathBuf },

    /// A middleware file was found but failed to compile.
    ///
    /// We carry the rhai error as a boxed string instead of the raw type
    /// because `compile_file` returns `Box<EvalAltResult>` — which doesn't
    /// implement `std::error::Error` cleanly across versions. Stringifying
    /// at the boundary keeps `AppError` portable without losing the
    /// human-readable message.
    #[error("failed to compile middleware `{path}`: {reason}")]
    MiddlewareCompile { path: PathBuf, reason: String },

    /// The TLS certificate or private key could not be loaded.
    #[error("TLS material load failed ({kind}) at `{path}`: {reason}")]
    TlsLoad {
        kind: TlsKind,
        path: PathBuf,
        reason: String,
    },

    /// A listener address failed to resolve or bind.
    #[error("invalid listener address `{addr}`: {reason}")]
    ListenerAddress { addr: String, reason: String },

    /// Startup-time validation failed (a validator returned `false`).
    ///
    /// The detailed reason has already been logged via `log::error!` at
    /// the call site; this variant marks that the process must abort.
    #[error("configuration validation failed")]
    Validation,

    /// A path on disk could not be canonicalized (used for relative-path
    /// resolution when computing respond dirs).
    #[error("failed to resolve path `{path}`: {source}")]
    PathResolve {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Catch-all for I/O that doesn't have a more specific variant.
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
}

/// Which half of a TLS keypair failed to load.
///
/// We split this out rather than using two separate `AppError` variants so
/// that the variant list stays shorter and `Display` output stays uniform.
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
