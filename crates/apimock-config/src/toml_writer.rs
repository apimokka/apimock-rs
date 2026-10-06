//! TOML rendering for the editable subset of `Config` and `RuleSet`,
//! plus in-place mutation of a previously-loaded document (RFC 056).
//!
//! # Why hand-rolled instead of `serde::Serialize`
//!
//! The runtime model stored in `Config` carries a number of fields
//! that exist only for matching speed (cached `StatusCode`, normalized
//! `UrlPath` with prefix applied, `dir_prefix` derived from `Prefix`,
//! etc.). A blanket `#[derive(Serialize)]` on every type would have to
//! mark each of those `#[serde(skip)]`, *and* the routing crate types
//! aren't `Serialize` today. Building `toml::Value` trees by hand is
//! both shorter and inherently selective: the writer only emits
//! editable-on-purpose fields.
//!
//! # Two ways to turn that tree into text
//!
//! `render_apimock_toml`/`render_rule_set_toml` serialise the tree
//! fresh with `toml::to_string_pretty` — sorted keys, no comments,
//! canonical quoting. `workspace.rs` uses this for the rendered
//! baseline (RFC 056 §2 Q1: kept, deliberately, so
//! `has_unsaved_changes` keeps comparing apples to apples), and
//! `diff.rs` uses it to diff *models*, which turn out not to care
//! about trivia either (RFC 056 §2 Q2: established from source — no
//! change needed there).
//!
//! `apply_in_place` instead mutates a `toml_edit::DocumentMut` parsed
//! from the file's own previous text, so comments, blank lines and key
//! order survive a save that only changed a few values. This is what
//! `workspace/save.rs` writes to disk. Building a fresh `toml_edit`
//! document from the model and serialising it would preserve nothing —
//! that would be today's old behaviour with a new dependency, so this
//! module never does that.
//!
//! Previously this module always rendered fresh, and the module doc
//! here (and `workspace/save.rs`'s) claimed `Workspace::save` carried
//! an `Info` diagnostic warning that comments and key order were lost.
//! That diagnostic was never actually wired into `SaveResult` — no
//! `Severity::Info` is constructed anywhere in this crate — so nothing
//! needed removing beyond that stale claim.

use apimock_routing::{
    Respond, RuleSet,
    rule_set::rule::{
        Rule,
        when::{
            When,
            request::{
                Request,
                body::{BodyConditionStatement, body_kind::BodyKind},
                headers::HeaderConditionStatement,
                http_method::HttpMethod,
                url_path::UrlPathConfig,
            },
        },
    },
};
use toml::{Value, value::Table};

use crate::{Config, ListenerConfig, ServiceConfig, config::log_config::LogConfig};

/// Render the root `apimock.toml` to TOML text.
pub fn render_apimock_toml(config: &Config) -> String {
    render_table_pretty(&root_table(config))
}

/// Build the editable-subset `Table` for the root `apimock.toml`.
///
/// Split out from `render_apimock_toml` so `workspace/save.rs` can use
/// the same tree as the *target* for `apply_in_place` without a second
/// hand-written builder to keep in sync with this one.
pub(crate) fn root_table(config: &Config) -> Table {
    let mut root = Table::new();

    if let Some(listener) = config.listener.as_ref() {
        root.insert(
            "listener".to_owned(),
            Value::Table(listener_table(listener)),
        );
    }
    if let Some(log) = config.log.as_ref()
        && let Some(t) = log_table(log)
    {
        root.insert("log".to_owned(), Value::Table(t));
    }
    root.insert(
        "service".to_owned(),
        Value::Table(service_table(&config.service)),
    );

    if let Some(ftv) = config.file_tree_view.as_ref()
        && let Some(t) = file_tree_view_table(ftv)
    {
        root.insert("file_tree_view".to_owned(), Value::Table(t));
    }

    root
}

/// Render one rule-set TOML to text.
pub fn render_rule_set_toml(rule_set: &RuleSet) -> String {
    render_table_pretty(&rule_set_table(rule_set))
}

/// Build the editable-subset `Table` for one rule-set file. See
/// `root_table` for why this is split out.
pub(crate) fn rule_set_table(rule_set: &RuleSet) -> Table {
    let mut root = Table::new();

    if let Some(prefix) = rule_set.prefix.as_ref() {
        let mut p = Table::new();
        if let Some(url) = prefix.url_path_prefix.as_ref() {
            p.insert("url_path".to_owned(), Value::String(url.clone()));
        }
        if let Some(dir) = prefix.respond_dir_prefix.as_ref() {
            p.insert("respond_dir".to_owned(), Value::String(dir.clone()));
        }
        if !p.is_empty() {
            root.insert("prefix".to_owned(), Value::Table(p));
        }
    }

    // RFC 025: per-rule-set strategy override.
    if let Some(strategy) = rule_set.strategy.as_ref() {
        root.insert("strategy".to_owned(), Value::String(strategy.to_string()));
    }

    if !rule_set.rules.is_empty() {
        let rules: Vec<Value> = rule_set
            .rules
            .iter()
            .map(|r| Value::Table(rule_table(r)))
            .collect();
        root.insert("rules".to_owned(), Value::Array(rules));
    }

    root
}

fn render_table_pretty(root: &Table) -> String {
    toml::to_string_pretty(&Value::Table(root.clone()))
        .unwrap_or_else(|err| format!("# failed to render: {}\n", err))
}

// -------------------------------------------------------------------
// Internal helpers — one function per editable struct in the config.
// Each returns a `toml::Table` rather than a `Value` so callers can
// decide whether to skip empty tables.
// -------------------------------------------------------------------

fn listener_table(l: &ListenerConfig) -> Table {
    let mut t = Table::new();
    t.insert("ip_address".to_owned(), Value::String(l.ip_address.clone()));
    t.insert("port".to_owned(), Value::Integer(i64::from(l.port)));
    if let Some(tls) = l.tls.as_ref() {
        let mut tt = Table::new();
        tt.insert("cert".to_owned(), Value::String(tls.cert.clone()));
        tt.insert("key".to_owned(), Value::String(tls.key.clone()));
        if let Some(p) = tls.port {
            tt.insert("port".to_owned(), Value::Integer(i64::from(p)));
        }
        t.insert("tls".to_owned(), Value::Table(tt));
    }
    t
}

fn log_table(l: &LogConfig) -> Option<Table> {
    let mut t = Table::new();
    let v = &l.verbose;
    let mut verbose = Table::new();
    verbose.insert("header".to_owned(), Value::Boolean(v.header));
    verbose.insert("body".to_owned(), Value::Boolean(v.body));
    t.insert("verbose".to_owned(), Value::Table(verbose));
    Some(t)
}

