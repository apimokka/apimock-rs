use constant::*;
use listener_config::ListenerConfig;
use log_config::LogConfig;
use serde::Deserialize;
use service_config::ServiceConfig;
use toml;

use std::{fs, path::Path};

use crate::core::{
    error::{AppError, AppResult},
    server::middleware::middleware_handler::MiddlewareHandler,
};

use super::{
    server::routing::rule_set::RuleSet, util::path::current_dir_to_file_parent_dir_relative_path,
};

pub mod constant;
pub mod listener_config;
pub mod log_config;
pub mod service_config;

/// Top-level application configuration, corresponding one-to-one with
/// `apimock.toml`.
///
/// # Why config lives in one struct, not one-per-section
///
/// TOML's section structure matches a single nested struct very naturally
/// via `serde::Deserialize`. Splitting into multiple top-level structs
/// would force us to parse the file more than once or to invent custom
/// deserialization, for no real benefit — sections are already grouped
/// inside this struct (`listener`, `log`, `service`).
#[derive(Clone, Deserialize)]
pub struct Config {
    /// Where this config was loaded from. Kept so we can resolve relative
    /// paths (rule sets, middlewares, respond dirs) against the config
    /// file's parent directory — not the process's working directory.
    #[serde(skip)]
    file_path: Option<String>,

    pub listener: Option<ListenerConfig>,
    pub log: Option<LogConfig>,
    pub service: ServiceConfig,
}

impl Config {
    /// Build a new `Config` by reading the given TOML file, resolving
    /// rule-set and middleware paths, and validating the result.
    ///
    /// # Why a single entry point does all of loading, resolving and validating
    ///
    /// In 4.6.x these stages were scattered across `Config::new`, and each
    /// failure mode either panicked or silently defaulted. Consolidating
    /// them here means each failure becomes one typed `AppError` variant,
    /// and the caller (`App::new`) can return a single `AppResult<Self>`.
    pub fn new(
        config_file_path: Option<&String>,
        fallback_respond_dir_path: Option<&String>,
    ) -> AppResult<Self> {
        let mut ret = Self::init(config_file_path)?;

        ret.set_rule_sets()?;

        let middlewares = ret.middlewares_from_file_paths()?;
        if !middlewares.is_empty() {
            log::info!("middleware is activated: {} file(s)", middlewares.len());
        }
        ret.service.middlewares = middlewares;

        ret.compute_fallback_respond_dir(fallback_respond_dir_path)?;

        if !ret.validate() {
            return Err(AppError::Validation);
        }

        log::info!("{}", ret);

        Ok(ret)
    }

    /// Load + parse the TOML file. Returns `Config::default()` when no
    /// path is provided (this is the zero-config "just serve a folder"
    /// path).
    fn init(config_file_path: Option<&String>) -> AppResult<Self> {
        let Some(config_file_path) = config_file_path else {
            return Ok(Config::default());
        };

        log::info!("[config] {}\n", config_file_path);

        let path = Path::new(config_file_path);
        let toml_string =
            fs::read_to_string(config_file_path).map_err(|e| AppError::ConfigRead {
                path: path.to_path_buf(),
                source: e,
            })?;

        let mut config: Config = toml::from_str(&toml_string).map_err(|e| AppError::ConfigParse {
            path: path.to_path_buf(),
            canonical: path.canonicalize().ok(),
            source: e,
        })?;
        config.file_path = Some(config_file_path.to_owned());

        Ok(config)
    }

