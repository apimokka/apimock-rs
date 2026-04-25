//! A developer-friendly, featherlight and functional HTTP(S) mock server
//! built in Rust, based on [hyper](https://hyper.rs/) and [tokio](https://tokio.rs/).
//!
//! # Workspace layout (5.0.0+)
//!
//! Starting with 5.0.0, apimock is split across four crates. This crate
//! (`apimock`) is a thin façade that re-exports the three member crates
//! and provides the CLI entry point:
//!
//! - [`apimock_config`] — the TOML config model; loading, validation,
//!   and (coming in stage 2) the GUI-facing edit API.
//! - [`apimock_routing`] — rule-set definitions, matching, and the
//!   read-only view types a GUI binds against.
//! - [`apimock_server`] — HTTP listener, request dispatch, middleware
//!   compilation.
//!
//! Most existing code that did `use apimock::core::…` can migrate by
//! replacing the prefix with the appropriate re-export below. For
//! example, `apimock::core::server::routing::rule_set::RuleSet` is now
//! `apimock::routing::RuleSet` (or, equivalently,
//! `apimock_routing::RuleSet`).

pub mod app;
pub mod args;
mod logger;

// Re-export the three member crates under shorter names so downstream
// code doesn't need to add them to Cargo.toml individually unless it
// wants to pin different versions.
pub use apimock_config as config;
pub use apimock_routing as routing;
pub use apimock_server as server;

pub use app::App;
pub use args::EnvArgs;

/// Build the app (default feature).
///
/// Returns a `Result` whose error side is `apimock_server::ServerError`,
/// which itself `#[from]`-wraps `ConfigError` and (via that)
/// `RoutingError`. At the binary's process boundary we flatten all of
/// these into `anyhow::Error`.
#[cfg(not(feature = "spawn"))]
pub async fn new(env_args: &EnvArgs) -> apimock_server::ServerResult<App> {
    App::new(env_args, None, true).await
}

#[cfg(feature = "spawn")]
use tokio::sync::mpsc::Sender;

/// Build the app under the `spawn` feature.
///
/// - `spawn_tx`: channel to forward log output to the embedding process.
/// - `includes_ansi_codes`: if `true`, forwarded log lines retain ANSI
///   colour escapes.
#[cfg(feature = "spawn")]
pub async fn new(
    env_args: &EnvArgs,
    spawn_tx: Sender<String>,
    includes_ansi_codes: bool,
) -> apimock_server::ServerResult<App> {
    App::new(env_args, Some(spawn_tx), includes_ansi_codes).await
}
