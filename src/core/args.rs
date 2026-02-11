use std::{env, fs, io, path::Path};

pub mod constant;

use constant::*;

/// env args passed at startup
#[derive(Clone)]
pub struct EnvArgs {
    /// config .toml file path
    pub config_file_path: Option<String>,
    /// overwrites value in config file
    pub port: Option<u16>,
    /// overwrites value in config file
    pub fallback_respond_dir_path: Option<String>,
}

impl EnvArgs {
    /// generate from env args, and set default if not specified at each arg
    pub fn default() -> Option<Self> {
        let mut ret = EnvArgs::from_args();

        let init_config = args_option_value(INIT_CONFIG_OPTION_NAMES.to_vec().as_ref()).is_some();
        if init_config {
            let includes_middleware =
                args_option_value(INCLUDES_MIDDLEWARE_OPTION_NAMES.to_vec().as_ref()).is_some();
            // generate config files and quit
            match ret.init_config(includes_middleware) {
                Ok(_) => (),
                Err(err) => log::error!("failed to init config ({})", err),
            }
            return None;
        }

        ret.default_config_file_path();

        match ret.validate() {
            Ok(_) => Some(ret),
            Err(_) => panic!("something wrong in env args"),
        }
    }

    pub fn validate(&self) -> Result<(), ()> {
        if let Some(config_file_path) = self.config_file_path.as_ref() {
            if !Path::new(config_file_path.as_str()).exists() {
                panic!(
                    "config file was specified but didn't exist: {}",
                    config_file_path
                );
            }
        }

        if let Some(fallback_respond_dir_path) = self.fallback_respond_dir_path.as_ref() {
            if !Path::new(fallback_respond_dir_path.as_str()).exists() {
                panic!(
                    "fallback response dir doesn't exist: {}",
                    fallback_respond_dir_path
                );
            }
        }

        Ok(())
    }

    /// generate from env args
    fn from_args() -> Self {
        let port =
            if let Some(port) =
                args_option_value(CONFIG_LISTENER_PORT_OPTION_NAMES.to_vec().as_ref())
            {
                Some(port.parse::<u16>().expect(
                    format!("env arg `port` is {} - specified but not number", port).as_str(),
                ))
            } else {
                None
            };

        let ret = EnvArgs {
            config_file_path: args_option_value(CONFIG_FILE_PATH_OPTION_NAMES.to_vec().as_ref()),
            port,
            fallback_respond_dir_path: args_option_value(
                FALLBACK_RESPOND_DIR_PATH_OPTION_NAMES.to_vec().as_ref(),
            ),
        };

        ret
    }

    /// initialize config files
    fn init_config(&mut self, includes_middleware: bool) -> Result<(), io::Error> {
        if includes_middleware {
            if !Path::new(DEFAULT_MIDDLEWARE_FILE_PATH).exists() {
                let content = include_str!("../../examples/config/default/apimock-middleware.rhai");
                let _ = fs::write(DEFAULT_MIDDLEWARE_FILE_PATH, content)?;
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
        let _ = fs::write(DEFAULT_CONFIG_FILE_PATH, config_content)?;
        println!("root config file is created: {}.", DEFAULT_CONFIG_FILE_PATH);

        if !Path::new(DEFAULT_RULE_SET_FILE_PATH).exists() {
            let rule_set_content =
                include_str!("../../examples/config/default/apimock-rule-set.toml");
            let _ = fs::write(DEFAULT_RULE_SET_FILE_PATH, rule_set_content)?;
            println!(
                "rule set config file is created: {}.",
                DEFAULT_RULE_SET_FILE_PATH
            );
        }

        Ok(())
    }

    /// app config file path
    ///
    /// - if specified in arguments, use it
    /// - else if default file exists, use it
    /// - else miss it
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

/// arguments: get option value if the option name is found
fn args_option_value(option_names: &Vec<&str>) -> Option<String> {
    let args: Vec<String> = env::args().collect();

    let name_index = args.iter().position(|arg| {
        option_names
            .iter()
            .any(|option_name| arg.as_str().eq(*option_name))
    });

    if let Some(name_index) = name_index {
        let name_value = args.get(name_index + 1);
        let ret = match name_value {
            Some(name_value) if !name_value.starts_with("-") => Some(name_value.to_owned()),
            _ => Some(String::new()),
        };
        return ret;
    }

    None
}
