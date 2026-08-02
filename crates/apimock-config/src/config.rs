//! The `Config` struct: orchestrates loading, validation, and relative-path
//! resolution for `apimock.toml` and every file it references.
//!
//! # What moved in 5.0
//!
//! Pre-5.0 `Config::new` also compiled every Rhai middleware. That step
//! produced `MiddlewareHandler` values — which live in `apimock-server`
//! after the split — so we deliberately stop there now. Middleware
//! compilation happens in the server crate, *driven by* the paths
//! listed in this struct's `service.middlewares_file_paths`. The
//! separation keeps config dependency-free of Rhai and hyper, and
//! prevents the GUI-oriented snapshot API (coming in stage 2) from
//! reaching into execution state it has no business seeing.

use apimock_routing::RuleSet;
use constant::*;
use listener_config::ListenerConfig;
use log_config::LogConfig;
use serde::Deserialize;
use service_config::ServiceConfig;

use std::{fs, path::Path};

use crate::{
    error::{ConfigError, ConfigResult},
    path_util::current_dir_to_file_parent_dir_relative_path,
};

pub mod constant;
pub mod file_tree_config;
pub mod listener_config;
pub mod log_config;
pub mod service_config;

/// Top-level application configuration, corresponding one-to-one with
/// `apimock.toml`.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// Where this config was loaded from. Kept so we can resolve relative
    /// paths (rule sets, middlewares, respond dirs) against the config
    /// file's parent directory — not the process's working directory.
    #[serde(skip)]
    pub file_path: Option<String>,

    pub listener: Option<ListenerConfig>,
    pub log: Option<LogConfig>,
    pub service: ServiceConfig,
    /// Optional filter configuration for `FileTreeView`. When absent,
    /// [`FileTreeViewConfig::default()`] applies (dotfiles hidden,
    /// built-in excludes on).
    pub file_tree_view: Option<file_tree_config::FileTreeViewConfig>,
}

impl Config {
    /// Build a `Config` by reading the TOML file, resolving rule-set
    /// paths, and validating the result.
    ///
    /// Middleware paths are *recorded* on the returned Config but not
    /// compiled here — the server crate performs compilation. See the
    /// module docstring for why.
    // clippy: ConfigError is a public error type (RFC 030 §6 escalation
    // trigger); boxing its large variant would change that type's shape.
    // See ESCALATION-002 in the RFC 030 review-request package.
    #[allow(clippy::result_large_err)]
    pub fn new(
        config_file_path: Option<&String>,
        fallback_respond_dir_path: Option<&String>,
    ) -> ConfigResult<Self> {
        let mut ret = Self::init(config_file_path)?;

        ret.set_rule_sets()?;

        ret.compute_fallback_respond_dir(fallback_respond_dir_path)?;

        if !ret.validate() {
            return Err(ConfigError::Validation);
        }

        log::info!("{}", ret);

        Ok(ret)
    }

    /// Load + parse the TOML file. Returns `Config::default()` when no
    /// path is provided (this is the zero-config "just serve a folder"
    /// path).
    // clippy: ConfigError is a public error type (RFC 030 §6 escalation
    // trigger); boxing its large variant would change that type's shape.
    // See ESCALATION-002 in the RFC 030 review-request package.
    #[allow(clippy::result_large_err)]
    fn init(config_file_path: Option<&String>) -> ConfigResult<Self> {
        let Some(config_file_path) = config_file_path else {
            return Ok(Config::default());
        };

        log::info!("[config] {}\n", config_file_path);

        let path = Path::new(config_file_path);
        let toml_string =
            fs::read_to_string(config_file_path).map_err(|e| ConfigError::ConfigRead {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut config: Config =
            toml::from_str(&toml_string).map_err(|e| ConfigError::ConfigParse {
                path: path.to_path_buf(),
                canonical: path.canonicalize().ok(),
                source: e,
            })?;
        config.file_path = Some(config_file_path.to_owned());

        Ok(config)
    }

    /// Load every rule-set file listed in `service.rule_sets`.
    // clippy: ConfigError is a public error type (RFC 030 §6 escalation
    // trigger); boxing its large variant would change that type's shape.
    // See ESCALATION-002 in the RFC 030 review-request package.
    #[allow(clippy::result_large_err)]
    fn set_rule_sets(&mut self) -> ConfigResult<()> {
        let relative_dir_path = self.current_dir_to_parent_dir_relative_path()?;

        let Some(rule_sets_file_paths) = self.service.rule_sets_file_paths.as_ref() else {
            return Ok(());
        };

        let mut rule_sets = Vec::with_capacity(rule_sets_file_paths.len());
        for (rule_set_idx, rule_set_file_path) in rule_sets_file_paths.iter().enumerate() {
            let joined = Path::new(relative_dir_path.as_str()).join(rule_set_file_path);
            let path_str = joined.to_str().ok_or_else(|| ConfigError::ConfigRead {
                path: joined.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "rule set #{} path contains non-UTF-8 bytes: {}",
                        rule_set_idx + 1,
                        joined.to_string_lossy(),
                    ),
                ),
            })?;

            // `RuleSet::new` returns `RoutingError`; `ConfigError::RuleSet`
            // converts via `#[from]`.
            rule_sets.push(RuleSet::new(
                path_str,
                relative_dir_path.as_str(),
                rule_set_idx,
            )?);
        }

        self.service.rule_sets = rule_sets;
        Ok(())
    }