    /// Load every rule-set file listed in `service.rule_sets`.
    fn set_rule_sets(&mut self) -> AppResult<()> {
        let relative_dir_path = self.current_dir_to_parent_dir_relative_path()?;

        let Some(rule_sets_file_paths) = self.service.rule_sets_file_paths.as_ref() else {
            return Ok(());
        };

        let mut rule_sets = Vec::with_capacity(rule_sets_file_paths.len());
        for (rule_set_idx, rule_set_file_path) in rule_sets_file_paths.iter().enumerate() {
            let joined = Path::new(relative_dir_path.as_str()).join(rule_set_file_path);
            // Non-UTF-8 paths are rare but possible on Unix. We surface
            // them as a `ConfigRead` error rather than panicking so the
            // user gets a message that points at the offending entry.
            let path_str = joined.to_str().ok_or_else(|| AppError::ConfigRead {
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

            rule_sets.push(RuleSet::new(
                path_str,
                relative_dir_path.as_str(),
                rule_set_idx,
            )?);
        }

        self.service.rule_sets = rule_sets;
        Ok(())
    }

    /// Compile every middleware file listed in `service.middlewares`.
    fn middlewares_from_file_paths(&self) -> AppResult<Vec<MiddlewareHandler>> {
        let relative_dir_path = self.current_dir_to_parent_dir_relative_path()?;

        let Some(middleware_file_paths) = self.service.middlewares_file_paths.as_ref() else {
            return Ok(Vec::new());
        };

        let mut handlers = Vec::with_capacity(middleware_file_paths.len());
        for (middleware_idx, middleware_file_path) in middleware_file_paths.iter().enumerate() {
            let joined = Path::new(relative_dir_path.as_str()).join(middleware_file_path);
            let path_str = joined.to_str().ok_or_else(|| AppError::ConfigRead {
                path: joined.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "middleware #{} path contains non-UTF-8 bytes: {}",
                        middleware_idx + 1,
                        joined.to_string_lossy(),
                    ),
                ),
            })?;
            handlers.push(MiddlewareHandler::new(path_str)?);
        }

        Ok(handlers)
    }

    /// Compute the resolved `fallback_respond_dir` used by the file-based
    /// (a.k.a. "dyn route") fallback responder.
    ///
    /// # Why this needs special handling
    ///
    /// The user supplies either a CLI flag (`-d`) or a value in the config
    /// file. A CLI flag wins unconditionally. Otherwise, the config value
    /// must be resolved relative to the config file (not CWD) to match
    /// user expectations — running `apimock -c path/to/conf.toml` from a
    /// different directory must still find `./responses/` as defined
    /// inside `conf.toml`.
    pub fn compute_fallback_respond_dir(
        &mut self,
        fallback_respond_dir_path: Option<&String>,
    ) -> AppResult<()> {
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
        let resolved = joined.to_str().ok_or_else(|| AppError::PathResolve {
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
    ///
    /// # Why HTTP can be disabled even when `[listener]` is present
    ///
    /// When TLS is configured without a dedicated `tls.port`, we treat the
    /// single listener port as HTTPS-only. This is the "just serve HTTPS
    /// on 3001" shape — the most common TLS configuration, and avoiding
    /// accidentally binding a duplicate plaintext listener on the same
    /// port. Returning `None` is how we signal "skip the HTTP listener".
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

    /// Validate settings.
    ///
    /// # Why validation is a bool, not an `AppResult`
    ///
    /// Individual validators log detailed error messages at the call site
    /// (so the user can see *every* problem in one pass, not just the
    /// first). This function only reports whether the config as a whole
    /// is acceptable — the `AppError::Validation` wrapping is added once
    /// by `Config::new`.
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
    ///
    /// Used to resolve every other path in the config (rule sets,
    /// middlewares, respond dirs) relative to the config file's location,
    /// not the process's working directory.
    fn current_dir_to_parent_dir_relative_path(&self) -> AppResult<String> {
        let Some(file_path) = self.file_path.as_ref() else {
            return Ok(String::from("."));
        };

        let relative_dir_path =
            current_dir_to_file_parent_dir_relative_path(file_path.as_str()).map_err(|e| {
                AppError::PathResolve {
                    path: Path::new(file_path).to_path_buf(),
                    source: e,
                }
            })?;

        let as_str = relative_dir_path
            .to_str()
            .ok_or_else(|| AppError::PathResolve {
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