/// Render `[file_tree_view]` section. Returns `None` when the config is
/// entirely default (omitting the section keeps the file clean).
fn file_tree_view_table(c: &crate::config::file_tree_config::FileTreeViewConfig) -> Option<Table> {
    // Only emit the section when at least one field differs from the default.
    let is_default = !c.show_hidden
        && c.builtin_excludes
        && c.extra_excludes.is_empty()
        && c.include.is_empty()
        && !c.respect_gitignore;
    if is_default {
        return None;
    }

    let mut t = Table::new();
    if c.show_hidden {
        t.insert("show_hidden".to_owned(), Value::Boolean(true));
    }
    if !c.builtin_excludes {
        t.insert("builtin_excludes".to_owned(), Value::Boolean(false));
    }
    if !c.extra_excludes.is_empty() {
        let arr = toml::value::Array::from_iter(
            c.extra_excludes.iter().map(|s| Value::String(s.clone())),
        );
        t.insert("extra_excludes".to_owned(), Value::Array(arr));
    }
    if !c.include.is_empty() {
        let arr = toml::value::Array::from_iter(c.include.iter().map(|s| Value::String(s.clone())));
        t.insert("include".to_owned(), Value::Array(arr));
    }
    if c.respect_gitignore {
        t.insert("respect_gitignore".to_owned(), Value::Boolean(true));
    }
    Some(t)
}

fn service_table(s: &ServiceConfig) -> Table {
    let mut t = Table::new();
    if let Some(strategy) = s.strategy.as_ref() {
        t.insert(
            "strategy".to_owned(),
            Value::String(format!("{}", strategy)),
        );
    }
    if let Some(paths) = s.rule_sets_file_paths.as_ref() {
        let arr: Vec<Value> = paths.iter().map(|p| Value::String(p.clone())).collect();
        t.insert("rule_sets".to_owned(), Value::Array(arr));
    }
    if let Some(paths) = s.middlewares_file_paths.as_ref() {
        let arr: Vec<Value> = paths.iter().map(|p| Value::String(p.clone())).collect();
        t.insert("middlewares".to_owned(), Value::Array(arr));
    }
    t.insert(
        "fallback_respond_dir".to_owned(),
        Value::String(s.fallback_respond_dir.clone()),
    );
    t
}

pub(crate) fn rule_table(r: &Rule) -> Table {
    let mut t = Table::new();
    if let Some(p) = r.priority {
        t.insert("priority".to_owned(), Value::Integer(i64::from(p)));
    }
    // Task 019 R-01: inside `[[rules]]` the writer manages every field the
    // model holds. A rule's identity is ambiguous across an edit (`set`
    // addresses it by index, the writer pairs rows by content), so a value
    // carried along by that pairing can land on the wrong rule. A value
    // emitted from the model cannot.
    if let Some(w) = r.weight {
        t.insert("weight".to_owned(), Value::Integer(i64::from(w)));
    }
    t.insert("when".to_owned(), Value::Table(when_table(&r.when)));
    t.insert(
        "respond".to_owned(),
        Value::Table(respond_table(&r.respond)),
    );
    t
}

fn when_table(w: &When) -> Table {
    let mut t = Table::new();
    t.insert(
        "request".to_owned(),
        Value::Table(request_table(&w.request)),
    );
    t
}

fn request_table(req: &Request) -> Table {
    let mut t = Table::new();

    if let Some(url_path_config) = req.url_path_config.as_ref() {
        match url_path_config {
            UrlPathConfig::Simple(s) => {
                t.insert("url_path".to_owned(), Value::String(s.clone()));
            }
            UrlPathConfig::Detailed(detail) => {
                let mut dt = Table::new();
                dt.insert("value".to_owned(), Value::String(detail.value.clone()));
                if let Some(op) = detail.op.as_ref() {
                    // RuleOp's `Display` impl produces a human-readable
                    // form (`" == "`, `" starts with "`) for log output.
                    // The TOML representation needs the snake_case
                    // serde tag (`equal`, `starts_with`). Use the
                    // routing crate's `op_name` helper so the round-
                    // trip is faithful.
                    dt.insert(
                        "op".to_owned(),
                        Value::String(apimock_routing::view::build::op_name(op)),
                    );
                }
                t.insert("url_path".to_owned(), Value::Table(dt));
            }
        }
    }

    if let Some(method) = req.http_method.as_ref() {
        t.insert("method".to_owned(), Value::String(http_method_name(method)));
    }

    // Headers conditions. The routing crate exposes `Headers` as a
    // newtype `pub struct Headers(pub HashMap<String, ConditionStatement>)`,
    // so we walk that map and emit one TOML sub-table per condition
    // statement: `[when.request.headers.<key>] op = "...", value = "..."`.
    if let Some(headers) = req.headers.as_ref() {
        let mut headers_table = Table::new();
        // Sort by key for determinism — TOML's `HashMap` deserialize
        // doesn't preserve order, so the round-trip text won't either,
        // but explicit sorting at write time means a save → save
        // sequence produces byte-identical output.
        let mut keys: Vec<&String> = headers.0.keys().collect();
        keys.sort();
        for key in keys {
            let stmt = &headers.0[key];
            headers_table.insert(
                key.clone(),
                Value::Table(header_condition_statement_table(stmt)),
            );
        }
        if !headers_table.is_empty() {
            t.insert("headers".to_owned(), Value::Table(headers_table));
        }
    }

    // Body conditions. `Body` is keyed first by `BodyKind` (currently
    // only `Json`) and then by a dotted-path string identifying the
    // value inside the JSON body to compare. The TOML form is
    // `[when.request.body.json."<dotted.path>"] op = "...", value = "..."`,
    // for example
    // `[when.request.body.json."order.items.0.product_id"] value = "X"`.
    //
    // Note: this is the routing crate's mini-syntax (object keys
    // joined by `.`, with numeric segments addressing array indices),
    // not canonical JSONPath. See `apimock_routing::util::json` for
    // the supported shapes.
    if let Some(body) = req.body.as_ref() {
        let mut body_table = Table::new();
        let mut kinds: Vec<&BodyKind> = body.0.keys().collect();
        kinds.sort_by_key(|k| body_kind_key(k));
        for kind in kinds {
            let kind_str = body_kind_key(kind);
            let inner = &body.0[kind];
            let mut kind_table = Table::new();
            let mut keys: Vec<&String> = inner.keys().collect();
            keys.sort();
            for key in keys {
                let stmt = &inner[key];
                kind_table.insert(
                    key.clone(),
                    Value::Table(body_condition_statement_table(stmt)),
                );
            }
            if !kind_table.is_empty() {
                body_table.insert(kind_str.to_owned(), Value::Table(kind_table));
            }
        }
        if !body_table.is_empty() {
            t.insert("body".to_owned(), Value::Table(body_table));
        }
    }

    t
}

/// Render a `HeaderConditionStatement` (used by Headers entries) as a TOML
/// table with `op` (optional) and `value` keys.
///
/// Presence operators (`exists`, `absent`) omit the `value` key — it is
/// meaningless for them. Value operators always emit `value`.
fn header_condition_statement_table(stmt: &HeaderConditionStatement) -> Table {
    use apimock_routing::rule_set::rule::when::request::headers::header_operator::HeaderOperator;
    let mut t = Table::new();
    if let Some(op) = stmt.op.as_ref() {
        t.insert("op".to_owned(), Value::String(op.as_str().to_owned()));
        match op {
            HeaderOperator::Exists | HeaderOperator::Absent => {
                // No value key for presence operators.
            }
            _ => {
                t.insert("value".to_owned(), Value::String(stmt.value.clone()));
            }
        }
    } else {
        t.insert("value".to_owned(), Value::String(stmt.value.clone()));
    }
    t
}

