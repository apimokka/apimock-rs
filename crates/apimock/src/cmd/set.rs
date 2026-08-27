//! `apimock set` — add and change rules from the command line. (RFC 057)
//!
//! # Addressing — the reason this command's shape is what it is
//!
//! `apimock-config`'s `EditCommand` addresses every target by `NodeId`,
//! a fresh UUID minted per `Workspace::load()`. That is fine for a GUI
//! holding one `Workspace` for a session; it cannot work here, because
//! **every `apimock set` invocation is a new process** — a new
//! `load()`, so new IDs. An address printed by one invocation is
//! meaningless to the next.
//!
//! So `set` never lets a `NodeId` cross its process boundary. It
//! addresses a rule the same way `apimock get --why` already reports
//! one: a rule set's **file path**, plus a **0-based** rule index
//! (matching `get`'s JSON `matched` block — RFC 057's handoff § 1.3;
//! text output there is 1-based for humans, JSON stays 0-based for
//! machines, and `set` follows the machine convention since that is
//! the contract meant to compose). `apimock_config::Workspace` exposes
//! exactly the resolution this needs (`rule_set_id_at`/`rule_id_at`/
//! `respond_id_at`) and an address renderer (`describe`) for turning a
//! changed `NodeId` back into text for a preview — never the other way.
//!
//! # `--dry-run` never serialises a `NodeId`
//!
//! `SaveResult`/`DiffItem` both derive `Serialize`, and `DiffItem`'s
//! `target: NodeId` is `#[serde(transparent)]` over a `Uuid`. Naively
//! serialising either would put a bare per-process UUID into `set`'s
//! JSON output — exactly the thing this command exists to keep out.
//! `--dry-run`'s preview is built by hand from `Workspace::describe`,
//! never by serialising `SaveResult`/`DiffItem` directly. A regression
//! test greps the JSON output for a UUID pattern and asserts zero
//! matches, on every path including every error kind.
//!
//! # Scope (RFC 057's handoff § 3)
//!
//! One rule per invocation, added or updated: `AddRuleSet`, `AddRule`,
//! `UpdateRule`/`UpdateRespond`, `AddHeaderCondition` — the exact
//! `EditCommand` variants that only ever append, never renumber, since
//! that append-only property is what makes a positional address
//! survive from one invocation to the next. `DeleteRule`, `MoveRule`,
//! `RemoveRuleSet` and root settings are out of this cut for exactly
//! that reason, not by oversight. `service.middlewares` is untouched by
//! every path here (T2).
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! |---|---|
//! | 0 | Applied (or, under `--dry-run`, would apply) successfully. |
//! | 1 | Loaded and addressed successfully, but the save failed — conflict, io, or an internal error. |
//! | 2 | Bad invocation, or the configuration couldn't be loaded. |

use std::path::{Path, PathBuf};

use apimock_config::{
    EditCommand, HeaderConditionPayload, HeaderOp, RespondPayload, RulePayload, Workspace,
};

use crate::args::constant::{DEFAULT_CONFIG_FILE_PATH, DEFAULT_RULE_SET_FILE_PATH};

use super::envelope::{self, ErrorKind, Format};
use super::flags::{
    flag_present, flag_value, flag_values_all, reject_empty_path_value, reject_unknown_flags,
};

const CONFIG_NAMES: &[&str] = &["--config", "-c"];
const RULE_SET_NAMES: &[&str] = &["--rule-set"];
const RULE_NAMES: &[&str] = &["--rule"];
const PATH_NAMES: &[&str] = &["--path"];
const METHOD_NAMES: &[&str] = &["--method"];
const HEADER_NAMES: &[&str] = &["--header", "-H"];
const STATUS_NAMES: &[&str] = &["--status"];
const JSON_NAMES: &[&str] = &["--json"];
const TEXT_NAMES: &[&str] = &["--text"];
const FILE_NAMES: &[&str] = &["--file"];
const DELAY_NAMES: &[&str] = &["--delay"];
const DRY_RUN_FLAG: &str = "--dry-run";
const FORMAT_FLAG: &str = "--format";
/// RFC 062: opts out of confining `--rule-set` to the root config's own
/// directory tree. Named for what it does, not what it's for — see
/// `crate::cmd::set`'s module doc and `docs/src/reference/threat-model.md`
/// for why the default confines at all.
const ALLOW_OUTSIDE_FLAG: &str = "--allow-outside";
/// Flags that take no value — every other known flag does. Used both
/// to parse (`SetRuleArgs::parse` never has to guess) and to reject
/// unknown input (`reject_unknown_flags` needs to know which flag
/// consumes the token after it).
const NO_VALUE_FLAG_NAMES: &[&str] = &[DRY_RUN_FLAG, ALLOW_OUTSIDE_FLAG];

