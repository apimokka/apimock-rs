//! Top-level wiring that binds config + server together.
//!
//! # Why this layer exists on top of [`Server`]
//!
//! `Server` is concerned with listening and dispatching requests.
//! `App` is concerned with everything that has to happen *before*
//! we start listening: installing the global logger exactly once,
//! reading and validating config, and applying CLI overrides.
//! Keeping these concerns apart means `Server` can be tested in
//! isolation without reaching into the filesystem or installing a
//! global logger.

use apimock_config::Config;
use apimock_server::{Server, ServerResult};
use tokio::sync::mpsc::Sender;

use crate::args::EnvArgs;
use crate::logger::init_logger;

/// The application — config loaded, logger installed, server ready to start.
pub struct App {
    pub server: Server,
}

impl App {
    /// Construct the app.
    ///
    /// - `env_args`: command-line arguments (already parsed by
    ///   `EnvArgs::default`).
    /// - `spawn_tx`: only under the `spawn` feature — a channel to forward
    ///   log output to the embedding process.
    /// - `includes_ansi_codes`: whether forwarded logs should retain ANSI
    ///   colour escapes. Ignored without the `spawn` feature.
    pub async fn new(
        env_args: &EnvArgs,
        spawn_tx: Option<Sender<String>>,
        includes_ansi_codes: bool,
    ) -> ServerResult<Self> {
        // `init_logger` is only fallible if something else in the process
        // already installed a global logger. In a normal binary run that
        // can't happen; in tests / embedding scenarios a prior init is a
        // no-op we tolerate deliberately.
        let _ = init_logger(spawn_tx, includes_ansi_codes);

        let mut config = Config::new(
            env_args.config_file_path.as_ref(),
            env_args.fallback_respond_dir_path.as_ref(),
        )?;

        // Overwrite port if `--port` was supplied. We apply the override
        // *after* loading config (not before) so that validation has run
        // against the as-written file first.
        if let Some(port) = env_args.port {
            let mut listener = config.listener.unwrap_or_default();
            listener.port = port;
            config.listener = Some(listener);
        }

        let server = Server::new(config).await?;

        Ok(Self { server })
    }
}