    /// Resolve the fallback respond dir against the config file's parent
    /// directory. See the module doc for why we don't resolve against CWD.
    // clippy: ConfigError is a public error type (RFC 030 §6 escalation
    // trigger); boxing its large variant would change that type's shape.
    // See ESCALATION-002 in the RFC 030 review-request package.
    #[allow(clippy::result_large_err)]
    pub fn compute_fallback_respond_dir(
        &mut self,
        fallback_respond_dir_path: Option<&String>,
    ) -> ConfigResult<()> {
        if let Some(fallback_respond_dir_path) = fallback_respond_dir_path {
            self.service.fallback_respond_dir = fallback_respond_dir_path.to_owned();
            return Ok(());
        }

        if self.service.fallback_respond_dir.as_str() == SERVICE_DEFAULT_FALLBACK_RESPOND_DIR {
            return Ok(());
        }

        let relative_path = self.current_dir_to_parent_dir_relative_path()?;
        let joined =
            Path::new(relative_path.as_str()).join(self.service.fallback_respond_dir.as_str());
        let resolved = joined.to_str().ok_or_else(|| ConfigError::PathResolve {
            path: joined.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "fallback_respond_dir path contains non-UTF-8 bytes: {}",
                    joined.to_string_lossy(),
                ),
            ),
        })?;
        self.service.fallback_respond_dir = resolved.to_owned();
        Ok(())
    }

    /// HTTP listener address, if HTTP is enabled.
    pub fn listener_http_addr(&self) -> Option<String> {
        let https_is_active = self.listener_https_addr().is_some();
        if https_is_active {
            let port_is_single = self
                .listener
                .as_ref()
                .and_then(|l| l.tls.as_ref())
                .map(|t| t.port.is_none())
                .unwrap_or(false);
            if port_is_single {
                return None;
            }
        }

        let listener_default;
        let listener = match self.listener.as_ref() {
            Some(l) => l,
            None => {
                listener_default = ListenerConfig::default();
                &listener_default
            }
        };

        Some(format!("{}:{}", listener.ip_address, listener.port))
    }

    /// HTTPS listener address, if TLS is configured.
    pub fn listener_https_addr(&self) -> Option<String> {
        let listener = self.listener.as_ref()?;
        let tls = listener.tls.as_ref()?;
        let port = tls.port.unwrap_or(listener.port);
        Some(format!("{}:{}", listener.ip_address, port))
    }

    /// Validate settings. Returns `false` when any subcomponent's
    /// validator returned false — they log details at their own call
    /// site so the user sees every problem in one pass.
    fn validate(&self) -> bool {
        if let Some(listener) = self.listener.as_ref() {
            if !listener.validate() {
                return false;
            }

            if self.listener_http_addr().is_none() && self.listener_https_addr().is_none() {
                log::error!("at least one listener (http or https) is required");
                return false;
            }
        }
        self.service.validate()
    }

    /// Relative path from CWD to the parent dir of the config file.
    // clippy: ConfigError is a public error type (RFC 030 §6 escalation
    // trigger); boxing its large variant would change that type's shape.
    // See ESCALATION-002 in the RFC 030 review-request package.
    #[allow(clippy::result_large_err)]
    pub fn current_dir_to_parent_dir_relative_path(&self) -> ConfigResult<String> {
        let Some(file_path) = self.file_path.as_ref() else {
            return Ok(String::from("."));
        };

        let relative_dir_path = current_dir_to_file_parent_dir_relative_path(file_path.as_str())
            .map_err(|e| ConfigError::PathResolve {
                path: Path::new(file_path).to_path_buf(),
                source: e,
            })?;

        let as_str = relative_dir_path
            .to_str()
            .ok_or_else(|| ConfigError::PathResolve {
                path: relative_dir_path.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "relative path contains non-UTF-8 bytes: {}",
                        relative_dir_path.to_string_lossy()
                    ),
                ),
            })?;

        Ok(as_str.to_owned())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            file_path: None,
            listener: Some(ListenerConfig {
                ip_address: LISTENER_DEFAULT_IP_ADDRESS.to_owned(),
                port: LISTENER_DEFAULT_PORT,
                tls: None,
            }),
            log: Some(LogConfig::default()),
            service: ServiceConfig::default(),
            file_tree_view: None,
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