fn known_flag_names() -> Vec<&'static str> {
    [
        CONFIG_NAMES,
        RULE_SET_NAMES,
        RULE_NAMES,
        PATH_NAMES,
        METHOD_NAMES,
        HEADER_NAMES,
        STATUS_NAMES,
        JSON_NAMES,
        TEXT_NAMES,
        FILE_NAMES,
        DELAY_NAMES,
    ]
    .into_iter()
    .flatten()
    .copied()
    .chain([DRY_RUN_FLAG, FORMAT_FLAG, ALLOW_OUTSIDE_FLAG])
    .collect()
}

/// Minimal empty rule set — `RuleSet`'s `rules: Vec<Rule>` has no
/// `#[serde(default)]`, so a genuinely empty file fails to parse
/// (`missing field 'rules'`, confirmed empirically before writing
/// this). This is the smallest text that round-trips.
const EMPTY_RULE_SET_TOML: &str = "rules = []\n";

/// Minimal starter root config — deliberately *not* `--init`'s
/// wizard-driven `render_apimock_toml`, which pulls in example rules,
/// commented-out TLS/middleware blocks and prompt-flow semantics `set`
/// has no business triggering as a side effect of adding one rule. An
/// agent running `set rule` in a fresh directory should get exactly
/// what it asked for, nothing else.
const MINIMAL_APIMOCK_TOML: &str = "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n\n[service]\nfallback_respond_dir = \".\"\n";

// ── Argument model ──────────────────────────────────────────────────────

struct SetRuleArgs {
    config_path: Option<String>,
    rule_set: Option<String>,
    rule_index: Option<usize>,
    path: Option<String>,
    method: Option<String>,
    headers: Vec<(String, String)>,
    status: Option<u16>,
    json: Option<String>,
    text: Option<String>,
    file_path: Option<String>,
    delay_ms: Option<u32>,
    dry_run: bool,
    format: Option<Format>,
    allow_outside: bool,
}

