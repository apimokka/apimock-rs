//! Stage-1 control/introspection API for the server.
//!
//! # Scope (5.0.0)
//!
//! Per the brief (§4.3, §5.3, §7), the server crate exposes:
//!
//! - A minimal handle the embedder can hold.
//! - A "state" enum the embedder can poll.
//! - A reload-hint type.
//!
//! Critically, **the server does not restart itself**. When a config
//! edit would need the listener rebuilt, the server-side code only
//! emits a hint; an external control layer (GUI / supervisor) decides
//! whether to restart. That's the brief's §7 rule and the shape here
//! reflects it.
//!
//! Implementation of actual shutdown / reload wiring is stage-2 work.
//! In 5.0.0 these types exist with placeholder methods so downstream
//! code can start coding against them.

use serde::Serialize;

/// Handle an embedder holds to interact with a running server.
///
/// # Why it doesn't expose a `.restart()`
///
/// The brief is specific: "restart は server crate の内部責務にしない"
/// (restart is not a server-crate responsibility). A `ServerHandle`
/// carries read-only introspection and a shutdown signal — nothing
/// more. If a change requires the listener to rebind a new port, the
/// embedder tears the server down and constructs a fresh one.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ServerHandle {
    /// Address the HTTP listener is bound to, if any.
    pub http_addr: Option<std::net::SocketAddr>,
    /// Address the HTTPS listener is bound to, if any.
    pub https_addr: Option<std::net::SocketAddr>,
}

/// Small control surface for the embedder.
///
/// 5.0.0 ships this as a placeholder; stage-2 adds the actual
/// shutdown-signal channel + reload trigger implementation.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ServerControl {}

impl ServerControl {
    pub fn new() -> Self {
        Self {}
    }
}

/// What the server is doing right now.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ServerState {
    /// The listener is being brought up.
    Starting,
    /// Requests are being served normally.
    Running,
    /// Shutdown has been requested; drains are in flight.
    ShuttingDown,
    /// The listener is no longer accepting connections.
    Stopped,
}

/// How much of the server needs to restart after a config change.
///
/// # Why this type also lives in `apimock-config`
///
/// The config crate defines the *same* `ReloadHint` enum as its
/// `view::ReloadHint` because it's what an `ApplyResult` carries — the
/// GUI consumes the hint via the config-layer API without pulling the
/// server crate. The server's copy exists so that runtime code can
/// also produce hints without depending on the config crate's
/// GUI-shaped module. Both types convert into each other trivially.
#[derive(Clone, Copy, Debug, Serialize)]
pub enum ReloadHint {
    /// No reload required.
    None,
    /// Rule sets / middlewares need to reload.
    Reload,
    /// Listener configuration changed; need a full restart.
    Restart,
}

impl From<apimock_config::ReloadHint> for ReloadHint {
    fn from(value: apimock_config::ReloadHint) -> Self {
        match value {
            apimock_config::ReloadHint::None => ReloadHint::None,
            apimock_config::ReloadHint::Reload => ReloadHint::Reload,
            apimock_config::ReloadHint::Restart => ReloadHint::Restart,
        }
    }
}

impl From<ReloadHint> for apimock_config::ReloadHint {
    fn from(value: ReloadHint) -> Self {
        match value {
            ReloadHint::None => apimock_config::ReloadHint::None,
            ReloadHint::Reload => apimock_config::ReloadHint::Reload,
            ReloadHint::Restart => apimock_config::ReloadHint::Restart,
        }
    }
}
