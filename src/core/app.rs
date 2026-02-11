use tokio::sync::mpsc::Sender;

pub mod app_state;
pub mod constant;

use super::args::EnvArgs;
use super::config::listener_config::ListenerConfig;
use super::config::Config;
use super::logger::init_logger;
use super::server::Server;
use app_state::AppState;

/// app
pub struct App {
    pub server: Server,
}

impl App {
    /// create new app
    ///
    /// - listener_port_to_overwrite: ignores port in config toml. used in both arguments and tests
    pub async fn new(
        env_args: &EnvArgs,
        spawn_tx: Option<Sender<String>>,
        includes_ansi_codes: bool,
    ) -> Self {
        let _ = init_logger(spawn_tx, includes_ansi_codes);

        let mut config = Config::new(
            env_args.config_file_path.as_ref(),
            env_args.fallback_respond_dir_path.as_ref(),
        );

        // overwrite port if the arg is specified
        if let Some(port) = env_args.port {
            let mut listener = if let Some(listener) = config.listener {
                listener
            } else {
                ListenerConfig::default()
            };
            listener.port = port;
            config.listener = Some(listener);
        }

        let app_state = AppState { config };

        let server = Server::new(app_state).await;

        Self { server }
    }
}
