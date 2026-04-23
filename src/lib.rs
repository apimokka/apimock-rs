//! A developer-friendly, featherlight and functional HTTP(S) mock server
//! built in Rust, based on [hyper](https://hyper.rs/) and [tokio](https://tokio.rs/).

pub mod core;
use core::app::App;
use core::args::EnvArgs;
use core::error::AppResult;

/// Build the app (default feature).
///
/// Returns an `AppResult` so that configuration or startup errors become
/// typed, formatted messages rather than panics. See `core::error` for
/// rationale.
#[cfg(not(feature = "spawn"))]
pub async fn new(env_args: &EnvArgs) -> AppResult<App> {
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
) -> AppResult<App> {
    App::new(env_args, Some(spawn_tx), includes_ansi_codes).await
}