impl SetRuleArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let config_path = reject_empty_path_value(CONFIG_NAMES, flag_value(args, CONFIG_NAMES)?)?;
        let rule_set = reject_empty_path_value(RULE_SET_NAMES, flag_value(args, RULE_SET_NAMES)?)?;
        let rule_index = flag_value(args, RULE_NAMES)?
            .map(|s| {
                s.parse::<usize>()
                    .map_err(|_| format!("--rule must be a non-negative integer, got '{}'", s))
            })
            .transpose()?;
        let path = flag_value(args, PATH_NAMES)?;
        let method = flag_value(args, METHOD_NAMES)?.map(|m| m.to_uppercase());
        let headers = flag_values_all(args, HEADER_NAMES)?
            .into_iter()
            .map(|h| {
                let idx = h
                    .find(':')
                    .ok_or_else(|| format!("--header must be 'Name: value', got '{}'", h))?;
                let name = h[..idx].trim().to_lowercase();
                let value = h[idx + 1..].trim().to_owned();
                Ok((name, value))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let status = flag_value(args, STATUS_NAMES)?
            .map(|s| {
                s.parse::<u16>()
                    .map_err(|_| format!("--status must be a valid HTTP status code, got '{}'", s))
            })
            .transpose()?;
        let json = flag_value(args, JSON_NAMES)?;
        if let Some(j) = json.as_deref() {
            serde_json::from_str::<serde_json::Value>(j)
                .map_err(|e| format!("--json is not valid JSON: {}", e))?;
        }
        let text = flag_value(args, TEXT_NAMES)?;
        if json.is_some() && text.is_some() {
            return Err("--json and --text are mutually exclusive".to_owned());
        }
        let file_path = reject_empty_path_value(FILE_NAMES, flag_value(args, FILE_NAMES)?)?;
        let delay_ms = flag_value(args, DELAY_NAMES)?
            .map(|s| {
                s.parse::<u32>()
                    .map_err(|_| format!("--delay must be a non-negative integer, got '{}'", s))
            })
            .transpose()?;
        let dry_run = flag_present(args, &[DRY_RUN_FLAG]);
        let allow_outside = flag_present(args, &[ALLOW_OUTSIDE_FLAG]);

        let format_raw = flag_value(args, &[FORMAT_FLAG])?;
        let format = match format_raw.as_deref() {
            None => None,
            Some("text") => Some(Format::Text),
            Some("json") => Some(Format::Json),
            Some(other) => {
                return Err(format!(
                    "invalid value for --format: '{}' (expected 'text' or 'json')",
                    other
                ));
            }
        };

        Ok(Self {
            config_path,
            rule_set,
            rule_index,
            path,
            method,
            headers,
            status,
            json,
            text,
            file_path,
            delay_ms,
            dry_run,
            format,
            allow_outside,
        })
    }

    fn respond_payload(&self) -> RespondPayload {
        // RFC 065: `--json`/`--text` write to `respond.json`/
        // `respond.text` respectively, never merged — `--json` used to
        // write into `respond.text`, which is D1: the body was correct,
        // the server derived `text/plain` for it regardless, since
        // `respond.text` carries no memory of having come from `--json`.
        let mut payload = RespondPayload::default();
        payload.file_path = self.file_path.clone();
        payload.text = self.text.clone();
        payload.json = self.json.clone();
        payload.status = self.status;
        payload.delay_milliseconds = self.delay_ms;
        payload
    }

    fn respond_fields_given(&self) -> bool {
        self.status.is_some()
            || self.json.is_some()
            || self.text.is_some()
            || self.file_path.is_some()
            || self.delay_ms.is_some()
    }

    fn header_payloads(&self) -> Vec<HeaderConditionPayload> {
        self.headers
            .iter()
            .map(|(name, value)| {
                let mut condition = HeaderConditionPayload::new(name.clone(), HeaderOp::Equal);
                condition.value = Some(value.clone());
                condition
            })
            .collect()
    }

    /// REVIEW-001 § 3: a bare `set rule`, or one with only addressing
    /// flags and nothing that actually changes anything, must be a
    /// `usage` error — not a write. Checked before `run_inner` ever
    /// touches disk (including its own bootstrap step), so a rejected
    /// invocation leaves every file untouched, not just leaves the
    /// *rule* out.
    fn validate_requests_something(&self) -> Result<(), String> {
        if self.rule_index.is_some() {
            let changes_when = self.path.is_some() || self.method.is_some();
            let changes_respond = self.respond_fields_given();
            let adds_headers = !self.headers.is_empty();
            if !changes_when && !changes_respond && !adds_headers {
                return Err(
                    "nothing to change — give at least one of --path, --method, --status, \
                     --json/--text, --file, --delay or --header"
                        .to_owned(),
                );
            }
        } else if !self.respond_fields_given() {
            return Err(
                "nothing to respond with — give at least one of --status, --json/--text, \
                 --file or --delay"
                    .to_owned(),
            );
        }
        Ok(())
    }
}

// ── Entry point ───────────────────────────────────────────────────────

const USAGE: &str = "Usage: apimock set rule [-c <config>] [--rule-set <path>] [--rule <index>] \
 [--path <url_path>] [--method <METHOD>] [-H \"Name: value\"]... \
 [--status <code>] [--json <value>|--text <value>] [--file <path>] \
 [--delay <ms>] [--dry-run] [--format text|json] [--allow-outside]";

fn usage_error(message: &str) -> i32 {
    eprintln!("apimock set rule: {}", message);
    eprintln!("{}", USAGE);
    2
}

