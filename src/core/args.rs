use std::{env, fs, io, path::Path};

pub mod constant;

use constant::*;

use super::error::{AppError, AppResult};

/// CLI arguments parsed at process start-up.
///
/// # Why these three fields are the only command-line surface
///
/// `apimock` deliberately keeps its CLI tiny: config-file path, port,
/// fallback respond dir. Anything richer than that belongs in the TOML
/// config so that it can be checked in with the rest of the mock setup
/// and reproduced between machines. The three CLI flags exist only for
/// quick ad-hoc overrides that don't warrant editing the config file.
#[derive(Clone)]
pub struct EnvArgs {
    /// path to the config TOML file (usually `./apimock.toml`)
    pub config_file_path: Option<String>,
    /// overrides `listener.port` in the config file
    pub port: Option<u16>,
    /// overrides `service.fallback_respond_dir` in the config file
    pub fallback_respond_dir_path: Option<String>,
}

impl EnvArgs {
    /// Parse `env::args()` and apply defaults.
    ///
    /// Returns:
    /// - `Ok(Some(args))` for the normal "start the server" path,
    /// - `Ok(None)` when a meta command (e.g. `--init`) has already
    ///   completed its side effect and the process should exit cleanly,
    /// - `Err(_)` when an argument was malformed or a referenced file
    ///   is missing.
    ///
    /// # Why return `AppResult<Option<_>>` instead of panicking
    ///
    /// Previously invalid arguments triggered `panic!`, which printed a
    /// backtrace for a user-level error. Returning a typed error lets the
    /// binary print "invalid port: foo" and exit 1, which is what users
    /// of CLI tools actually expect.
    pub fn default() -> AppResult<Option<Self>> {
        let mut ret = EnvArgs::from_args()?;

        let init_config = args_option_value(INIT_CONFIG_OPTION_NAMES.as_ref()).is_some();
        if init_config {
            let includes_middleware =
                args_option_value(INCLUDES_MIDDLEWARE_OPTION_NAMES.as_ref()).is_some();
            // generate config files and quit
            if let Err(err) = ret.init_config(includes_middleware) {
                log::error!("failed to init config ({})", err);
            }
            return Ok(None);
        }

        ret.default_config_file_path();
        ret.validate()?;

        Ok(Some(ret))
    }

    /// Ensure paths referenced by CLI flags actually exist.
    ///
    /// We only check existence, not permission or content — a file the
    /// process can see but can't read will still produce a better error
    /// downstream at the point it's actually used.
    pub fn validate(&self) -> AppResult<()> {
        if let Some(config_file_path) = self.config_file_path.as_ref() {
            if !Path::new(config_file_path.as_str()).exists() {
                return Err(AppError::ConfigRead {
                    path: Path::new(config_file_path).to_path_buf(),
                    source: io::Error::new(
                        io::ErrorKind::NotFound,
                        "config file specified via --config does not exist",
                    ),
                });
            }
        }

        if let Some(fallback_respond_dir_path) = self.fallback_respond_dir_path.as_ref() {
            if !Path::new(fallback_respond_dir_path.as_str()).exists() {
                return Err(AppError::PathResolve {
                    path: Path::new(fallback_respond_dir_path).to_path_buf(),
                    source: io::Error::new(
                        io::ErrorKind::NotFound,
                        "fallback response dir specified via --dir does not exist",
                    ),
                });
            }
        }

        Ok(())
    }

    /// Build an `EnvArgs` by reading `env::args()`.
    fn from_args() -> AppResult<Self> {
        let port = match args_option_value(CONFIG_LISTENER_PORT_OPTION_NAMES.as_ref()) {
            Some(port_str) => Some(port_str.parse::<u16>().map_err(|e| {
                AppError::ListenerAddress {
                    addr: port_str.clone(),
                    reason: format!("--port value is not a valid u16: {}", e),
                }
            })?),
            None => None,
        };

        Ok(EnvArgs {
            config_file_path: args_option_value(CONFIG_FILE_PATH_OPTION_NAMES.as_ref()),
            port,
            fallback_respond_dir_path: args_option_value(
                FALLBACK_RESPOND_DIR_PATH_OPTION_NAMES.as_ref(),
            ),
        })
    }

    /// Scaffold `apimock.toml` (and related files) into the current directory.
    ///
    /// Skips files that already exist — `--init` is a convenience, not an
    /// overwrite tool. The caller logs results via `println!` so a user
    /// running `apimock --init` for the first time sees what was created.
    fn init_config(&mut self, includes_middleware: bool) -> Result<(), io::Error> {
        if includes_middleware {
            if !Path::new(DEFAULT_MIDDLEWARE_FILE_PATH).exists() {
                let content = include_str!("../../examples/config/default/apimock-middleware.rhai");
                fs::write(DEFAULT_MIDDLEWARE_FILE_PATH, content)?;
                println!(
                    "middleware scripting file is created: {}.",
                    DEFAULT_MIDDLEWARE_FILE_PATH
                );
            } else {
                println!(
                    "[warn] middlware scripting file exists: {}.",
                    DEFAULT_MIDDLEWARE_FILE_PATH
                );
            }
        }

        if Path::new(DEFAULT_CONFIG_FILE_PATH).exists() {
            println!(
                "[warn] quit because default root config file exists: {}.",
                DEFAULT_CONFIG_FILE_PATH
            );
            return Ok(());
        }

        let config_content = include_str!("../../examples/config/default/apimock.toml");
        fs::write(DEFAULT_CONFIG_FILE_PATH, config_content)?;
        println!("root config file is created: {}.", DEFAULT_CONFIG_FILE_PATH);

        if !Path::new(DEFAULT_RULE_SET_FILE_PATH).exists() {
            let rule_set_content =
                include_str!("../../examples/config/default/apimock-rule-set.toml");
            fs::write(DEFAULT_RULE_SET_FILE_PATH, rule_set_content)?;
            println!(
                "rule set config file is created: {}.",
                DEFAULT_RULE_SET_FILE_PATH
            );
        }

        Ok(())
    }

    /// If no config file was specified on the command line and one exists
    /// at `./apimock.toml`, use that.
    ///
    /// This is what powers the "run `apimock` in your project directory
    /// and it just picks up the config" behaviour.
    fn default_config_file_path(&mut self) {
        if self.config_file_path.is_some() {
            return;
        }
        if !Path::new(DEFAULT_CONFIG_FILE_PATH).exists() {
            return;
        }
        self.config_file_path = Some(DEFAULT_CONFIG_FILE_PATH.to_owned());
    }
}

/// Look up the value associated with any of the given option names in
/// `env::args()`.
///
/// For flags that don't take a value (e.g. `--init`), returns `Some("")`
/// so the caller can check `.is_some()` without caring about the payload.
fn args_option_value(option_names: &[&str]) -> Option<String> {
    let args: Vec<String> = env::args().collect();

    let name_index = args
        .iter()
        .position(|arg| option_names.iter().any(|n| arg.as_str() == *n))?;

    let name_value = args.get(name_index + 1);
    match name_value {
        Some(v) if !v.starts_with('-') => Some(v.to_owned()),
        _ => Some(String::new()),
    }
}