/// Render a `BodyConditionStatement` (used by Body entries) as a TOML
/// table with `op` (optional) and `value` keys.
fn body_condition_statement_table(stmt: &BodyConditionStatement) -> Table {
    use apimock_routing::view::build::body_op_name_pub;
    let mut t = Table::new();
    if let Some(op) = stmt.op.as_ref() {
        t.insert("op".to_owned(), Value::String(body_op_name_pub(op)));
    }
    t.insert("value".to_owned(), Value::String(stmt.value.clone()));
    t
}

/// Snake-case TOML key for a `BodyKind` variant. Matches the
/// `serde(rename_all = "snake_case")` tag the routing crate uses
/// when deserialising — guarantees the round-trip works.
fn body_kind_key(kind: &BodyKind) -> &'static str {
    match kind {
        BodyKind::Json => "json",
    }
}

/// Serialize an HTTP method back to its TOML form. Inverse of
/// `HttpMethod::parse_config_token` (RFC 082): `as_str` yields exactly the
/// spelling that parse accepts.
fn http_method_name(m: &HttpMethod) -> String {
    m.as_str().to_owned()
}

fn respond_table(r: &Respond) -> Table {
    let mut t = Table::new();
    if let Some(p) = r.file_path.as_ref() {
        t.insert("file_path".to_owned(), Value::String(p.clone()));
    }
    if let Some(k) = r.csv_records_key.as_ref() {
        t.insert("csv_records_key".to_owned(), Value::String(k.clone()));
    }
    if let Some(text) = r.text.as_ref() {
        t.insert("text".to_owned(), Value::String(text.clone()));
    }
    if let Some(json) = r.json.as_ref() {
        t.insert("json".to_owned(), Value::String(json.clone()));
    }
    if let Some(s) = r.status.as_ref() {
        t.insert("status".to_owned(), Value::Integer(i64::from(*s)));
    }
    if let Some(headers) = r.headers.as_ref() {
        let mut ht = Table::new();
        for (k, v) in headers.iter() {
            match v {
                Some(val) => ht.insert(k.clone(), Value::String(val.clone())),
                None => ht.insert(k.clone(), Value::Boolean(false)),
            };
        }
        t.insert("headers".to_owned(), Value::Table(ht));
    }
    if let Some(d) = r.delay_response_milliseconds.as_ref() {
        t.insert(
            "delay_response_milliseconds".to_owned(),
            Value::Integer(i64::from(*d)),
        );
    }
    t
}

// -------------------------------------------------------------------
// Ownership (task 019) — which keys the writer may delete.
// -------------------------------------------------------------------

/// What the writer manages at one level of a config document.
///
/// # Why this exists
///
/// `apply_in_place` used to remove every key of the file that the
/// editable-subset target did not contain. The target is built from what
/// the writer *models*, so every key the schema gained after the writer
/// was written (`[listener.tls] max_connections`, `[service]
/// max_request_body_bytes`, a rule set's `[default]`) was silently deleted by the first `set`, and the file still validated.
///
/// The rule is now: **the writer may only delete or rewrite keys it
/// manages.** A key it does not list here is never touched, whatever else
/// is in the file, so a key added to the schema tomorrow is safe without
/// anyone remembering to teach the writer about it.
///
/// # Why a list per level and not "keys the model had a value for"
///
/// The writer must still remove a managed key the user cleared (a rule's
/// `priority`, a `[listener.tls] port`). That key is absent from the
/// target precisely because it has no value, so "absent from the target"
/// cannot tell *cleared* from *never modelled*. Only a list can: a key on
/// it that is absent from the target was cleared; a key off it was never
/// the writer's. `tests::the_ownership_tree_is_exactly_what_the_writer_emits`
/// keeps the lists and the emitters in step in both directions.
pub(crate) struct Owned {
    /// Plain keys at this level the writer manages.
    keys: &'static [&'static str],
    /// Nested tables (or arrays of tables) the writer manages, each with
    /// its own ownership below.
    children: &'static [(&'static str, &'static Owned)],
    /// When `Some`, every key at this level is a name the user chose (a
    /// header name, a JSON path) and is managed; the value is the level
    /// below it.
    each: Option<&'static Owned>,
}

impl Owned {
    fn owns(&self, key: &str) -> bool {
        self.each.is_some()
            || self.keys.contains(&key)
            || self.children.iter().any(|(name, _)| *name == key)
    }

    /// Ownership of what lives under `key`. A key the tree does not know
    /// owns nothing, which is the safe direction: preserve.
    fn below(&self, key: &str) -> &'static Owned {
        self.each
            .or_else(|| {
                self.children
                    .iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, owned)| *owned)
            })
            .unwrap_or(&EMPTY)
    }

    /// Every key path this tree manages, `*` standing for a user-chosen
    /// name. For the sync test.
    #[cfg(test)]
    fn paths(&self, prefix: &str, out: &mut Vec<String>) {
        let join = |key: &str| {
            if prefix.is_empty() {
                key.to_owned()
            } else {
                format!("{prefix}.{key}")
            }
        };
        if let Some(each) = self.each {
            out.push(join("*"));
            each.paths(&join("*"), out);
            return;
        }
        for key in self.keys {
            out.push(join(key));
        }
        for (key, below) in self.children {
            out.push(join(key));
            below.paths(&join(key), out);
        }
    }
}

static EMPTY: Owned = Owned {
    keys: &[],
    children: &[],
    each: None,
};