pub fn run(raw_args: &[String]) -> i32 {
    let Some(noun) = raw_args.first() else {
        return usage_error("missing required subcommand");
    };
    if noun != "rule" {
        return usage_error(&format!(
            "unknown target '{}' (only 'rule' is supported)",
            noun
        ));
    }
    let rest = &raw_args[1..];

    // Every check in this block runs before `run_inner` touches disk at
    // all — including its own bootstrap-if-missing step — so a
    // rejected invocation never writes anything, not even a starter
    // file (REVIEW-001 § 3).
    //
    // `strict_bare_tokens: true` (RFC 064 Amendment 1) preserves this
    // command's own pre-existing strictness — unlike `get`'s `<path>`,
    // `set rule` has no positional argument once its leading `rule`
    // noun is stripped above, so any leftover token that isn't a known
    // flag is a mistake, not a value, even if it doesn't start with
    // `-`. This is `set`'s one behavioural difference from the other
    // three commands sharing this function, preserved deliberately
    // rather than loosened by folding its former private copy in.
    if let Err(e) = reject_unknown_flags(
        rest,
        &known_flag_names(),
        NO_VALUE_FLAG_NAMES,
        true,
        "unrecognized argument",
    ) {
        return usage_error(&e);
    }
    let args = match SetRuleArgs::parse(rest) {
        Ok(a) => a,
        Err(e) => return usage_error(&e),
    };
    if let Err(e) = args.validate_requests_something() {
        return usage_error(&e);
    }

    let format = args.format.unwrap_or(Format::Text);
    let is_envelope = format == Format::Json;

    run_inner(&args, is_envelope)
}

/// RFC 062: the directory a caller-supplied write target resolves
/// into, for confinement purposes — canonicalised where the target
/// already exists, and canonicalised-*parent* where it doesn't, since
/// `set` legitimately creates rule-set files that don't exist yet
/// (bootstrapping) and a check that required existence would break
/// that. Errors are the caller's to map to an `ErrorKind` — a failure
/// here is almost always the parent directory not existing either,
/// which is a real (if different) problem, not silently "confined".
fn write_target_dir(path: &Path) -> std::io::Result<PathBuf> {
    if path.exists() {
        let canonical = path.canonicalize()?;
        Ok(if canonical.is_dir() {
            canonical
        } else {
            canonical
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(canonical)
        })
    } else {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parent.canonicalize()
    }
}

/// RFC 062 Option A: `target` is confined to `root`'s own directory
/// tree — `root` itself, or any descendant of it.
fn is_confined(target_dir: &Path, root_dir: &Path) -> bool {
    target_dir.starts_with(root_dir)
}

