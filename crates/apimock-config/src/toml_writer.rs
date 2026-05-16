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
            request::{Request, http_method::HttpMethod, url_path::UrlPathConfig},
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

fn rule_table(r: &Rule) -> Table {
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
                    // Display impl on RuleOp gives the canonical TOML name
                    // (`equals`, `starts_with`, `wild_card`, etc.).
                    dt.insert("op".to_owned(), Value::String(format!("{}", op)));
                }
                t.insert("url_path".to_owned(), Value::Table(dt));
            }
        }
    }

    if let Some(method) = req.http_method.as_ref() {
        t.insert("method".to_owned(), Value::String(http_method_name(method)));
    }

    // headers and body conditions: these have nested complex shapes.
    // Stage-1 GUI editing doesn't expose them, so the editor never
    // reaches in. If the loaded TOML had them, the `When::Request`
    // model preserves them — but we don't have a reverse path because
    // the routing crate's `Headers` / `Body` types aren't `Serialize`
    // and don't expose their internal `HashMap` shape publicly. For
    // 5.2.0 we accept that headers/body conditions in a rule will be
    // dropped on save; this is documented in the SaveResult.diagnostics.
    //
    // A future stage can add `Serialize` (or expose the maps) and
    // round-trip those clauses faithfully. Until then, the GUI is
    // strongly encouraged to warn users editing rule-sets that
    // contain them.

    t
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
