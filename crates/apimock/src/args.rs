use std::{env, fs, io, path::Path};

pub mod constant;
pub mod init_interactive;

use constant::*;

use anyhow::{Result as AppResult, bail};

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
#[non_exhaustive]
pub struct EnvArgs {
    /// path to the config TOML file (usually `./apimock.toml`)
    pub config_file_path: Option<String>,
    /// overrides `listener.port` in the config file
    pub port: Option<u16>,
    /// overrides `service.fallback_respond_dir` in the config file
    pub fallback_respond_dir_path: Option<String>,
}

impl EnvArgs {
    /// Every field unset.
    ///
    /// Distinct from [`EnvArgs::default`] below, which parses
    /// `env::args()` and is fallible — this is the plain constructor
    /// `#[non_exhaustive]` requires now that struct-literal construction
    /// is unavailable from outside the crate. Named `empty` rather than
    /// `new` or `default` because both those names are already taken by
    /// unrelated methods on this type.
    pub fn empty() -> Self {
        Self {
            config_file_path: None,
            port: None,
            fallback_respond_dir_path: None,
        }
    }

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
    // clippy: renaming `default` would change apimock::args::EnvArgs's
    // public API surface; this is a fallible constructor, not the
    // std::default::Default trait's method.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> AppResult<Option<Self>> {
        let raw: Vec<String> = env::args().collect();