fn run_inner(args: &SetRuleArgs, is_envelope: bool) -> i32 {
    let config_path = args
        .config_path
        .clone()
        .unwrap_or_else(|| DEFAULT_CONFIG_FILE_PATH.to_owned());

    // RFC 062: confinement is checked *before anything is written* —
    // including the config-bootstrap write just below — so a
    // `--rule-set` that resolves outside the config's directory tree
    // never causes even a starter `apimock.toml` to be created. Derived
    // independently of `Workspace`/`ws.config()` on purpose: those
    // require a *loaded* config, and loading may itself require the
    // bootstrap write this check has to happen ahead of. The config's
    // own directory is just `config_path`'s parent, which needs no load
    // to compute.
    let rule_set_path = args
        .rule_set
        .clone()
        .unwrap_or_else(|| DEFAULT_RULE_SET_FILE_PATH.to_owned())
        .trim_start_matches("./")
        .to_owned();
    if !args.allow_outside {
        let config_dir = Path::new(&config_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let candidate = config_dir.join(&rule_set_path);
        match (write_target_dir(config_dir), write_target_dir(&candidate)) {
            (Ok(config_dir), Ok(target_dir)) if !is_confined(&target_dir, &config_dir) => {
                return fail(
                    is_envelope,
                    ErrorKind::Usage,
                    format!(
                        "--rule-set `{}` resolves outside the config directory; \
                         pass --allow-outside to permit this",
                        rule_set_path
                    ),
                );
            }
            (Err(e), _) | (_, Err(e)) => {
                return fail(
                    is_envelope,
                    ErrorKind::Io,
                    format!("failed to resolve --rule-set path: {}", e),
                );
            }
            _ => {}
        }
    }

    if !Path::new(&config_path).exists() {
        // REVIEW-001 § 4: `--dry-run` is a safety affordance — a
        // caller must be able to trust it never writes, full stop, not
        // "never writes except this one corner". Refusing here is the
        // whole fix: no in-memory `Workspace` can be built without a
        // real file to load (`Workspace::load` has no zero-config path
        // the way `Config::new(None, None)` does for `get`), so there
        // is nothing to preview without creating something first — and
        // creating something is exactly the write `--dry-run` promises
        // not to make.
        if args.dry_run {
            return fail(
                is_envelope,
                ErrorKind::Usage,
                format!(
                    "cannot preview with --dry-run: no config found at `{}` — run without \
                     --dry-run to create one first, or point --config at an existing file",
                    config_path
                ),
            );
        }
        if let Err(e) = std::fs::write(&config_path, MINIMAL_APIMOCK_TOML) {
            return fail(
                is_envelope,
                ErrorKind::Io,
                format!("failed to create `{}`: {}", config_path, e),
            );
        }
    }

    let mut ws = match Workspace::load(PathBuf::from(&config_path)) {
        Ok(ws) => ws,
        Err(e) => {
            return fail(
                is_envelope,
                envelope::kind_for_workspace_error(&e),
                format!("failed to load config: {}", e),
            );
        }
    };

    // `rule_set_path` (bare, no `./` prefix) was already computed above
    // for the confinement check — this is the same value that ends up
    // written into `service.rule_sets = [...]` (via `AddRuleSet`'s
    // `path` field), matching how every other producer of that array
    // writes it (`--init`'s wizard, a GUI's own `AddRuleSet`).
    // `cmd_add_rule_set` joins it against the config's directory itself;
    // joining it here too (below) with an already-`./`-prefixed string
    // would double up to `././...` — harmless to load, but needless
    // noise in every preview/diff row this command prints.
    let relative_dir = match ws.config().current_dir_to_parent_dir_relative_path() {
        Ok(d) => d,
        Err(e) => {
            return fail(
                is_envelope,
                ErrorKind::Internal,
                format!("failed to resolve config directory: {}", e),
            );
        }
    };
    let joined_rule_set_path = Path::new(&relative_dir).join(&rule_set_path);

    let existing_rs_idx = find_rule_set_index(&ws, &joined_rule_set_path);

    let (rs_idx, rule_set_node_id) = match existing_rs_idx {
        Some(idx) => {
            let id = ws
                .rule_set_id_at(idx)
                .expect("every loaded rule set has an id seeded at load()");
            (idx, id)
        }
        None => {
            if args.rule_index.is_some() {
                return fail(
                    is_envelope,
                    ErrorKind::Usage,
                    format!(
                        "no such rule set `{}` — it is not in service.rule_sets",
                        rule_set_path
                    ),
                );
            }
            if let Err(e) = bootstrap_rule_set_file_if_missing(
                &joined_rule_set_path,
                &rule_set_path,
                args.dry_run,
                is_envelope,
            ) {
                return e;
            }
            let idx = ws.config().service.rule_sets.len();
            match ws.apply(EditCommand::AddRuleSet {
                path: rule_set_path.clone(),
            }) {
                Ok(result) => {
                    let id = result
                        .changed_nodes
                        .first()
                        .copied()
                        .expect("AddRuleSet always reports the new rule set's own id first");
                    (idx, id)
                }
                Err(e) => {
                    return fail(
                        is_envelope,
                        envelope::kind_for_apply_error(&e),
                        e.to_string(),
                    );
                }
            }
        }
    };

    let apply_outcome = match args.rule_index {
        None => apply_add(&mut ws, rule_set_node_id, args),
        Some(rule_index) => apply_update(&mut ws, rs_idx, rule_index, args),
    };
    if let Err(e) = apply_outcome {
        return fail(is_envelope, e.0, e.1);
    }

    let args_view = ArgsView {
        rule_set: &rule_set_path,
        rule_index: args.rule_index,
    };

    if args.dry_run {
        let preview = ws.preview_changes();
        print_result(is_envelope, &args_view, &preview_json(&ws, &preview), true);
        return 0;
    }

    match ws.save() {
        Ok(save_result) => {
            print_result(
                is_envelope,
                &args_view,
                &save_result_json(&ws, &save_result),
                false,
            );
            0
        }
        Err(e) => fail(
            is_envelope,
            envelope::kind_for_save_error(&e),
            e.to_string(),
        ),
    }
}

/// `(rule set index, rule set node id)`'s rule-set half — an
/// `ApplyError`/message pair, threaded through `run_inner`'s early
/// returns so every failure path (usage, load, apply, save) reports
/// through the same envelope machinery.
type Failure = (ErrorKind, String);

fn apply_add(
    ws: &mut Workspace,
    parent: apimock_config::NodeId,
    args: &SetRuleArgs,
) -> Result<(), Failure> {
    let headers = args.header_payloads();
    let mut rule = RulePayload::default();
    rule.url_path = args.path.clone();
    rule.method = args.method.clone();
    rule.headers = if headers.is_empty() {
        None
    } else {
        Some(headers)
    };
    rule.respond = args.respond_payload();
    ws.apply(EditCommand::AddRule { parent, rule })
        .map(|_| ())
        .map_err(|e| (envelope::kind_for_apply_error(&e), e.to_string()))
}

fn apply_update(
    ws: &mut Workspace,
    rs_idx: usize,
    rule_index: usize,
    args: &SetRuleArgs,
) -> Result<(), Failure> {
    let Some(rule_id) = ws.rule_id_at(rs_idx, rule_index) else {
        return Err((
            ErrorKind::Usage,
            format!("no such rule #{} in this rule set", rule_index),
        ));
    };

    // `validate_requests_something` (run before `run_inner`, so before
    // any disk write) already guarantees at least one of these is true.
    let changes_when = args.path.is_some() || args.method.is_some();
    let changes_respond = args.respond_fields_given();

    if changes_when {
        // `RulePayload.respond` has no "None = preserve" semantic the
        // way `headers`/`body` do — `build_respond_from_payload` uses
        // exactly the fields it's given, full stop. So an update that
        // only meant to change `when` (no respond flags given) would
        // otherwise wipe the rule's existing respond to all-`None`,
        // which then fails `Respond::validate()` the same way a bare
        // `set rule` used to (REVIEW-001 § 3's failure mode, reachable
        // here too, just via a different flag combination). Read the
        // rule's *current* respond and carry it forward unless the
        // caller actually asked to change it.
        let respond = if changes_respond {
            args.respond_payload()
        } else {
            let current = &ws.config().service.rule_sets[rs_idx].rules[rule_index].respond;
            let mut payload = RespondPayload::default();
            payload.file_path = current.file_path.clone();
            payload.text = current.text.clone();
            payload.json = current.json.clone();
            payload.status = current.status;
            payload.delay_milliseconds = current.delay_response_milliseconds;
            payload
        };
        let mut rule = RulePayload::default();
        rule.url_path = args.path.clone();
        rule.method = args.method.clone();
        // headers: left None — preserve; AddHeaderCondition below layers on top, doesn't replace
        rule.respond = respond;
        ws.apply(EditCommand::UpdateRule { id: rule_id, rule })
            .map_err(|e| (envelope::kind_for_apply_error(&e), e.to_string()))?;
    } else if changes_respond {
        let Some(respond_id) = ws.respond_id_at(rs_idx, rule_index) else {
            return Err((
                ErrorKind::Internal,
                "rule exists but its respond block has no id".to_owned(),
            ));
        };
        ws.apply(EditCommand::UpdateRespond {
            id: respond_id,
            respond: args.respond_payload(),
        })
        .map_err(|e| (envelope::kind_for_apply_error(&e), e.to_string()))?;
    }

    for condition in args.header_payloads() {
        ws.apply(EditCommand::AddHeaderCondition { rule_id, condition })
            .map_err(|e| (envelope::kind_for_apply_error(&e), e.to_string()))?;
    }

    Ok(())
}

fn find_rule_set_index(ws: &Workspace, joined_path: &Path) -> Option<usize> {
    let target = std::fs::canonicalize(joined_path).ok()?;
    ws.config()
        .service
        .rule_sets
        .iter()
        .position(|rs| std::fs::canonicalize(rs.file_path.as_str()).ok().as_ref() == Some(&target))
}

fn bootstrap_rule_set_file_if_missing(
    joined_path: &Path,
    display_path: &str,
    dry_run: bool,
    is_envelope: bool,
) -> Result<(), i32> {
    if joined_path.exists() {
        return Ok(());
    }
    // Same promise as the config-path check above: `--dry-run` never
    // writes, not even a bootstrap skeleton.
    if dry_run {
        return Err(fail(
            is_envelope,
            ErrorKind::Usage,
            format!(
                "cannot preview with --dry-run: rule set `{}` does not exist yet — run without \
                 --dry-run to create it first",
                display_path
            ),
        ));
    }
    if let Err(e) = std::fs::write(joined_path, EMPTY_RULE_SET_TOML) {
        return Err(fail(
            is_envelope,
            ErrorKind::Io,
            format!("failed to create `{}`: {}", joined_path.display(), e),
        ));
    }
    Ok(())
}

// ── Output ───────────────────────────────────────────────────────────

struct ArgsView<'a> {
    rule_set: &'a str,
    rule_index: Option<usize>,
}

