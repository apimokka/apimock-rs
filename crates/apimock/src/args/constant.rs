pub const CONFIG_FILE_PATH_OPTION_NAMES: [&str; 2] = ["-c", "--config"];
pub const CONFIG_LISTENER_PORT_OPTION_NAMES: [&str; 2] = ["-p", "--port"];
pub const FALLBACK_RESPOND_DIR_PATH_OPTION_NAMES: [&str; 2] = ["-d", "--dir"];
pub const INIT_CONFIG_OPTION_NAMES: [&str; 1] = ["--init"];
pub const INCLUDES_MIDDLEWARE_OPTION_NAMES: [&str; 1] = ["--middleware"];
/// Skip every prompt in interactive `--init` and accept defaults.
///
/// Exists so that scripts / CI that previously relied on `--init`
/// running non-interactively can stay explicit: `apimock --init --yes`
/// is self-documenting about "I know this runs unattended". Without
/// this flag we still fall back to defaults when stdin isn't a TTY, so
/// `yes | apimock --init` keeps working too.
pub const YES_OPTION_NAMES: [&str; 2] = ["-y", "--yes"];

/// RFC 049: short-circuit before config is read or any listener binds.
pub const VERSION_OPTION_NAMES: [&str; 1] = ["--version"];
pub const HELP_OPTION_NAMES: [&str; 2] = ["--help", "-h"];

/// Every top-level flag name the "start the server" / `--init` surface
/// recognises. Used by RFC 049's unknown-argument rejection and its
/// near-match suggestion — **not** `match-test` / `validate`, which
/// parse their own, separate flag surface.
pub const KNOWN_TOP_LEVEL_OPTION_NAMES: [&str; 13] = [
    "-c",
    "--config",
    "-p",
    "--port",
    "-d",
    "--dir",
    "--init",
    "--middleware",
    "-y",
    "--yes",
    "--version",
    "--help",
    "-h",
];

pub const DEFAULT_CONFIG_FILE_PATH: &str = "./apimock.toml";
pub const DEFAULT_RULE_SET_FILE_PATH: &str = "./apimock-rule-set.toml";
pub const DEFAULT_MIDDLEWARE_FILE_PATH: &str = "./apimock-middleware.rhai";
