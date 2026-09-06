//! Interactive `--init` prompt flow.
//!
//! # Why this is its own module
//!
//! The non-interactive init (`EnvArgs::init_config`) is ~40 lines of
//! straight-line file writes. Interactive init brings in TTY detection,
//! prompt I/O, answer validation, and TOML templating — all of which
//! would balloon `args.rs` if inlined. Splitting it out keeps the
//! existing code readable and lets us swap the prompt backend (e.g. to
//! `dialoguer` later) without touching the arg parser.
//!
//! # Why no prompt-library dependency
//!
//! The project README advertises a tiny boot footprint. `dialoguer` /
//! `inquire` / `requestty` each pull in a terminal-handling stack
//! (`crossterm`, `termion`, …) that we don't otherwise need. The three
//! prompt shapes we actually use — string-with-default, yes/no,
//! bounded-integer — fit in ~60 lines of plain `std::io`, which is the
//! right trade for this codebase.
//!
//! # Why non-TTY falls back to defaults
//!
//! Invoking `apimock --init` in a CI job, Docker build, or piped shell
//! (`yes | apimock --init`) used to work reliably: fixed boilerplate,
//! no user interaction. Blocking on a prompt that nobody is watching
//! would break that. `IsTerminal::is_terminal` on stdin/stdout lets us
//! detect this and silently take defaults, preserving the contract.

use std::io::{self, BufRead, IsTerminal, Write};

use super::constant::{
    DEFAULT_CONFIG_FILE_PATH, DEFAULT_MIDDLEWARE_FILE_PATH, DEFAULT_RULE_SET_FILE_PATH,
};

/// Answers collected from the user (or defaults) during `--init`.
#[derive(Clone, Debug)]
pub struct InitAnswers {
    pub ip_address: String,
    pub port: u16,
    pub fallback_respond_dir: String,
    pub include_rule_set: bool,
    pub include_middleware: bool,
    pub enable_tls: bool,
}

impl Default for InitAnswers {
    /// The defaults used when stdin is not a terminal, when `--yes` is
    /// passed, or when the user hits Enter on each prompt.
    ///
    /// These match the pre-4.8.0 non-interactive init output, so piping
    /// `apimock --init` in CI yields the same files it always has.
    fn default() -> Self {
        Self {
            ip_address: "127.0.0.1".to_owned(),
            port: 3001,
            fallback_respond_dir: ".".to_owned(),
            include_rule_set: true,
            // Matches the existing opt-in behaviour driven by --middleware.
            include_middleware: false,
            enable_tls: false,
        }
    }
}

/// Whether the current session can actually ask the user something.
///
/// We require *both* stdin and stdout to be terminals: stdin so input
/// will ever arrive, stdout so the prompt text isn't silently eaten by
/// a redirect and followed by a visible hang.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Run the interactive prompt flow.
///
/// `force_defaults` short-circuits every prompt. It's driven by the
/// `--yes` / `-y` CLI flag, intended for users who prefer the old
/// "just drop the boilerplate and go" experience.
///
/// If `--middleware` was passed on the CLI, `cli_middleware_override`
/// pre-seeds the middleware answer to `true` so we don't contradict the
/// flag — but we still show the other prompts.
pub fn run(force_defaults: bool, cli_middleware_override: bool) -> io::Result<InitAnswers> {
    let mut answers = InitAnswers::default();
    if cli_middleware_override {
        answers.include_middleware = true;
    }

    if force_defaults || !is_interactive() {
        return Ok(answers);
    }

    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    writeln!(
        stdout,
        "apimock --init: setting up a new mock-server config. Press Enter to accept any default."
    )?;

    answers.ip_address = prompt_string(
        &mut stdin,
        &mut stdout,
        "Listener IP address",
        &answers.ip_address,
    )?;
    answers.port = prompt_u16(&mut stdin, &mut stdout, "Listener port", answers.port)?;
    answers.fallback_respond_dir = prompt_string(
        &mut stdin,
        &mut stdout,
        "Fallback response directory (served when no rule matches)",
        &answers.fallback_respond_dir,
    )?;
    answers.include_rule_set = prompt_yes_no(
        &mut stdin,
        &mut stdout,
        "Create an example rule-set file",
        answers.include_rule_set,
    )?;
    // Only ask about middleware if the CLI flag didn't already force it on.
    if !cli_middleware_override {
        answers.include_middleware = prompt_yes_no(
            &mut stdin,
            &mut stdout,
            "Create a middleware (Rhai script) file",
            answers.include_middleware,
        )?;
    }
    answers.enable_tls = prompt_yes_no(
        &mut stdin,
        &mut stdout,
        "Enable HTTPS (TLS) section in the config? (you'll need cert/key files)",
        answers.enable_tls,
    )?;

    writeln!(stdout)?;

    Ok(answers)
}