        // `--version` / `--help` short-circuit before anything else -
        // before a config file is read and before any listener binds
        // (RFC 049 Goals 2/3). This must work in a directory with no
        // config file, and in one with a deliberately broken config:
        // "what version am I running" is asked precisely when something
        // is wrong, so it can't depend on config loading having
        // succeeded. `--help` is reachable per subcommand too - the
        // subcommand name (if any) is `raw.get(1)`.
        if any_present(&raw, VERSION_OPTION_NAMES.as_ref()) {
            println!("apimock {}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }
        if any_present(&raw, HELP_OPTION_NAMES.as_ref()) {
            println!("{}", help_text(raw.get(1).map(String::as_str)));
            std::process::exit(0);
        }

        // `apimock match-test …` — dry-run rule matching.
        if raw.get(1).map(String::as_str) == Some("match-test") {
            std::process::exit(crate::cmd::match_test::run(&raw[2..]));
        }

        // `apimock validate …` — validate config without starting the server.
        if raw.get(1).map(String::as_str) == Some("validate") {
            std::process::exit(crate::cmd::validate::run(&raw[2..]));
        }

        // `apimock get …` — what would the server return? (RFC 055)
        if raw.get(1).map(String::as_str) == Some("get") {
            std::process::exit(crate::cmd::get::run(&raw[2..]));
        }

        // `apimock set …` — add or change a rule. (RFC 057)
        if raw.get(1).map(String::as_str) == Some("set") {
            std::process::exit(crate::cmd::set::run(&raw[2..]));
        }

        // RFC 048 § 6 / RFC 059 / RFC 064: a bare word in the
        // subcommand position that matched none of the four above is an
        // unknown subcommand, not silent permission to start a server.
        // Checked only for a token that does *not* start with `-` — one
        // that does is a flag attempt at position 1, `-p`/`-c`/`--init`
        // and friends, left for `reject_unknown_arguments` just below.
        // `filter` also means bare `apimock` (no position 1 at all)
        // never reaches this: zero-config still starts the server.
        if let Some(candidate) = raw.get(1).filter(|a| !a.starts_with('-')) {
            exit_unknown_subcommand(candidate);
        }

        // RFC 049 Goal 1: anything left that looks like a flag and isn't
        // one of the top-level names above is unrecognised and must
        // error, not be silently discarded. Applied before `--init`
        // branches so both the "start the server" and "--init" surfaces
        // get the same treatment - the defect this closes isn't
        // specific to either.
        reject_unknown_arguments(&raw);

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
        if let Some(config_file_path) = self.config_file_path.as_ref()
            && !Path::new(config_file_path.as_str()).exists()
        {
            bail!(
                "config file specified via --config does not exist: {}",
                config_file_path
            );
        }

        if let Some(fallback_respond_dir_path) = self.fallback_respond_dir_path.as_ref()
            && !Path::new(fallback_respond_dir_path.as_str()).exists()
        {
            bail!(
                "fallback response dir specified via --dir does not exist: {}",
                fallback_respond_dir_path
            );
        }

        Ok(())
    }

    /// Build an `EnvArgs` by reading `env::args()`.
    fn from_args() -> AppResult<Self> {
        // RFC 049: an invalid --port value is a usage error (exit 2), not
        // "everything else" (exit 1) - it's caught before any config is
        // read or listener bound, same as an unknown flag.
        let port = args_option_value(CONFIG_LISTENER_PORT_OPTION_NAMES.as_ref()).map(|port_str| {
            port_str.parse::<u16>().unwrap_or_else(|_| {
                exit_usage_error(&format!("invalid value for --port: '{}'", port_str))
            })
        });

        Ok(EnvArgs {
            // RFC 049 Goal 4: a bare relative `--config apimock.toml`
            // must resolve the same as `--config ./apimock.toml`. The
            // actual defect is downstream, in
            // `apimock_config::path_util` (`Path::parent()` returns
            // `Some("")` for a bare filename, not `None`, and
            // canonicalizing "" fails) - but this RFC's scope is the CLI
            // surface only, so the fix is applied to the input here,
            // before it ever reaches config loading, rather than to
            // config loading itself.
            config_file_path: args_option_value(CONFIG_FILE_PATH_OPTION_NAMES.as_ref())
                .map(normalize_bare_relative_path),
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
                let content = include_str!("../examples/config/default/apimock-middleware.rhai");
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
            let rule_set_content = include_str!("../examples/config/default/apimock-rule-set.toml");
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
/// `env::args()`, in either the space form (`--config path`) or the
/// `=` form (`--config=path`, RFC 064 Amendment 1).
///
/// For flags that don't take a value (e.g. `--init`), returns `Some("")`
/// so the caller can check `.is_some()` without caring about the
/// payload — reachable here only for the space form, since
/// `reject_unknown_arguments` (called before this, in
/// `EnvArgs::default`) already rejects a no-value flag given any `=`
/// form as a usage error before this function ever runs.
fn args_option_value(option_names: &[&str]) -> Option<String> {
    let args: Vec<String> = env::args().collect();

    for (i, arg) in args.iter().enumerate() {
        if let Some((name, value)) = crate::cmd::flags::split_equals_form(arg) {
            if option_names.contains(&name) {
                return Some(value.to_owned());
            }
            continue;
        }
        if option_names.contains(&arg.as_str()) {
            return match args.get(i + 1) {
                Some(v) if !v.starts_with('-') => Some(v.to_owned()),
                _ => Some(String::new()),
            };
        }
    }
    None
}

/// True if any of `raw` matches one of `option_names` exactly.
fn any_present(raw: &[String], option_names: &[&str]) -> bool {
    raw.iter().any(|arg| option_names.contains(&arg.as_str()))
}

/// Print a usage-error message to stderr and exit 2 (RFC 049: "usage
/// error - unknown option, missing or invalid value"), without
/// producing any output on stdout and without starting a server.
fn exit_usage_error(message: &str) -> ! {
    eprintln!("apimock: {}", message);
    std::process::exit(2);
}

/// RFC 048 § 6 / RFC 059 / RFC 064: an unknown bare subcommand (`apimock
/// banana`, a typo like `apimock validat`) is a usage error, not silent
/// permission to start a server — the same "don't silently absorb
/// unknown input" contract RFC 059 already gives unknown flags, closed
/// here for the one shape it left open. Suggests a near match against
/// the four real subcommands where the edit distance makes one
/// plausible (`validat` → `validate`, `gett`/`st` → `get`/`set`), same
/// shared `near_match` and threshold as every flag suggestion in this
/// CLI — reused deliberately rather than given its own tuned threshold,
/// so a subcommand typo and a flag typo are judged "close enough" the
/// same way. One consequence measured, not assumed: `gte` (a two-letter
/// transposition of `get`, itself only 3 characters) sits right at
/// `near_match`'s threshold for its length and doesn't suggest — see
/// the review package for this task.
fn exit_unknown_subcommand(candidate: &str) -> ! {
    match near_match(candidate, KNOWN_SUBCOMMAND_NAMES.as_ref()) {
        Some(suggestion) => exit_usage_error(&format!(
            "unknown subcommand '{}'; did you mean '{}'?",
            candidate, suggestion
        )),
        None => exit_usage_error(&format!("unknown subcommand '{}'", candidate)),
    }
}

/// RFC 049 Goal 1: after every known top-level flag name is accounted
/// for, anything left that looks like a flag (starts with `-`) is
/// unrecognised. Exits 2 naming the offender, with a near-match
/// suggestion where one exists - the difference between a dead end and
/// a self-correction, for a person and for an agent alike.
///
/// Delegates the actual scan (skip-next for a value-taking flag's
/// value, `--flag=value` recognition, the no-value-flag-plus-`=` gate)
/// to `crate::cmd::flags::reject_unknown_flags` — RFC 064 Amendment 1
/// folded this function's own copy of that logic into the one already
/// shared by `get`/`validate`/`match-test`/`set`, rather than adding
/// `=`-form support to a fourth private copy. `strict_bare_tokens:
/// false` preserves this command's own pre-existing leniency: a bare
/// word that isn't a known flag (e.g. a stray positional-looking
/// token) was never rejected here, and still isn't.
///
/// `raw[0]` (the binary path) is skipped by starting the delegated
/// scan at `raw[1..]`. A subcommand name at `raw[1]` never reaches this
/// function at all — `EnvArgs::default` dispatches and exits for all
/// four before this runs, and `exit_unknown_subcommand` catches any
/// other bare token there first (RFC 048 § 6 / RFC 059 / RFC 064). So
/// `raw[1]` is always either a real subcommand or a `-`-prefixed flag
/// by the time this function's scan reaches it — no special case for
/// subcommand names is needed here; the leniency below
/// (`strict_bare_tokens: false`) is about a *later* bare token, like
/// `get`'s positional `<path>`, not position 1.
///
/// The `"unknown option"` phrase (rather than `reject_unknown_flags`'s
/// own default `"unrecognized argument"`) matches this function's
/// pre-existing wording, pinned by `crates/apimock/tests/args.rs`.
fn reject_unknown_arguments(raw: &[String]) {
    if let Err(e) = crate::cmd::flags::reject_unknown_flags(
        raw.get(1..).unwrap_or(&[]),
        KNOWN_TOP_LEVEL_OPTION_NAMES.as_ref(),
        NO_VALUE_TOP_LEVEL_OPTION_NAMES.as_ref(),
        false,
        "unknown option",
    ) {
        exit_usage_error(&e);
    }
}

/// Find the closest known flag to an unrecognised one, if the edit
/// distance is small enough relative to length to be a plausible typo
/// rather than an unrelated word.
pub(crate) fn near_match<'a>(unknown: &str, known: &[&'a str]) -> Option<&'a str> {
    known
        .iter()
        .map(|&candidate| (candidate, edit_distance(unknown, candidate)))
        .filter(|&(candidate, distance)| {
            distance > 0 && distance <= (unknown.len().max(candidate.len()) / 3).max(1)
        })
        .min_by_key(|&(_, distance)| distance)
        .map(|(candidate, _)| candidate)
}

/// Levenshtein edit distance, for [`near_match`].
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];

    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b.len()]
}

