use std::{env, fs, io, path::Path};

pub mod constant;
pub mod init_interactive;

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
            let force_defaults = args_option_value(YES_OPTION_NAMES.as_ref()).is_some();
            // Drive the interactive prompt (or fall back to defaults in
            // non-TTY / --yes contexts). We log but don't propagate the
            // error: a failed init is a user-level problem, and forcing
            // the binary to exit 1 on a partial write would be more
            // disruptive than informative.
            if let Err(err) = ret.init_config_interactive(includes_middleware, force_defaults) {
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

    /// Scaffold `apimock.toml` (and related files) into the current directory,
    /// driven by interactive prompts when stdin is a TTY.
    ///
    /// Files that already exist are left untouched — `--init` is a
    /// convenience for fresh directories, not an overwrite tool.
    ///
    /// # Why this never returns an error for an existing config file
    ///
    /// If the operator already has an `apimock.toml`, bailing out with a
    /// non-zero exit would break repeatable idempotent scripts that run
    /// `--init` before starting the server. Printing a warning and
    /// continuing preserves that usage pattern.
    fn init_config_interactive(
        &mut self,
        cli_middleware_override: bool,
        force_defaults: bool,
    ) -> Result<(), io::Error> {
        // Early exit if the root config already exists — we never overwrite
        // it, and asking a barrage of questions we're about to ignore would
        // waste the user's time.
        if Path::new(DEFAULT_CONFIG_FILE_PATH).exists() {
            println!(
                "[warn] quit because default root config file exists: {}.",
                DEFAULT_CONFIG_FILE_PATH
            );
            return Ok(());
        }

        let answers = init_interactive::run(force_defaults, cli_middleware_override)?;

        // Middleware file — honours both the CLI flag and the interactive answer.
        if answers.include_middleware {
            if !Path::new(DEFAULT_MIDDLEWARE_FILE_PATH).exists() {
                let content =
                    include_str!("../../examples/config/default/apimock-middleware.rhai");
                fs::write(DEFAULT_MIDDLEWARE_FILE_PATH, content)?;
                println!(
                    "middleware scripting file is created: {}.",
                    DEFAULT_MIDDLEWARE_FILE_PATH
                );
            } else {
                println!(
                    "[warn] middleware scripting file exists: {}.",
                    DEFAULT_MIDDLEWARE_FILE_PATH
                );
            }
        }

        // Root config — templated from the collected answers so the file
        // reflects the user's actual choices rather than a fixed example.
        let config_content = init_interactive::render_apimock_toml(&answers);
        fs::write(DEFAULT_CONFIG_FILE_PATH, config_content)?;
        println!("root config file is created: {}.", DEFAULT_CONFIG_FILE_PATH);

        // Rule set file — still the example content, because customising
        // rule shapes interactively would be a much larger prompt tree
        // for diminishing value. Users are expected to edit this file.
        if answers.include_rule_set && !Path::new(DEFAULT_RULE_SET_FILE_PATH).exists() {
            let rule_set_content =
                include_str!("../../examples/config/default/apimock-rule-set.toml");
            fs::write(DEFAULT_RULE_SET_FILE_PATH, rule_set_content)?;
            println!(
                "rule set config file is created: {}.",
                DEFAULT_RULE_SET_FILE_PATH
            );
        }

        init_interactive::print_summary(&answers);
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
