//! HTTP(S) server runtime for apimock.
//!
//! # Responsibilities
//!
//! - Listener binding and the accept loop (HTTP + HTTPS).
//! - Per-request dispatch (OPTIONS / middleware / rule sets / dyn_route).
//! - Compiling Rhai middlewares from paths recorded in config.
//! - Building `hyper::Response` values from a matched `Respond`.
//!
//! # What is deliberately *not* here
//!
//! - Rule-set *matching logic*. That's `apimock-routing`.
//! - Config parsing / validation. That's `apimock-config`.
//! - Server restart policy. The brief is explicit (§7): a running
//!   server does not restart itself. It may emit [`ReloadHint`]; an
//!   external control layer decides what to do.

pub mod constant;
pub mod control;
pub mod dyn_route;
pub mod error;
pub mod http_util;
pub mod json_path_util;
pub mod middleware;
pub mod parsed_request;
pub mod respond_response;
pub mod respond_util;
pub mod response;
pub mod response_handler;
pub mod server;
pub mod tls;
pub mod trace;
pub mod types;

pub use control::{ReloadHint, ServerControl, ServerHandle, ServerState};
pub use error::{ServerError, ServerResult, TlsKind};
pub use middleware::{LoadedMiddlewares, MiddlewareHandler};
pub use server::{AppState, Server, service};