/// RFC 049 Goal 4: give a bare relative path (no directory component,
/// e.g. `"apimock.toml"`) an explicit `./` prefix, so it resolves the
/// same way `"./apimock.toml"` already does. Absolute paths and paths
/// that already have a directory component (including a leading `./`
/// or `../`) are returned unchanged.
fn normalize_bare_relative_path(path: String) -> String {
    // `args_option_value` returns `Some("")` for a value-taking flag
    // given with nothing after it (the same encoding it uses for a
    // boolean flag's mere presence) - already a meaningless invocation
    // either way, but prepending "./" to it would turn "path not given"
    // into "path is the current directory", a more confusing failure
    // than the original "no such file" for an empty path. Leave it
    // alone so it fails the same way it always did.
    if path.is_empty() {
        return path;
    }

    let p = Path::new(&path);
    let has_directory_component = p
        .parent()
        .is_some_and(|parent| !parent.as_os_str().is_empty());
    if has_directory_component || p.is_absolute() {
        path
    } else {
        format!("./{}", path)
    }
}

/// Top-level usage text, or a subcommand's, matching
/// `docs/src/reference/cli-reference.md`.
fn help_text(subcommand: Option<&str>) -> &'static str {
    match subcommand {
        Some("match-test") => {
            "apimock match-test --rule-set <path> [--rule <n>] [--path <url_path>] \\\n  [--method <METHOD>] [--header \"Name: value\"]... \\\n  [--body <json> | --body-file <path>] [--quiet]\n\nBuilds a synthetic request from the flags below and checks it against\na rule set directly - no server, no network request.\n\n  --rule-set, -r <path>       Required. The rule-set file to check against\n  --rule <n>                  Check only this rule, 1-based\n  --path, -p <url_path>       The synthetic request's URL path\n  --method, -m <METHOD>       The synthetic request's HTTP method\n  --header, -H \"Name: value\"  Add a header; repeatable\n  --body, -b <json>           The synthetic request's JSON body, inline\n  --body-file <path>          The synthetic request's JSON body, from a file\n  --quiet, -q                 Suppress the per-condition breakdown\n\nExit codes: 0 matched, 1 no rule matched, 2 an argument or input error."
        }
        Some("validate") => {
            "apimock validate --config <path> [--strict] [--quiet] [--format text|json]\n\nLoads the whole workspace - root config and every rule set it\nreferences - and reports diagnostics, without binding a port.\n\n  --config, -c <path>  Required. The root config to validate\n  --strict             Treat warnings as failures too\n  --quiet              Suppress non-error output\n  --format text|json   text (default): today's output. json: the RFC 053 response envelope\n\n--json was removed in 6.0.0; use --format json instead.\n\nExit codes: 0 clean, 1 at least one error, 2 the config couldn't be loaded, or a bad invocation."
        }
        Some("get") => {
            "apimock get <path> [-c <config>] [-m <METHOD>] [-H \"Name: value\"]... \\\n  [-b <json> | --body-file <path>] [--why] [--format text|json]\n\nAnswers what the server would return for this request - status, body,\nheaders - from configuration on disk, no server running. Covers the\nsame dispatch order the server uses: OPTIONS, then rule sets, then the\nfallback directory (zero-config mode included). Configured middleware\nis never executed; if any is configured, the answer says so and is\nmarked incomplete rather than silently skipping it.\n\n  --config, -c <path>   The root config to answer from (default: ./apimock.toml, or zero-config if absent)\n  --method, -m <METHOD> The request's HTTP method (default: GET)\n  --header, -H \"Name: value\"  Add a header; repeatable\n  --body, -b <json>     The request's JSON body, inline\n  --body-file <path>    The request's JSON body, from a file\n  --why                 Explain which rule decided the answer, and which condition failed for a near-miss. Off by default in text, on by default with --format json\n  --format text|json    text (default): human-readable. json: the RFC 053 response envelope, with provenance\n\nmatch-test still exits 1 on no match; get exits 0 with a result saying\nso - deliberately different, per RFC 053.\n\nExit codes: 0 answered (including a 404 or no match), 2 a bad invocation or the config couldn't be loaded."
        }
        Some("set") => {
            "apimock set rule [-c <config>] [--rule-set <path>] [--rule <n>] \\\n  [--path <url_path>] [--method <METHOD>] [-H \"Name: value\"]... \\\n  [--status <code>] [--json <value> | --text <value>] [--file <path>] \\\n  [--delay <ms>] [--dry-run] [--format text|json]\n\nAdds a rule (default), or changes an existing one when --rule is\ngiven, and writes it to the rule-set file - keeping that file's\ncomments and formatting (RFC 056). Neither the root config nor the\nrule-set file need to exist yet; a fresh directory gets a minimal\nstarting pair of files.\n\nAddressing is by (rule-set file, 0-based rule index) - never a\nprocess-local id - so an address printed by `get --why` can be passed\nto --rule-set/--rule unmodified.\n\n  --config, -c <path>   The root config to edit (default: ./apimock.toml, created if absent)\n  --rule-set <path>     The rule-set file to add to or edit within (default: ./apimock-rule-set.toml, created if absent)\n  --rule <n>            Edit the existing rule at this 0-based index, instead of adding a new one\n  --path <url_path>     The rule's url_path condition\n  --method <METHOD>     The rule's method condition\n  --header, -H \"Name: value\"  Add a header condition; repeatable. With --rule, layers onto the existing rule's conditions rather than replacing them\n  --status <code>       The response status code\n  --json <value>        The response body, as JSON (validated). Writes respond.json, served as application/json\n  --text <value>        The response body, as plain text (mutually exclusive with --json)\n  --file <path>         The response body, served from a file\n  --delay <ms>          Delay the response by this many milliseconds\n  --dry-run             Show what would change, without writing anything\n  --format text|json    text (default): human-readable. json: the RFC 053 response envelope\n\nA config file changed on disk since it was loaded fails the save with\nerror.kind \"conflict\", rather than overwriting it (RFC 056).\n\nExit codes: 0 applied (or, under --dry-run, would apply), 1 loaded and addressed successfully but the save failed, 2 a bad invocation or the config couldn't be loaded."
        }
        _ => {
            "apimock [-p <port>] [-d <dir>] [-c <config>] [--init [--yes] [--middleware]]\n\nRun with no flags to serve the current directory: zero-config mode\nserves ./ by URL path on port 3001, or ./apimock.toml if it exists.\n\n  -c, --config <path>  Load a config file (a bare relative path resolves\n                       the same as one prefixed with ./)\n  -p, --port <port>    Listen on a custom port\n  -d, --dir <dir>      Serve a custom fallback directory instead of ./\n  --init               Scaffold a starting config in the current directory\n  --yes                With --init, skip prompts and accept defaults\n  --middleware         With --init, also scaffold a middleware file\n  -h, --help           Print this help and exit\n  --version            Print the version and exit\n\nSubcommands:\n  get         What would the server return for this request? No server\n  set         Add or change a rule, and write it to disk\n  match-test  Dry-run a rule match against a rule set, no server\n  validate    Validate a config, no server\n\nRun 'apimock <subcommand> --help' for subcommand-specific help."
        }
    }
}
