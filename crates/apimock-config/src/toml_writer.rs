//! TOML rendering for the editable subset of `Config` and `RuleSet`.
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
//! # Why we don't try to preserve formatting
//!
//! Per the GUI extension spec §6 ("コメント保持は best effort（必須要件
//! ではない）") and §11 ("完全なコメント保持" is explicit non-goal),
//! the save path is allowed to lose comments and key ordering. The
//! `SaveResult` returned by `Workspace::save` includes an `Info`
//! diagnostic noting this so the GUI can warn the user once.
//!
//! Future work could swap this module for `toml_edit` to preserve
//! formatting; the public `Workspace::save` API would not change.

use apimock_routing::{
    Respond, RuleSet,
    rule_set::rule::{
        Rule,
        when::{
            When,
            condition_statement::ConditionStatement,
            request::{
                Request, body::{body_kind::BodyKind, BodyConditionStatement},
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
    let mut root = Table::new();

    if let Some(listener) = config.listener.as_ref() {
        root.insert("listener".to_owned(), Value::Table(listener_table(listener)));
    }
    if let Some(log) = config.log.as_ref() {
        if let Some(t) = log_table(log) {
            root.insert("log".to_owned(), Value::Table(t));
        }
    }
    root.insert("service".to_owned(), Value::Table(service_table(&config.service)));

    toml::to_string_pretty(&Value::Table(root))
        .unwrap_or_else(|err| format!("# failed to render apimock.toml: {}\n", err))
}

/// Render one rule-set TOML to text.
pub fn render_rule_set_toml(rule_set: &RuleSet) -> String {
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

    if !rule_set.rules.is_empty() {
        let rules: Vec<Value> = rule_set
            .rules
            .iter()
            .map(|r| Value::Table(rule_table(r)))
            .collect();
        root.insert("rules".to_owned(), Value::Array(rules));
    }

    toml::to_string_pretty(&Value::Table(root))
        .unwrap_or_else(|err| format!("# failed to render rule set: {}\n", err))
}

// -------------------------------------------------------------------
// Internal helpers — one function per editable struct in the config.
// Each returns a `toml::Table` rather than a `Value` so callers can
// decide whether to skip empty tables.
// -------------------------------------------------------------------

fn listener_table(l: &ListenerConfig) -> Table {
    let mut t = Table::new();
    t.insert("ip_address".to_owned(), Value::String(l.ip_address.clone()));
    t.insert(
        "port".to_owned(),
        Value::Integer(i64::from(l.port)),
    );
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
    t.insert("when".to_owned(), Value::Table(when_table(&r.when)));
    t.insert("respond".to_owned(), Value::Table(respond_table(&r.respond)));
    t
}

fn when_table(w: &When) -> Table {
    let mut t = Table::new();
    t.insert("request".to_owned(), Value::Table(request_table(&w.request)));
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
            headers_table.insert(key.clone(), Value::Table(condition_statement_table(stmt)));
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
                kind_table.insert(key.clone(), Value::Table(body_condition_statement_table(stmt)));
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

/// Render a `ConditionStatement` (used by Headers entries) as a TOML
/// table with `op` (optional) and `value` keys.
fn condition_statement_table(stmt: &ConditionStatement) -> Table {
    let mut t = Table::new();
    if let Some(op) = stmt.op.as_ref() {
        t.insert(
            "op".to_owned(),
            Value::String(apimock_routing::view::build::op_name(op)),
        );
    }
    t.insert("value".to_owned(), Value::String(stmt.value.clone()));
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
            Some(apimock_routing::rule_set::rule::when::request::rule_op::RuleOp::StartsWith)
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
        let json_kind = apimock_routing::rule_set::rule::when::request::body::body_kind::BodyKind::Json;
        let inner = b.0.get(&json_kind).expect("json body kind present");
        assert!(inner.contains_key("user.name"));
        assert_eq!(inner["user.name"].value, "alice");
    }
}