/// Render the `apimock.toml` file contents from the collected answers.
///
/// # Why we template here and don't use `include_str!`
///
/// The pre-4.8.0 init wrote a fixed example file. To honour user answers
/// we need at least port, IP, fallback dir, and whether TLS is commented
/// in. A tiny `format!` is simpler than introducing a templating engine
/// and produces output that stays diff-friendly next to the old output.
pub fn render_apimock_toml(answers: &InitAnswers) -> String {
    let tls_block = if answers.enable_tls {
        "[listener.tls]\ncert = \"./cert.pem\"\nkey = \"./key.pem\"\n# port = 3002\n"
    } else {
        "# [listener.tls]\n# cert = \"./cert.pem\"\n# key = \"./key.pem\"\n# # port = 3002\n"
    };

    let rule_sets_line = if answers.include_rule_set {
        format!("rule_sets = [\"{}\"]\n", DEFAULT_RULE_SET_FILE_PATH_REL)
    } else {
        "# rule_sets = [\"apimock-rule-set.toml\"]\n".to_owned()
    };

    let middlewares_line = if answers.include_middleware {
        format!("middlewares = [\"{}\"]\n", DEFAULT_MIDDLEWARE_FILE_PATH_REL)
    } else {
        "# middlewares = []\n".to_owned()
    };

    format!(
        "# Users Guide:\n\
         # https://apimokka.github.io/apimock-rs/\n\
         \n\
         [listener]\n\
         ip_address = \"{ip}\"\n\
         port = {port}\n\
         {tls_block}\n\
         [log]\n\
         verbose = {{ header = true, body = true }}\n\
         \n\
         [service]\n\
         {rule_sets_line}\
         {middlewares_line}\
         fallback_respond_dir = \"{fallback}\"\n",
        ip = answers.ip_address,
        port = answers.port,
        tls_block = tls_block,
        rule_sets_line = rule_sets_line,
        middlewares_line = middlewares_line,
        fallback = answers.fallback_respond_dir,
    )
}

// -- prompt primitives -------------------------------------------------

/// Ask for a string, returning `default` when the user hits Enter.
fn prompt_string(
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    label: &str,
    default: &str,
) -> io::Result<String> {
    write!(stdout, "{} [{}]: ", label, default)?;
    stdout.flush()?;

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    let trimmed = line.trim();
    Ok(if trimmed.is_empty() {
        default.to_owned()
    } else {
        trimmed.to_owned()
    })
}

/// Ask for a `u16` (port), re-prompting once on a parse failure before
/// falling back to the default. We don't loop forever: if a user is
/// typing nonsense and then giving up, silently accepting the default is
/// better than holding the process open.
fn prompt_u16(
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    label: &str,
    default: u16,
) -> io::Result<u16> {
    for attempt in 0..2 {
        write!(stdout, "{} [{}]: ", label, default)?;
        stdout.flush()?;

        let mut line = String::new();
        let read = stdin.read_line(&mut line)?;
        let trimmed = line.trim();

        if trimmed.is_empty() || read == 0 {
            return Ok(default);
        }
        match trimmed.parse::<u16>() {
            Ok(v) => return Ok(v),
            Err(_) if attempt == 0 => {
                writeln!(
                    stdout,
                    "  '{}' is not a valid port (must be 0..=65535). Try again.",
                    trimmed
                )?;
            }
            Err(_) => {
                writeln!(stdout, "  Using default {}.", default)?;
                return Ok(default);
            }
        }
    }
    Ok(default)
}

/// Ask a yes/no question. Accepts `y`, `yes`, `n`, `no` (any case).
/// Anything else, including empty input, returns `default`.
fn prompt_yes_no(
    stdin: &mut impl BufRead,
    stdout: &mut impl Write,
    label: &str,
    default: bool,
) -> io::Result<bool> {
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    write!(stdout, "{} {}: ", label, hint)?;
    stdout.flush()?;

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(match answer.as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    })
}

// Relative paths as written inside the config file (these are the
// scaffolded filenames *without* leading `./`, since that's how operators
// tend to read them back).
const DEFAULT_RULE_SET_FILE_PATH_REL: &str = "apimock-rule-set.toml";
const DEFAULT_MIDDLEWARE_FILE_PATH_REL: &str = "apimock-middleware.rhai";

