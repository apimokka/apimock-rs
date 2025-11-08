//! A developer-friendly, featherlight and functional HTTP mock server
//! built in Rust, based on [hyper](https://hyper.rs/) and [tokio](https://tokio.rs/).

pub mod core;
use core::app::App;
use core::args::EnvArgs;

/// return hyper http server
#[cfg(not(feature = "spawn"))]
pub async fn new(env_args: &EnvArgs) -> App {
    App::new(env_args, None, true).await
}

#[cfg(feature = "spawn")]
use tokio::sync::mpsc::Sender;

/// accept sender to main proc set to logger and
/// return hyper http server
/// `includes_ansi_codes`: if true, log includes ansi escape codes for console text color
#[cfg(feature = "spawn")]
pub async fn new(env_args: &EnvArgs, spawn_tx: Sender<String>, includes_ansi_codes: bool) -> App {
    App::new(env_args, Some(spawn_tx), includes_ansi_codes).await
}