/// `{ op, value }`: a header condition, a body condition, and the table
/// form of `url_path`.
static CONDITION: Owned = Owned {
    keys: &["op", "value"],
    children: &[],
    each: None,
};
static HEADER_CONDITIONS: Owned = Owned {
    keys: &[],
    children: &[],
    each: Some(&CONDITION),
};
static BODY_JSON: Owned = Owned {
    keys: &[],
    children: &[],
    each: Some(&CONDITION),
};
static BODY: Owned = Owned {
    keys: &[],
    children: &[("json", &BODY_JSON)],
    each: None,
};
static REQUEST: Owned = Owned {
    keys: &["method"],
    children: &[
        ("url_path", &CONDITION),
        ("headers", &HEADER_CONDITIONS),
        ("body", &BODY),
    ],
    each: None,
};
static WHEN: Owned = Owned {
    keys: &[],
    children: &[("request", &REQUEST)],
    each: None,
};
static RESPOND_HEADERS: Owned = Owned {
    keys: &[],
    children: &[],
    each: Some(&EMPTY),
};
static RESPOND: Owned = Owned {
    keys: &[
        "file_path",
        "csv_records_key",
        "text",
        "json",
        "status",
        "delay_response_milliseconds",
    ],
    children: &[("headers", &RESPOND_HEADERS)],
    each: None,
};
/// A rule. Every field the model holds is managed here (R-01): rows are
/// paired by content, which cannot be exact, so nothing may ride along on
/// that pairing except comments.
static RULE: Owned = Owned {
    keys: &["priority", "weight"],
    children: &[("when", &WHEN), ("respond", &RESPOND)],
    each: None,
};
static PREFIX: Owned = Owned {
    keys: &["url_path", "respond_dir"],
    children: &[],
    each: None,
};
/// A rule-set file. `[default]` and `[guard]` are deliberately absent.
static RULE_SET: Owned = Owned {
    keys: &["strategy"],
    children: &[("prefix", &PREFIX), ("rules", &RULE)],
    each: None,
};
/// `[listener.tls]`. `handshake_timeout_seconds` and `max_connections`
/// are deliberately absent.
static TLS: Owned = Owned {
    keys: &["cert", "key", "port"],
    children: &[],
    each: None,
};
static LISTENER: Owned = Owned {
    keys: &["ip_address", "port"],
    children: &[("tls", &TLS)],
    each: None,
};
static VERBOSE: Owned = Owned {
    keys: &["header", "body"],
    children: &[],
    each: None,
};
static LOG: Owned = Owned {
    keys: &[],
    children: &[("verbose", &VERBOSE)],
    each: None,
};
/// `[service]`. `cors_allow_credentials_origins`,
/// `max_request_body_bytes` and `middleware_max_operations` are
/// deliberately absent.
static SERVICE: Owned = Owned {
    keys: &[
        "strategy",
        "rule_sets",
        "middlewares",
        "fallback_respond_dir",
    ],
    children: &[],
    each: None,
};
static FILE_TREE_VIEW: Owned = Owned {
    keys: &[
        "show_hidden",
        "builtin_excludes",
        "extra_excludes",
        "include",
        "respect_gitignore",
    ],
    children: &[],
    each: None,
};
static ROOT: Owned = Owned {
    keys: &[],
    children: &[
        ("listener", &LISTENER),
        ("log", &LOG),
        ("service", &SERVICE),
        ("file_tree_view", &FILE_TREE_VIEW),
    ],
    each: None,
};

/// Ownership of the root `apimock.toml`.
pub(crate) fn root_owned() -> &'static Owned {
    &ROOT
}

/// Ownership of one rule-set file.
pub(crate) fn rule_set_owned() -> &'static Owned {
    &RULE_SET
}

// -------------------------------------------------------------------
// In-place mutation (RFC 056) — reconcile a parsed `toml_edit`
// document against the editable-subset `Table` above. This mutates
// the document rather than rebuilding one: existing keys keep their
// comments, blank lines and position; only a changed key's *value* is
// replaced, and only when it actually differs in meaning.
// -------------------------------------------------------------------

/// Apply `target` onto `original`'s previous text, touching only what
/// `owned` says the writer manages. Everything else — keys the writer
/// has never heard of, with their values, comments and position — comes
/// out byte-for-byte as it went in. Returns the file's new text.
///
/// # Errors
///
/// Only if `original` fails to re-parse as TOML. In practice this
/// path is unreachable in normal operation: `workspace/save.rs` only
/// calls this after confirming the on-disk text is still exactly
/// `original` (RFC 056 §2 Q3's conflict check), and `original` was
/// itself accepted by `Config::new`'s `toml`-crate parser at load
/// time — `toml` and `toml_edit` are the same project's siblings
/// targeting the same TOML spec version, so what one accepts the
/// other does too. Kept as a `Result` rather than an `unwrap` because
/// that reasoning lives in prose, not in the type system.
pub(crate) fn apply_in_place(
    original: &str,
    target: &Table,
    owned: &Owned,
) -> Result<String, toml_edit::TomlError> {
    let mut doc: toml_edit::DocumentMut = original.parse()?;
    reconcile_table(doc.as_table_mut(), target, owned);
    Ok(restore_line_endings(original, doc.to_string()))
}