/// Summary printed after a successful init so the user sees what we did.
pub fn print_summary(answers: &InitAnswers) {
    println!();
    println!("Created:");
    println!("  - {}", DEFAULT_CONFIG_FILE_PATH);
    if answers.include_rule_set {
        println!("  - {}", DEFAULT_RULE_SET_FILE_PATH);
    }
    if answers.include_middleware {
        println!("  - {}", DEFAULT_MIDDLEWARE_FILE_PATH);
    }
    println!(
        "\nStart the server with `apimock` (reads {} from the current directory).",
        DEFAULT_CONFIG_FILE_PATH
    );
    if answers.enable_tls {
        println!(
            "\n[!] TLS was enabled. Place `cert.pem` and `key.pem` next to {} before starting.",
            DEFAULT_CONFIG_FILE_PATH,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_defaults_match_legacy_shape() {
        let out = render_apimock_toml(&InitAnswers::default());
        assert!(out.contains("ip_address = \"127.0.0.1\""));
        assert!(out.contains("port = 3001"));
        assert!(out.contains("fallback_respond_dir = \".\""));
        // TLS commented out by default
        assert!(out.contains("# [listener.tls]"));
        // Rule set included by default
        assert!(out.contains("rule_sets = [\"apimock-rule-set.toml\"]"));
        // Middleware commented out by default
        assert!(out.contains("# middlewares = []"));
    }

    #[test]
    fn render_with_tls_enabled() {
        let answers = InitAnswers {
            enable_tls: true,
            ..InitAnswers::default()
        };
        let out = render_apimock_toml(&answers);
        assert!(out.contains("[listener.tls]\ncert = \"./cert.pem\""));
        assert!(!out.contains("# [listener.tls]"));
    }

    #[test]
    fn render_with_middleware_enabled() {
        let answers = InitAnswers {
            include_middleware: true,
            ..InitAnswers::default()
        };
        let out = render_apimock_toml(&answers);
        assert!(out.contains("middlewares = [\"apimock-middleware.rhai\"]"));
    }

    #[test]
    fn render_custom_port_and_ip() {
        let answers = InitAnswers {
            ip_address: "0.0.0.0".to_owned(),
            port: 8080,
            ..InitAnswers::default()
        };
        let out = render_apimock_toml(&answers);
        assert!(out.contains("ip_address = \"0.0.0.0\""));
        assert!(out.contains("port = 8080"));
    }

    #[test]
    fn prompt_string_uses_default_on_empty() {
        let input = b"\n";
        let mut out = Vec::new();
        let got = prompt_string(&mut &input[..], &mut out, "Label", "the-default").unwrap();
        assert_eq!(got, "the-default");
    }

    #[test]
    fn prompt_string_uses_typed_value() {
        let input = b"custom-value\n";
        let mut out = Vec::new();
        let got = prompt_string(&mut &input[..], &mut out, "Label", "default").unwrap();
        assert_eq!(got, "custom-value");
    }

    #[test]
    fn prompt_u16_parses_valid_input() {
        let input = b"4000\n";
        let mut out = Vec::new();
        let got = prompt_u16(&mut &input[..], &mut out, "Port", 3001).unwrap();
        assert_eq!(got, 4000);
    }

    #[test]
    fn prompt_u16_falls_back_after_two_bad_attempts() {
        let input = b"abc\nalso-bad\n";
        let mut out = Vec::new();
        let got = prompt_u16(&mut &input[..], &mut out, "Port", 3001).unwrap();
        assert_eq!(got, 3001);
    }

    #[test]
    fn prompt_u16_uses_default_on_empty() {
        let input = b"\n";
        let mut out = Vec::new();
        let got = prompt_u16(&mut &input[..], &mut out, "Port", 3001).unwrap();
        assert_eq!(got, 3001);
    }

    #[test]
    fn prompt_yes_no_accepts_various_forms() {
        for (input_bytes, default, expected) in [
            (&b"y\n"[..], false, true),
            (&b"Y\n"[..], false, true),
            (&b"yes\n"[..], false, true),
            (&b"YES\n"[..], false, true),
            (&b"n\n"[..], true, false),
            (&b"no\n"[..], true, false),
            (&b"\n"[..], true, true),
            (&b"\n"[..], false, false),
            (&b"maybe\n"[..], true, true),
        ] {
            let mut out = Vec::new();
            let got = prompt_yes_no(&mut &input_bytes[..], &mut out, "Q", default).unwrap();
            assert_eq!(got, expected, "input={:?} default={}", input_bytes, default);
        }
    }
}
