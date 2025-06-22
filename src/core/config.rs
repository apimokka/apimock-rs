use constant::*;
use listener_config::ListenerConfig;
use log_config::LogConfig;
use serde::Deserialize;
use service_config::ServiceConfig;
use toml;

use std::{fs, path::Path};

use crate::core::server::middleware::middleware_handler::MiddlewareHandler;

use super::{
    server::routing::rule_set::RuleSet, util::path::current_dir_to_file_parent_dir_relative_path,
};

pub mod constant;
pub mod listener_config;
pub mod log_config;
pub mod service_config;

/// app config
#[derive(Clone, Deserialize)]
pub struct Config {
    #[serde(skip)]
    file_path: Option<String>,

    pub listener: Option<ListenerConfig>,
    pub log: Option<LogConfig>,
    pub service: ServiceConfig,
}

/// app config
impl Config {
    /// create new instance
    pub fn new(config_file_path: Option<&String>) -> Self {
        let mut ret = Self::init(config_file_path);

        ret.set_rule_sets();
        match ret.middlewares_from_file_paths() {
            Ok(x) => {
                if !x.is_empty() {
                    log::info!("middleware is activated: {} file(s)", x.len());
                }
                ret.service.middlewares = x
            }
            Err(x) => panic!("{}", x),
        }

        ret.compute_fallback_respond_dir();

        if !ret.validate() {
            panic!("failed to start up due to invalid config");
        }

        log::info!("{}", ret);

        ret
    }

    /// initialize
    fn init(config_file_path: Option<&String>) -> Self {
        let ret = if let Some(config_file_path) = config_file_path {
            log::info!("[config] {}\n", config_file_path);

            let toml_string = fs::read_to_string(config_file_path.as_str()).unwrap();
            let mut config: Config = match toml::from_str(&toml_string) {
                Ok(x) => x,
                Err(err) => panic!(
                    "invalid toml content: {} ({})\n({})",
                    config_file_path,
                    Path::new(config_file_path)
                        .canonicalize()
                        .unwrap_or_default()
                        .to_string_lossy(),
                    err
                ),
            };

            config.file_path = Some(config_file_path.to_owned());

            config
        } else {
            Config::default()
        };

        ret
    }

    /// set rule sets from rule sets file paths
    fn set_rule_sets(&mut self) {
        let relative_dir_path = self.current_dir_to_parent_dir_relative_path();

        let rule_sets_file_paths = match self.service.rule_sets_file_paths.as_ref() {
            Some(x) => x,
            None => return,
        };

        self.service.rule_sets = rule_sets_file_paths
            .iter()
            .enumerate()
            .map(|(rule_set_idx, rule_set_file_path)| {
                let rule_set_file_path =
                    Path::new(relative_dir_path.as_str()).join(rule_set_file_path);
                let rule_set_file_path = rule_set_file_path.to_str().expect(
                    format!(
                        "failed to get relative path from current dir to rule set #{} ({})",
                        rule_set_idx + 1,
                        rule_set_file_path.to_string_lossy()
                    )
                    .as_str(),
                );

                RuleSet::new(rule_set_file_path, relative_dir_path.as_str(), rule_set_idx)
            })
            .collect();
    }

    /// set middlewares from middlewares file paths
    fn middlewares_from_file_paths(&mut self) -> Result<Vec<MiddlewareHandler>, String> {
        let relative_dir_path = self.current_dir_to_parent_dir_relative_path();

        match self.service.middlewares_file_paths.as_ref() {
            Some(x) => x
                .iter()
                .enumerate()
                .map(|(middleware_idx, middlware_file_path)| {
                    let middlware_file_path =
                        Path::new(relative_dir_path.as_str()).join(middlware_file_path);
                    let middlware_file_path = middlware_file_path.to_str().expect(
                        format!(
                            "failed to get relative path from current dir to rule set #{} ({})",
                            middleware_idx + 1,
                            middlware_file_path.to_string_lossy()
                        )
                        .as_str(),
                    );

                    MiddlewareHandler::new(middlware_file_path)
                })
                .collect(),
            None => Ok(vec![]),
        }
    }

    /// compute relative fallback_respond_dir from current dir
    pub fn compute_fallback_respond_dir(&mut self) {
        if self.service.fallback_respond_dir.as_str() == SERVICE_DEFAULT_FALLBACK_RESPOND_DIR {
            return;
        }

        let relative_path = self.current_dir_to_parent_dir_relative_path();
        let fallback_respond_dir =
            Path::new(relative_path.as_str()).join(self.service.fallback_respond_dir.as_str());
        let fallback_respond_dir = fallback_respond_dir.to_str().expect(
            format!(
                "failed to get path str: {}",
                fallback_respond_dir.to_string_lossy()
            )
            .as_str(),
        );
        self.service.fallback_respond_dir = fallback_respond_dir.to_owned();
    }

    /// address listened to
    pub fn listener_address(&self) -> String {
        let listener = if let Some(listener) = self.listener.as_ref() {
            listener
        } else {
            &ListenerConfig::default()
        };
        format!("{}:{}", listener.ip_address, listener.port)
    }

    /// update `fallback_respond_dir`
    // pub fn update_fallback_respond_dir(&mut self, data_dir: &str, old_data_dir: &str) {}

    /// validate settings in app config
    ///
    /// note: as to ListenerConfig validation, tcp listener is expected to run afterward instead
    /// note: none requires validation in LogConfig
    fn validate(&self) -> bool {
        self.service.validate()
    }

    /// get relative path from current dir (working dir) to parent dir of this file
    fn current_dir_to_parent_dir_relative_path(&self) -> String {
        match self.file_path.as_ref() {
            Some(x) => {
                let relative_dir_path =
                    current_dir_to_file_parent_dir_relative_path(x.as_str())
                    .expect(&format!("failed to get relative path from current dir to config toml file dir: config toml = {}", self.file_path.clone().unwrap_or_default()));
                let relative_dir_path = relative_dir_path.to_str().expect(
                    format!(
                        "failed to get relative file str: {}",
                        relative_dir_path.to_string_lossy()
                    )
                    .as_str(),
                );
                relative_dir_path.to_owned()
            }
            None => String::from("."),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            file_path: None,
            listener: Some(ListenerConfig {
                ip_address: LISTENER_DEFAULT_IP_ADDRESS.to_owned(),
                port: LISTENER_DEFAULT_PORT,
            }),
            log: Some(LogConfig::default()),
            service: ServiceConfig::default(),
        }
    }
}

impl std::fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let log = self.log.clone().unwrap_or_default();
        let _ = write!(f, "{}", log);
        let _ = writeln!(f, "{}", PRINT_DELIMITER);
        let _ = write!(f, "{}", self.service);
        Ok(())
    }
}