/// Give `text` the line endings `original` had.
///
/// `toml_edit` turns every `\r\n` into `\n` when it parses, even on a
/// round trip that changes nothing, so without this a save would rewrite
/// every line of a CRLF file (the usual case on Windows, and under git's
/// `autocrlf`), including all the lines the writer never touched. The
/// ending that was used more often wins, so a file edited by hand into a
/// mix comes out in whichever it mostly was.
fn restore_line_endings(original: &str, text: String) -> String {
    let crlf = original.matches("\r\n").count();
    let bare_lf = original.matches('\n').count() - crlf;
    if crlf > bare_lf {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

/// Reconcile one table — standard or inline — against the corresponding
/// `target` table: remove the keys *the writer manages* that `target` no
/// longer has, recurse into nested tables, match `[[rules]]`-style
/// arrays-of-tables row by row, and otherwise overwrite a leaf only when
/// its meaning changed.
///
/// `dyn TableLike` is what lets one routine serve `[table]` and
/// `key = { inline = "table" }` alike. Before this, an inline table was
/// not recognised as a table, so it was rebuilt from the target and every
/// key in it the writer did not model went with it.
fn reconcile_table(doc: &mut dyn toml_edit::TableLike, target: &Table, owned: &Owned) {
    let stale: Vec<String> = doc
        .iter()
        .map(|(k, _)| k.to_owned())
        .filter(|k| owned.owns(k) && !target.contains_key(k.as_str()))
        .collect();
    for key in &stale {
        doc.remove(key);
    }

    for (key, value) in target.iter() {
        let below = owned.below(key);
        match value {
            Value::Table(sub) => reconcile_subtable(doc, key, sub, below),
            Value::Array(items) if is_table_array(items) => {
                reconcile_array_of_tables(doc, key, items, below)
            }
            leaf => set_scalar(doc, key, edit_value_from_leaf(leaf)),
        }
    }
}

fn reconcile_subtable(
    doc: &mut dyn toml_edit::TableLike,
    key: &str,
    target: &Table,
    owned: &Owned,
) {
    if let Some(item) = doc.get_mut(key)
        && let Some(existing) = item.as_table_like_mut()
    {
        reconcile_table(existing, target, owned);
        return;
    }
    let mut fresh = toml_edit::Table::new();
    fill_table(&mut fresh, target);
    replace_item(doc, key, toml_edit::Item::Table(fresh));
}

/// Put `item` at `key`, keeping the key's own decor (the comment lines
/// above it) when the key already exists. `TableLike::insert` on an
/// existing key replaces the key too, and takes that comment with it.
fn replace_item(doc: &mut dyn toml_edit::TableLike, key: &str, item: toml_edit::Item) {
    if let Some(slot) = doc.get_mut(key) {
        *slot = item;
    } else {
        doc.insert(key, item);
    }
}

/// Reconcile a `[[rules]]`-style array of tables.
///
/// Rows are matched to their previous selves by what they say, not by
/// index (see [`match_rows`]). That decides one thing: which rule a
/// *comment* stays with when a rule is deleted or moved. It carries no
/// values. Inside a rule the writer manages every field the model holds
/// (R-01), because the match cannot be exact: two rules that briefly look
/// alike tie, and a value carried across a tie can land on the wrong rule.
/// A tie can put a comment on the wrong rule, which is cosmetic.
fn reconcile_array_of_tables(
    doc: &mut dyn toml_edit::TableLike,
    key: &str,
    target_rows: &[Value],
    owned: &Owned,
) {
    let rows: Vec<&Table> = target_rows
        .iter()
        .map(|row| {
            row.as_table()
                .expect("is_table_array guarantees every row is a Value::Table")
        })
        .collect();
    let existing: Vec<toml_edit::Table> = doc.get(key).map(existing_rows).unwrap_or_default();
    let pairing = match_rows(&existing, &rows, owned);

    let mut out = toml_edit::ArrayOfTables::new();
    for (row, paired) in rows.iter().zip(&pairing) {
        match paired {
            Some(index) => {
                let mut kept = existing[*index].clone();
                reconcile_table(&mut kept, row, owned);
                out.push(kept);
            }
            None => {
                let mut fresh = toml_edit::Table::new();
                fill_table(&mut fresh, row);
                out.push(fresh);
            }
        }
    }

    // `toml_edit` prints tables in `position` order, and a row we kept
    // carries the position it had in the file. If the rows no longer run
    // in file order (a move), those positions would print the rows, and
    // their `[rules.respond]` sub-tables, back in the old order or
    // interleaved. Clearing them makes each table follow the one before
    // it, which is the order of `out`.
    let matched: Vec<usize> = pairing.iter().flatten().copied().collect();
    if matched.windows(2).any(|pair| pair[0] >= pair[1]) {
        for row in out.iter_mut() {
            clear_positions(row);
        }
    }

    replace_item(doc, key, toml_edit::Item::ArrayOfTables(out));
}

fn clear_positions(table: &mut toml_edit::Table) {
    table.set_position(None);
    for (_, item) in table.iter_mut() {
        match item {
            toml_edit::Item::Table(sub) => clear_positions(sub),
            toml_edit::Item::ArrayOfTables(rows) => rows.iter_mut().for_each(clear_positions),
            _ => {}
        }
    }
}

/// The rows already in the document at an array-of-tables key, whether
/// written as `[[key]]` sections or as `key = [ { … }, { … } ]`.
fn existing_rows(item: &toml_edit::Item) -> Vec<toml_edit::Table> {
    match item {
        toml_edit::Item::ArrayOfTables(rows) => rows.iter().cloned().collect(),
        toml_edit::Item::Value(toml_edit::Value::Array(rows))
            if !rows.is_empty() && rows.iter().all(toml_edit::Value::is_inline_table) =>
        {
            rows.iter()
                .filter_map(|row| row.as_inline_table().cloned())
                .map(toml_edit::InlineTable::into_table)
                .collect()
        }
        _ => Vec::new(),
    }
}

/// For each target row, the index of the existing row it is a new
/// version of, or `None` for a new rule.
///
/// An existing row is projected onto the keys the writer manages, then
/// compared with each target row: an identical projection is the same
/// rule untouched; otherwise the rows sharing the most `path = value`
/// pairs are the same rule edited. Pairs are taken best-first, each row
/// used once, so the choice does not depend on how many rules were added
/// or removed around it. A target row that shares nothing with any
/// leftover row is a new rule and starts with no comments or unmanaged keys.
/// The match is a heuristic and ties are possible (two rules that differ in
/// nothing the writer reads); nothing that changes behaviour may depend on
/// it, which is why a rule's fields are all managed (R-01).
fn match_rows(
    existing: &[toml_edit::Table],
    target: &[&Table],
    owned: &Owned,
) -> Vec<Option<usize>> {
    let projected: Vec<Table> = existing.iter().map(|row| project(row, owned)).collect();
    let leaf_sets: Vec<Vec<(String, String)>> = projected.iter().map(leaves).collect();

    let mut candidates: Vec<(bool, usize, usize, usize)> = Vec::new();
    for (ti, row) in target.iter().enumerate() {
        let row_leaves = leaves(row);
        for (ei, existing_leaves) in leaf_sets.iter().enumerate() {
            let exact = **row == projected[ei];
            let shared = row_leaves
                .iter()
                .filter(|leaf| existing_leaves.contains(leaf))
                .count();
            if exact || shared > 0 {
                candidates.push((exact, shared, ti, ei));
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
    });

    let mut pairing: Vec<Option<usize>> = vec![None; target.len()];
    let mut taken = vec![false; existing.len()];
    for (_, _, ti, ei) in candidates {
        if pairing[ti].is_none() && !taken[ei] {
            pairing[ti] = Some(ei);
            taken[ei] = true;
        }
    }
    pairing
}

/// The part of an existing table the writer manages, as a plain
/// `toml::Table`, so it can be compared with a target row.
fn project(doc: &dyn toml_edit::TableLike, owned: &Owned) -> Table {
    let mut out = Table::new();
    for (key, item) in doc.iter() {
        if !owned.owns(key) {
            continue;
        }
        if let Some(value) = project_item(item, owned.below(key)) {
            out.insert(key.to_owned(), value);
        }
    }
    out
}

fn project_item(item: &toml_edit::Item, owned: &Owned) -> Option<Value> {
    if let Some(table) = item.as_table_like() {
        return Some(Value::Table(project(table, owned)));
    }
    if let Some(rows) = item.as_array_of_tables() {
        return Some(Value::Array(
            rows.iter()
                .map(|row| Value::Table(project(row, owned)))
                .collect(),
        ));
    }
    item.as_value().and_then(value_from_edit)
}

fn value_from_edit(value: &toml_edit::Value) -> Option<Value> {
    match value {
        toml_edit::Value::String(s) => Some(Value::String(s.value().clone())),
        toml_edit::Value::Integer(i) => Some(Value::Integer(*i.value())),
        toml_edit::Value::Float(f) => Some(Value::Float(*f.value())),
        toml_edit::Value::Boolean(b) => Some(Value::Boolean(*b.value())),
        toml_edit::Value::Array(items) => Some(Value::Array(
            items.iter().filter_map(value_from_edit).collect(),
        )),
        toml_edit::Value::Datetime(_) | toml_edit::Value::InlineTable(_) => None,
    }
}

/// Every `path = value` leaf of a table, for comparing two rows.
fn leaves(table: &Table) -> Vec<(String, String)> {
    fn walk(prefix: &str, table: &Table, out: &mut Vec<(String, String)>) {
        for (key, value) in table {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value {
                Value::Table(sub) => walk(&path, sub, out),
                leaf => out.push((path, leaf.to_string())),
            }
        }
    }
    let mut out = Vec::new();
    walk("", table, &mut out);
    out
}

/// Overwrite `key`'s value only if its meaning changed, carrying over
/// its previous decor (the same-line trailing comment) when it did.
///
/// An unchanged leaf is not touched at all. Before, every managed leaf
/// was re-inserted on every save, and `Table::insert` replaces the key
/// along with the value, so the comment lines *above* any managed key
/// were deleted by every `set`. The value is replaced through the
/// existing slot so the key, and the comment above it, stay.
fn set_scalar(doc: &mut dyn toml_edit::TableLike, key: &str, mut new_value: toml_edit::Value) {
    if let Some(item) = doc.get_mut(key) {
        if leaf_is_unchanged(item, &new_value) {
            return;
        }
        if let Some(old) = item.as_value() {
            *new_value.decor_mut() = old.decor().clone();
        }
        *item = toml_edit::Item::Value(new_value);
        return;
    }
    doc.insert(key, toml_edit::value(new_value));
}

/// Whether the existing item already says what `new` says.
///
/// Two spellings count as the same: a value equal in meaning (`'a'` and
/// `"a"`), and a strategy written as a table
/// (`strategy = { weighted_random = { seed = 7 } }`) when the target
/// names the same variant. The target models a strategy as its bare name,
/// so it cannot say `seed`; overwriting would silently drop it, and for
/// `priority` it would turn a valid table into a bare string, which does
/// not load.
fn leaf_is_unchanged(existing: &toml_edit::Item, new: &toml_edit::Value) -> bool {
    if let Some(old) = existing.as_value()
        && same_value(old, new)
    {
        return true;
    }
    if let (Some(table), Some(name)) = (existing.as_table_like(), new.as_str()) {
        return table.len() == 1 && table.iter().next().is_some_and(|(key, _)| key == name);
    }
    false
}

fn same_value(a: &toml_edit::Value, b: &toml_edit::Value) -> bool {
    match (a, b) {
        (toml_edit::Value::String(a), toml_edit::Value::String(b)) => a.value() == b.value(),
        (toml_edit::Value::Integer(a), toml_edit::Value::Integer(b)) => a.value() == b.value(),
        (toml_edit::Value::Float(a), toml_edit::Value::Float(b)) => a.value() == b.value(),
        (toml_edit::Value::Boolean(a), toml_edit::Value::Boolean(b)) => a.value() == b.value(),
        (toml_edit::Value::Array(a), toml_edit::Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| same_value(a, b))
        }
        _ => false,
    }
}

/// A `toml::Value::Array` counts as an array-of-tables target when
/// every element is itself a table — mirrors how `toml::to_string_pretty`
/// already renders such an array as `[[key]]` sections.
fn is_table_array(items: &[Value]) -> bool {
    !items.is_empty() && items.iter().all(|v| matches!(v, Value::Table(_)))
}

/// Convert a `toml::Value` leaf (never a bare `Table`, and never an
/// array of tables — those are handled by the two functions above) to
/// its `toml_edit` equivalent.
fn edit_value_from_leaf(value: &Value) -> toml_edit::Value {
    match value {
        Value::String(s) => toml_edit::Value::from(s.clone()),
        Value::Integer(i) => toml_edit::Value::from(*i),
        Value::Float(f) => toml_edit::Value::from(*f),
        Value::Boolean(b) => toml_edit::Value::from(*b),
        Value::Array(items) => {
            let mut array = toml_edit::Array::new();
            for item in items {
                array.push(edit_value_from_leaf(item));
            }
            toml_edit::Value::from(array)
        }
        Value::Datetime(_) | Value::Table(_) => unreachable!(
            "toml_writer's editable subset never emits a datetime, \
             and a bare Table is handled by reconcile_subtable before \
             reaching a leaf converter"
        ),
    }
}

/// Populate a brand-new `toml_edit::Table` from a `toml::Table` — used
/// only for keys/rows that don't exist in the document being edited
/// yet (e.g. a rule just added via `EditCommand::AddRule`), so there's
/// no prior formatting to preserve.
fn fill_table(dst: &mut toml_edit::Table, src: &Table) {
    for (key, value) in src.iter() {
        match value {
            Value::Table(sub) => {
                let mut nested = toml_edit::Table::new();
                fill_table(&mut nested, sub);
                dst.insert(key, toml_edit::Item::Table(nested));
            }
            Value::Array(items) if is_table_array(items) => {
                let mut aot = toml_edit::ArrayOfTables::new();
                for row in items {
                    let row_table = row
                        .as_table()
                        .expect("is_table_array guarantees every row is a Value::Table");
                    let mut row_out = toml_edit::Table::new();
                    fill_table(&mut row_out, row_table);
                    aot.push(row_out);
                }
                dst.insert(key, toml_edit::Item::ArrayOfTables(aot));
            }
            leaf => {
                dst.insert(key, toml_edit::value(edit_value_from_leaf(leaf)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compose a minimal rule-set TOML containing the given inner
    /// rules block and parse it back to a `RuleSet` for assertions.
    fn parse_rule_set(toml_text: &str) -> RuleSet {
        // Use a temp dir as the rule-set's owning location so the
        // RuleSet::new path-resolution logic has somewhere real.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("apimock-rule-set.toml");
        std::fs::write(&path, toml_text).expect("write");
        RuleSet::new(path.to_str().unwrap(), ".", 0).expect("parse rule set")
    }

    #[test]
    fn round_trip_rule_with_single_header() {
        let original = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api\"\n",
            "when.request.headers.x-api-key = { value = \"secret\" }\n",
            "respond = { text = \"ok\" }\n",
        );
        let rs = parse_rule_set(original);
        assert!(rs.rules[0].when.request.headers.is_some());

        // Render and re-parse — headers should still be present.
        let rendered = render_rule_set_toml(&rs);
        let rs2 = parse_rule_set(&rendered);
        let h = rs2.rules[0]
            .when
            .request
            .headers
            .as_ref()
            .expect("headers preserved across round trip");
        assert!(h.0.contains_key("x-api-key"));
        assert_eq!(h.0["x-api-key"].value, "secret");
    }

    #[test]
    fn round_trip_rule_with_header_op() {
        let original = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api\"\n",
            "when.request.headers.user-agent = { op = \"starts_with\", value = \"Mozilla\" }\n",
            "respond = { text = \"ok\" }\n",
        );
        let rs = parse_rule_set(original);
        let rendered = render_rule_set_toml(&rs);
        let rs2 = parse_rule_set(&rendered);
        let h = rs2.rules[0].when.request.headers.as_ref().unwrap();
        let stmt = &h.0["user-agent"];
        assert!(matches!(
            stmt.op,
            Some(apimock_routing::rule_set::rule::when::request::headers::header_operator::HeaderOperator::StartsWith)
        ));
        assert_eq!(stmt.value, "Mozilla");
    }

    #[test]
    fn round_trip_rule_with_multiple_headers() {
        let original = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api\"\n",
            "when.request.headers.x-api-key = { value = \"secret\" }\n",
            "when.request.headers.x-tenant = { op = \"equal\", value = \"acme\" }\n",
            "respond = { text = \"ok\" }\n",
        );
        let rs = parse_rule_set(original);
        let rendered = render_rule_set_toml(&rs);
        let rs2 = parse_rule_set(&rendered);
        let h = rs2.rules[0].when.request.headers.as_ref().unwrap();
        assert_eq!(h.0.len(), 2);
        assert!(h.0.contains_key("x-api-key"));
        assert!(h.0.contains_key("x-tenant"));
    }

    #[test]
    fn round_trip_rule_with_body_json() {
        let original = concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/api\"\n",
            "when.request.body.json.\"user.name\" = { value = \"alice\" }\n",
            "respond = { text = \"ok\" }\n",
        );
        let rs = parse_rule_set(original);
        let rendered = render_rule_set_toml(&rs);
        let rs2 = parse_rule_set(&rendered);
        let b = rs2.rules[0]
            .when
            .request
            .body
            .as_ref()
            .expect("body preserved across round trip");
        // Body has BodyKind::Json keyed map containing the dotted path.
        let json_kind =
            apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind::Json;
        let inner = b.0.get(&json_kind).expect("json body kind present");
        assert!(inner.contains_key("user.name"));
        assert_eq!(inner["user.name"].value, "alice");
    }

    // ---------------------------------------------------------------
    // Task 019 — the writer deletes only what it manages.
    // ---------------------------------------------------------------

    /// `text` with every line mentioning `future` removed: the document as
    /// the writer's own model would describe it, with nothing the writer
    /// has never heard of.
    fn without_future_keys(text: &str) -> String {
        text.lines()
            .filter(|line| !line.contains("future"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    }

    fn table_of(text: &str) -> Table {
        toml::from_str(text).expect("fixture parses as TOML")
    }

    /// A key the writer has never heard of, at every level of the root
    /// config, comes out exactly as it went in. The assertion is that the
    /// whole file is byte-identical, so it also proves the writer touched
    /// nothing it manages when nothing about that changed.
    #[test]
    fn an_unknown_key_at_every_level_of_the_root_survives_byte_for_byte() {
        let original = "\
# top comment
future_root = 1 # keep-root

[listener]
ip_address = \"127.0.0.1\"
port = 3001
future_listener = 2 # keep-listener

[listener.tls]
cert = \"c.pem\"
key = \"k.pem\"
future_tls = 3 # keep-tls

[log]
future_log = 4 # keep-log

[log.verbose]
header = true
body = false
future_verbose = 5 # keep-verbose

[service]
strategy = \"first_match\"
fallback_respond_dir = \".\"
future_service = 6 # keep-service

[file_tree_view]
show_hidden = true
future_ftv = 7 # keep-ftv
";
        let target = table_of(&without_future_keys(original));
        let out = apply_in_place(original, &target, root_owned()).unwrap();
        assert_eq!(out, original);
    }

    /// The same, for a rule-set file, down to the leaves of a rule.
    #[test]
    fn an_unknown_key_at_every_level_of_a_rule_set_survives_byte_for_byte() {
        let original = "\
future_top = 1 # keep-top
strategy = \"round_robin\"

[future_table] # keep-table
future_inner = 2

[prefix]
url_path = \"/api\"
future_prefix = 3 # keep-prefix

[[rules]]
priority = 1
future_rule = 4 # keep-rule

[rules.when]
future_when = 5 # keep-when

[rules.when.request]
url_path = \"/x\"
method = \"GET\"
future_request = 6 # keep-request

[rules.when.request.headers.x-a]
op = \"equal\"
value = \"1\"
future_header = 7 # keep-header

[rules.when.request.body.json.\"a.b\"]
op = \"equal\"
value = \"2\"
future_body = 8 # keep-body

[rules.respond]
text = \"ok\"
future_respond = 9 # keep-respond
";
        let target = table_of(&without_future_keys(original));
        let out = apply_in_place(original, &target, rule_set_owned()).unwrap();
        assert_eq!(out, original);
    }

    /// An inline table is edited in place, not rebuilt: a key inside it
    /// that the writer does not manage survives, and so does the form.
    #[test]
    fn an_unknown_key_inside_an_inline_table_survives() {
        let original = "\
[listener]
ip_address = \"127.0.0.1\"
port = 3001
tls = { cert = \"c.pem\", key = \"k.pem\", max_connections = 8 }

[[rules]]
when.request.url_path = \"/x\"
respond = { text = \"ok\", future_respond = 1 }
";
        // What the writer's model says: no unknown keys. The target is a
        // listener and a rule; `service` is not part of this fixture.
        let target = table_of(
            "[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n[listener.tls]\ncert = \"c.pem\"\nkey = \"k.pem\"\n",
        );
        let out = apply_in_place(original, &target, root_owned()).unwrap();
        assert!(out.contains("max_connections = 8"), "{out}");
        assert!(
            out.contains("tls = { cert = \"c.pem\", key = \"k.pem\", max_connections = 8 }"),
            "an inline table must stay inline: {out}"
        );

        let target =
            table_of("[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n");
        let out = apply_in_place(original, &target, rule_set_owned()).unwrap();
        assert!(out.contains("future_respond = 1"), "{out}");
        assert!(
            out.contains("respond = { text = \"ok\", future_respond = 1 }"),
            "an inline table must stay inline: {out}"
        );
    }

    /// The other direction: a key the writer *does* manage, which the
    /// model no longer has, is removed — and its unmanaged neighbours stay.
    #[test]
    fn a_managed_key_the_model_no_longer_has_is_removed() {
        let original = "\
[[rules]]
priority = 3
weight = 7
future_rule = 7 # keep-future
when.request.url_path = \"/x\"
respond = { text = \"ok\", status = 201, delay_response_milliseconds = 5 }
";
        // The model cleared priority, weight, status and the delay.
        let target =
            table_of("[[rules]]\nwhen.request.url_path = \"/x\"\nrespond = { text = \"ok\" }\n");
        let out = apply_in_place(original, &target, rule_set_owned()).unwrap();
        assert!(!out.contains("priority"), "{out}");
        assert!(!out.contains("weight"), "{out}");
        assert!(!out.contains("status"), "{out}");
        assert!(!out.contains("delay_response_milliseconds"), "{out}");
        assert!(out.contains("future_rule = 7 # keep-future"), "{out}");
        assert!(out.contains("text = \"ok\""), "{out}");
    }

    /// A managed *table* the model no longer has goes whole, including
    /// keys inside it the writer does not manage: they belong to a section
    /// that no longer exists, and `[listener.tls]` without `cert`/`key`
    /// would not load.
    #[test]
    fn a_managed_table_the_model_no_longer_has_is_removed_whole() {
        let original = "\
[listener]
ip_address = \"127.0.0.1\"
port = 3001

[listener.tls]
cert = \"c.pem\"
key = \"k.pem\"
max_connections = 8
";
        let target = table_of("[listener]\nip_address = \"127.0.0.1\"\nport = 3001\n");
        let out = apply_in_place(original, &target, root_owned()).unwrap();
        assert!(!out.contains("tls"), "{out}");
        assert!(!out.contains("max_connections"), "{out}");
    }

    /// Comments above a managed key used to be deleted by every save: the
    /// leaf was re-inserted, and that replaced the key along with its
    /// comment. An unchanged leaf is now not touched, and a changed one
    /// keeps its key.
    #[test]
    fn comments_above_and_beside_a_managed_key_survive_a_change() {
        let original = "\
# above-port
port = 3001 # beside-port
";
        let mut target = Table::new();
        let mut listener = Table::new();
        listener.insert("port".to_owned(), Value::Integer(4000));
        target.insert("listener".to_owned(), Value::Table(listener));
        let original = format!("[listener]\n{original}");
        let out = apply_in_place(&original, &target, root_owned()).unwrap();
        assert_eq!(out, "[listener]\n# above-port\nport = 4000 # beside-port\n");
    }

    /// A strategy written as a table keeps its parameters when the model
    /// names the same variant. The model holds only the name, so
    /// overwriting would drop `seed`, and for `priority` it would turn a
    /// table that loads into a bare string that does not.
    #[test]
    fn a_strategy_table_survives_when_the_variant_is_unchanged() {
        let original = "strategy = { priority = { tiebreaker = \"uniform_random\" } } # keep\n";
        let target = table_of("strategy = \"priority\"\n");
        let out = apply_in_place(original, &target, rule_set_owned()).unwrap();
        assert_eq!(out, original);

        // A real change is still a change.
        let target = table_of("strategy = \"round_robin\"\n");
        let out = apply_in_place(original, &target, rule_set_owned()).unwrap();
        assert_eq!(out, "strategy = \"round_robin\" # keep\n");
    }

    /// `toml_edit` converts `\r\n` to `\n` on parse, so a CRLF file used to
    /// come back LF-only with every line changed, not just the one that was
    /// edited. Found by the Windows leg of CI on task 019's first push, where
    /// git's `autocrlf` gives every fixture CRLF line endings.
    #[test]
    fn a_crlf_file_keeps_its_line_endings() {
        let original = "# top\r\n[listener]\r\nport = 3001 # p\r\nfuture = 1 # keep\r\n\r\n[service]\r\nfallback_respond_dir = \".\"\r\n";

        // Nothing changed: byte-identical, which a parse-and-print alone is not.
        let target = table_of(&without_future_keys(original).replace("\r\n", "\n"));
        let out = apply_in_place(original, &target, root_owned()).unwrap();
        assert_eq!(out, original);

        // One value changed: that value changes, every line is still CRLF.
        let mut changed = target.clone();
        changed
            .get_mut("listener")
            .and_then(Value::as_table_mut)
            .unwrap()
            .insert("port".to_owned(), Value::Integer(4000));
        let out = apply_in_place(original, &changed, root_owned()).unwrap();
        assert_eq!(out, original.replace("port = 3001 # p", "port = 4000 # p"));
        assert!(
            !out.replace("\r\n", "").contains('\n'),
            "bare LF in: {out:?}"
        );
    }

    /// A line added to a CRLF file is CRLF too, and an LF file stays LF.
    #[test]
    fn added_lines_follow_the_files_line_endings() {
        let target = table_of("[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n");
        let crlf = apply_in_place("", &target, rule_set_owned()).unwrap();
        assert!(
            !crlf.contains('\r'),
            "an empty file has no CRLF to keep: {crlf:?}"
        );

        let original = "[[rules]]\r\nwhen.request.url_path = \"/a\"\r\nrespond.text = \"a\"\r\n";
        let two = table_of(
            "[[rules]]\nwhen.request.url_path = \"/a\"\nrespond.text = \"a\"\n[[rules]]\nwhen.request.url_path = \"/b\"\nrespond.text = \"b\"\n",
        );
        let out = apply_in_place(original, &two, rule_set_owned()).unwrap();
        assert!(out.starts_with(original), "{out:?}");
        assert!(
            !out.replace("\r\n", "").contains('\n'),
            "bare LF in: {out:?}"
        );

        let lf = original.replace("\r\n", "\n");
        let out = apply_in_place(&lf, &two, rule_set_owned()).unwrap();
        assert!(!out.contains('\r'), "an LF file must stay LF: {out:?}");
    }

    /// Key paths of an emitted table, with a user-chosen name (a header
    /// name, a JSON path) shown as `*`; asserts on the way down that the
    /// ownership tree manages every key the writer emitted.
    fn emitted_paths(
        table: &Table,
        owned: &Owned,
        prefix: &str,
        out: &mut std::collections::BTreeSet<String>,
    ) {
        for (key, value) in table {
            assert!(
                owned.owns(key),
                "the writer emits `{prefix}.{key}` but the ownership tree does not manage it, \
                 so it would never be removed when the model clears it"
            );
            let segment = if owned.each.is_some() { "*" } else { key };
            let path = if prefix.is_empty() {
                segment.to_owned()
            } else {
                format!("{prefix}.{segment}")
            };
            out.insert(path.clone());
            match value {
                Value::Table(sub) => emitted_paths(sub, owned.below(key), &path, out),
                Value::Array(rows) if is_table_array(rows) => {
                    for row in rows {
                        emitted_paths(row.as_table().unwrap(), owned.below(key), &path, out);
                    }
                }
                _ => {}
            }
        }
    }

    const MAXIMAL_ROOT: &str = include_str!("../tests/fixtures/maximal/apimock.toml");
    const MAXIMAL_RULE_SET: &str = include_str!("../tests/fixtures/maximal/rules.toml");

    /// R-01. Inside `[[rules]]` the writer must emit every field the model
    /// holds, because a value it does not emit can only survive a save by
    /// riding along on the row pairing, and that pairing is a heuristic. The
    /// fixture sets every rule-level key the docs list (and a docs-driven
    /// test keeps it that way), so a rule key the writer does not emit for
    /// the loaded model shows up here as a difference, instead of as a value
    /// silently lost or moved to another rule. This is the test that would
    /// have caught `weight`.
    #[test]
    fn every_rule_key_the_fixture_sets_is_emitted_for_the_loaded_model() {
        let owned_rule = rule_set_owned().below("rules");

        let raw = table_of(MAXIMAL_RULE_SET);
        let mut in_file = std::collections::BTreeSet::new();
        for row in raw["rules"].as_array().unwrap() {
            emitted_paths(row.as_table().unwrap(), owned_rule, "rules", &mut in_file);
        }

        let model: RuleSet = toml::from_str(MAXIMAL_RULE_SET).expect("rule-set fixture");
        let emitted_table = rule_set_table(&model);
        let mut emitted = std::collections::BTreeSet::new();
        for row in emitted_table["rules"].as_array().unwrap() {
            emitted_paths(row.as_table().unwrap(), owned_rule, "rules", &mut emitted);
        }

        let unemitted: Vec<_> = in_file.difference(&emitted).collect();
        assert!(
            unemitted.is_empty(),
            "the fixture sets these rule keys but the writer does not emit them for the \
             loaded model, so a save cannot keep them attached to their rule: {unemitted:?}"
        );
    }

    /// The ownership tree and the emitters are one description of what
    /// the writer manages, written twice. This keeps them in step in both
    /// directions, against a config that sets every key the writer can
    /// emit: a key emitted but not owned would never be removed when
    /// cleared; a key owned but never emitted is a claim on something the
    /// writer does not write.
    #[test]
    fn the_ownership_tree_is_exactly_what_the_writer_emits() {
        let config: Config = toml::from_str(MAXIMAL_ROOT).expect("root fixture");
        let mut emitted = std::collections::BTreeSet::new();
        emitted_paths(&root_table(&config), root_owned(), "", &mut emitted);
        let mut owned = Vec::new();
        root_owned().paths("", &mut owned);
        assert_eq!(
            emitted,
            owned.into_iter().collect(),
            "root: emitted paths (left) vs owned paths (right)"
        );

        let rule_set: RuleSet = toml::from_str(MAXIMAL_RULE_SET).expect("rule-set fixture");
        let mut emitted = std::collections::BTreeSet::new();
        emitted_paths(
            &rule_set_table(&rule_set),
            rule_set_owned(),
            "",
            &mut emitted,
        );
        let mut owned = Vec::new();
        rule_set_owned().paths("", &mut owned);
        assert_eq!(
            emitted,
            owned.into_iter().collect(),
            "rule set: emitted paths (left) vs owned paths (right)"
        );
    }
}
