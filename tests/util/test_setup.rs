use rand::RngExt;

use std::{
    env,
    net::{SocketAddr, TcpListener},
    path::Path,
};

use apimock::core::{app::App, args::EnvArgs};

use super::{
    constant::{CONFIG_FILE_NAME, CONFIG_TESTS_ROOT_DIR_PATH},
    tls::{generate_tls_credentials, tls_credentials_are_ready},
};

#[derive(Clone)]
pub struct TestSetup {
    pub root_config_file_path: Option<String>,
    pub port: Option<u16>,
    pub fallback_respond_dir_path: Option<String>,
    /// bound to set_current_dir(). **caution:** affects globally
    pub current_dir_path: Option<String>,
}

impl TestSetup {
    /// generate setup args with specific dir where root config file is located
    pub fn default_with_root_config_dir(root_config_dir_path: &str) -> Self {
        let mut ret = Self::default();

        ret.root_config_file_path = Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(CONFIG_TESTS_ROOT_DIR_PATH)
                .join(root_config_dir_path)
                .join(CONFIG_FILE_NAME)
                .to_str()
                .expect("failed to generate root config file path")
                .to_string(),
        );

        ret
    }

    /// test initial setup with dynamic port selected
    pub async fn launch(&self) -> u16 {
        let port = if let Some(port) = self.port {
            port
        } else {
            dynamic_port()
        };

        if !tls_credentials_are_ready() {
            generate_tls_credentials();
        }

        let _ = self.launch_impl(port).await;
        port
    }

    /// test initial setup: start up mock server
    async fn launch_impl(&self, port: u16) {
        if let Some(current_dir_path) = self.current_dir_path.as_ref() {
            let current_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(CONFIG_TESTS_ROOT_DIR_PATH)
                .join(current_dir_path.as_str());

            let _ = match env::set_current_dir(current_dir.clone()) {
                Ok(_) => (),
                Err(err) => {
                    panic!(
                        "failed to set current dir: {} ({})",
                        current_dir.to_string_lossy(),
                        err
                    );
                }
            };
        }

        let mut app_env_args = env_args(port);

        if let Some(root_config_file_path) = self.root_config_file_path.as_ref() {
            app_env_args.config_file_path = Some(root_config_file_path.to_owned());
        }

        if let Some(fallback_respond_dir_path) = self.fallback_respond_dir_path.as_ref() {
            app_env_args.fallback_respond_dir_path = Some(fallback_respond_dir_path.to_owned());
        }

        tokio::spawn(async move {
            let app = App::new(&app_env_args, None, true).await;
            app.server.start().await
        });

        // wait for server started
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }
}

impl Default for TestSetup {
    fn default() -> Self {
        Self {
            root_config_file_path: Some(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(CONFIG_TESTS_ROOT_DIR_PATH)
                    .join(CONFIG_FILE_NAME)
                    .to_str()
                    .expect("failed to generate root config file path")
                    .to_string(),
            ),
            port: None,
            fallback_respond_dir_path: None,
            current_dir_path: None,
        }
    }
}

/// select dynamic port randomly
fn dynamic_port() -> u16 {
    let port = rand::rng().random_range(49152..=65535);

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    match TcpListener::bind(addr) {
        Ok(_) => port,
        Err(_) => dynamic_port(),
    }
}

/// env args for testing
fn env_args(port: u16) -> EnvArgs {
    let mut ret = EnvArgs::default().expect("failed to get env args");

    ret.port = Some(port);

    match ret.validate() {
        Ok(_) => ret,
        Err(_) => panic!("something wrong in env args"),
    }
}
