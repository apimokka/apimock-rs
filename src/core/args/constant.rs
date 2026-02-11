pub const CONFIG_FILE_PATH_OPTION_NAMES: [&str; 2] = ["-c", "--config"];
pub const CONFIG_LISTENER_PORT_OPTION_NAMES: [&str; 2] = ["-p", "--port"];
pub const FALLBACK_RESPOND_DIR_PATH_OPTION_NAMES: [&str; 2] = ["-d", "--dir"];
pub const INIT_CONFIG_OPTION_NAMES: [&str; 1] = ["--init"];
pub const INCLUDES_MIDDLEWARE_OPTION_NAMES: [&str; 1] = ["--middleware"];

pub const DEFAULT_CONFIG_FILE_PATH: &str = "./apimock.toml";
pub const DEFAULT_RULE_SET_FILE_PATH: &str = "./apimock-rule-set.toml";
pub const DEFAULT_MIDDLEWARE_FILE_PATH: &str = "./apimock-middleware.rhai";