fn preview_json(ws: &Workspace, preview: &[apimock_config::DiffItem]) -> serde_json::Value {
    serde_json::json!({
        "would_change": preview.iter().map(|item| serde_json::json!({
            "kind": serde_json::to_value(item.kind).unwrap_or_default(),
            "target": ws.describe(item.target).unwrap_or_else(|| "(unknown)".to_owned()),
            "summary": item.summary,
        })).collect::<Vec<_>>(),
    })
}

fn save_result_json(ws: &Workspace, save: &apimock_config::SaveResult) -> serde_json::Value {
    serde_json::json!({
        "changed_files": save.changed_files.iter().map(|p| p.to_string_lossy().into_owned()).collect::<Vec<_>>(),
        "changes": save.diff_summary.iter().map(|item| serde_json::json!({
            "kind": serde_json::to_value(item.kind).unwrap_or_default(),
            "target": ws.describe(item.target).unwrap_or_else(|| "(unknown)".to_owned()),
            "summary": item.summary,
        })).collect::<Vec<_>>(),
        "requires_reload": save.requires_reload,
    })
}

fn print_result(is_envelope: bool, args_view: &ArgsView, body: &serde_json::Value, dry_run: bool) {
    if is_envelope {
        let mut result = body.clone();
        result["dry_run"] = serde_json::json!(dry_run);
        result["rule_set"] = serde_json::json!(args_view.rule_set);
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope::ok(result)).unwrap_or_default()
        );
        return;
    }

    if dry_run {
        println!("Would apply (--dry-run, nothing written):");
    } else {
        println!("Applied:");
    }
    if let Some(idx) = args_view.rule_index {
        println!("  rule set: {}, rule #{}", args_view.rule_set, idx);
    } else {
        println!("  rule set: {} (new rule)", args_view.rule_set);
    }
    let changes = body
        .get("would_change")
        .or_else(|| body.get("changes"))
        .and_then(|v| v.as_array());
    if let Some(changes) = changes {
        for c in changes {
            println!(
                "  {}: {} — {}",
                c["kind"].as_str().unwrap_or("?"),
                c["target"].as_str().unwrap_or("?"),
                c["summary"].as_str().unwrap_or("")
            );
        }
    }
}

