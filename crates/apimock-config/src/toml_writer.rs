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

/// Serialize an HTTP method back to its TOML form. Inverse of the
/// `Deserialize` derive on `HttpMethod`.
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
// In-place mutation (RFC 056) — reconcile a parsed `toml_edit`
// document against the editable-subset `Table` above. This mutates
// the document rather than rebuilding one: existing keys keep their
// comments, blank lines and position; only a changed key's *value* is
// replaced, and only when it actually differs in shape or content.
// -------------------------------------------------------------------

/// Apply `target` onto `original`'s previous text, preserving
/// everything `target` doesn't touch. Returns the file's new text.
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
) -> Result<String, toml_edit::TomlError> {
    let mut doc: toml_edit::DocumentMut = original.parse()?;
    reconcile_table(doc.as_table_mut(), target);
    Ok(doc.to_string())
}

/// Reconcile one `toml_edit` table against the corresponding `target`
/// table: remove keys `target` no longer has, recurse into nested
/// tables, recurse index-by-index into `[[rules]]`-style
/// arrays-of-tables (mirroring `workspace/diff.rs`'s own precedent for
/// comparing them), and otherwise overwrite the leaf value in place.
fn reconcile_table(doc: &mut toml_edit::Table, target: &Table) {
    let stale: Vec<String> = doc
        .iter()
        .map(|(k, _)| k.to_owned())
        .filter(|k| !target.contains_key(k.as_str()))
        .collect();
    for key in &stale {
        doc.remove(key);
    }

    for (key, value) in target.iter() {
        match value {
            Value::Table(sub) => reconcile_subtable(doc, key, sub),
            Value::Array(items) if is_table_array(items) => {
                reconcile_array_of_tables(doc, key, items)
            }
            leaf => set_scalar(doc, key, edit_value_from_leaf(leaf)),
        }
    }
}

fn reconcile_subtable(doc: &mut toml_edit::Table, key: &str, target: &Table) {
    if let Some(existing) = doc.get_mut(key)
        && let Some(existing_table) = existing.as_table_mut()
    {
        reconcile_table(existing_table, target);
        return;
    }
    let mut fresh = toml_edit::Table::new();
    fill_table(&mut fresh, target);
    doc.insert(key, toml_edit::Item::Table(fresh));
}

fn reconcile_array_of_tables(doc: &mut toml_edit::Table, key: &str, target_rows: &[Value]) {
    if target_rows.is_empty() {
        doc.remove(key);
        return;
    }
    let already_array_of_tables = doc
        .get(key)
        .map(toml_edit::Item::is_array_of_tables)
        .unwrap_or(false);
    if !already_array_of_tables {
        doc.insert(
            key,
            toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
        );
    }
    let aot = doc
        .get_mut(key)
        .and_then(toml_edit::Item::as_array_of_tables_mut)
        .expect("just inserted or confirmed an array-of-tables at this key");

    while aot.len() > target_rows.len() {
        aot.remove(aot.len() - 1);
    }
    for (index, row) in target_rows.iter().enumerate() {
        let row_table = row
            .as_table()
            .expect("is_table_array guarantees every row is a Value::Table");
        if index < aot.len() {
            let existing_row = aot.get_mut(index).expect("index < aot.len()");
            reconcile_table(existing_row, row_table);
        } else {
            let mut fresh = toml_edit::Table::new();
            fill_table(&mut fresh, row_table);
            aot.push(fresh);
        }
    }
}

/// Overwrite `key`'s value, carrying over its previous decor (leading
/// blank lines / comment lines, and same-line trailing comment) so a
/// scalar edit doesn't strip formatting that belongs to that line.
fn set_scalar(doc: &mut toml_edit::Table, key: &str, mut new_value: toml_edit::Value) {
    if let Some(old_decor) = doc
        .get(key)
        .and_then(toml_edit::Item::as_value)
        .map(toml_edit::Value::decor)
        .cloned()
    {
        *new_value.decor_mut() = old_decor;
    }
    doc.insert(key, toml_edit::Item::Value(new_value));
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
}