fn fail(is_envelope: bool, kind: ErrorKind, message: String) -> i32 {
    // 2 for a bad invocation / unloadable config, 1 for everything that
    // happens after a successful load+address resolution (RFC 049's
    // exit-code convention: 0/2/1).
    let exit_code = if matches!(
        kind,
        ErrorKind::Usage | ErrorKind::ConfigInvalid | ErrorKind::ConfigUnreadable
    ) {
        2
    } else {
        1
    };
    if is_envelope {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope::err(kind, message)).unwrap_or_default()
        );
    } else {
        eprintln!("apimock set rule: {}", message);
    }
    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_rule_add() {
        let args: Vec<String> = ["--path", "/x", "--status", "200"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let a = SetRuleArgs::parse(&args).unwrap();
        assert_eq!(a.path.as_deref(), Some("/x"));
        assert_eq!(a.status, Some(200));
        assert!(a.rule_index.is_none());
    }

    #[test]
    fn parse_rejects_bad_json() {
        let args: Vec<String> = ["--json", "{not json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(SetRuleArgs::parse(&args).is_err());
    }

    #[test]
    fn parse_rejects_json_and_text_together() {
        let args: Vec<String> = ["--json", "{}", "--text", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(SetRuleArgs::parse(&args).is_err());
    }

    #[test]
    fn parse_rejects_non_numeric_rule_index() {
        let args: Vec<String> = ["--rule", "not-a-number"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(SetRuleArgs::parse(&args).is_err());
    }

    #[test]
    fn header_payloads_use_equal_op() {
        let args: Vec<String> = ["--header", "X-Api-Key: shh"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let a = SetRuleArgs::parse(&args).unwrap();
        let payloads = a.header_payloads();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].name, "x-api-key");
        assert_eq!(payloads[0].value.as_deref(), Some("shh"));
        assert!(matches!(payloads[0].op, HeaderOp::Equal));
    }
}
