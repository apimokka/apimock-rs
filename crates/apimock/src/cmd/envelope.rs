//! The RFC 053 response envelope — shared by every command that emits
//! `--format json`.
//!
//! # Why a helper, not inline `json!` calls per command
//!
//! `validate` is this envelope's first producer, but not its last —
//! v6's `get`/`set` are built against the same contract (RFC 053 § 1).
//! A shared helper means there is exactly one place that decides "is
//! this an object, is `schema` present, is there exactly one of
//! `result`/`error`" — so the next command reuses it rather than
//! re-deriving the shape, which is exactly the drift RFC 053 exists to
//! prevent (its own Motivation: a bare array "cannot gain a field").

/// The envelope's schema version. RFC 053 § 2: producers may add fields
/// to `result`/`error` freely; only a field's removal, a type change, or
/// a meaning change requires incrementing this.
pub const SCHEMA_VERSION: u32 = 1;

/// `error.kind` — a stable, closed set (RFC 053 § Layer 3), defined here
/// in full even though `validate` (this envelope's first producer) only
/// ever emits three of the six — the set is the contract, not a menu
/// `validate` gets to shrink, and a future command reusing this module
/// should never need to extend this enum for a kind RFC 053 already
/// named. New variants may still be added later; that is itself an
/// additive change under § 2's rule, which is why consumers are told to
/// treat an unrecognised `kind` as a generic failure rather than
/// erroring on it.
///
/// No `serde` derive: `apimock`'s own `Cargo.toml` only pulls in
/// `serde_json`, not `serde` itself, and a plain `&str` mapping is all
/// four keys of this envelope need — not worth a new crate dependency.
pub enum ErrorKind {
    /// Bad invocation — unknown option, missing or invalid value.
    Usage,
    /// Configuration read, but not valid.
    ConfigInvalid,
    /// Configuration missing or unreadable.
    ConfigUnreadable,
    /// A filesystem failure that is not the configuration itself.
    Io,
    /// State changed underneath — `set` only (RFC 053 § 6).
    Conflict,
    /// A bug in apimock.
    Internal,
}

impl ErrorKind {
    /// The `error.kind` string RFC 053 § Layer 3 specifies —
    /// lowercase, snake_case, stable.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::ConfigInvalid => "config_invalid",
            Self::ConfigUnreadable => "config_unreadable",
            Self::Io => "io",
            Self::Conflict => "conflict",
            Self::Internal => "internal",
        }
    }
}

/// `--format`'s value — shared by every command that offers it
/// (`validate`, `get`), so there is one definition of "text or json"
/// rather than a copy per command.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
}

/// Map a config-loading failure to one of RFC 053's error kinds, rather
/// than labelling every load failure the same way. Shared by `validate`
/// and `get` — both load an `apimock_config::Config` (or the
/// `Workspace` wrapper around one) and both need this exact judgement:
/// file genuinely missing/unreadable is `config_unreadable`; the file
/// was read but is syntactically or semantically invalid is
/// `config_invalid`.
///
/// `ConfigError` is `#[non_exhaustive]` (RFC 041), so this can no
/// longer force a considered `ErrorKind` for a future variant at
/// compile time the way it could when the type was exhaustive — a new
/// `ConfigError` variant now falls into the wildcard arm below as
/// `Internal` until this match is revisited by hand. This is
/// independent of `ConfigError::kind()` (also added by RFC 041): that
/// accessor describes *library* failures; this function is RFC 053's
/// CLI contract. Deliberately not delegating one to the other — see
/// `apimock_config::error`'s module doc for why.
pub fn kind_for_config_error(e: &apimock_config::ConfigError) -> ErrorKind {
    use apimock_config::ConfigError;
    match e {
        ConfigError::ConfigRead { .. } => ErrorKind::ConfigUnreadable,
        ConfigError::PathResolve { .. } => ErrorKind::ConfigUnreadable,
        ConfigError::ConfigParse { .. } => ErrorKind::ConfigInvalid,
        ConfigError::Validation { .. } => ErrorKind::ConfigInvalid,
        ConfigError::RuleSet(_) => ErrorKind::ConfigInvalid,
        _ => ErrorKind::Internal,
    }
}

/// Map a `Workspace::load` failure to one of RFC 053's error kinds.
/// `WorkspaceError` adds one variant (`InvalidRoot`) on top of
/// `ConfigError`, which `kind_for_config_error` already maps — that
/// shared mapping does the rest.
///
/// Relocated here from `validate.rs` (RFC 057): `set` needs the exact
/// same judgement `validate` already made, and a second private copy
/// is the drift this module's own doc comment argues against.
pub fn kind_for_workspace_error(e: &apimock_config::WorkspaceError) -> ErrorKind {
    use apimock_config::WorkspaceError;
    match e {
        WorkspaceError::InvalidRoot { .. } => ErrorKind::ConfigUnreadable,
        WorkspaceError::Config(inner) => kind_for_config_error(inner),
        _ => ErrorKind::Internal,
    }
}

/// Map a `Workspace::apply` failure to RFC 053's error kinds. Every
/// `ApplyError` variant (RFC 057: `set`'s first caller of this) is the
/// same shape of mistake — a bad address or a bad payload the *caller*
/// supplied, not a state, filesystem or internal problem — so all three
/// map to `usage` rather than needing a per-variant judgement call.
pub fn kind_for_apply_error(_e: &apimock_config::ApplyError) -> ErrorKind {
    ErrorKind::Usage
}

/// Map a `Workspace::save` failure to RFC 053's error kinds. `Conflict`
/// is the one RFC 053 § 6 reserved specifically for this (`set` is its
/// first producer); `Read`/`Write` are filesystem failures ahead of or
/// during the write, mapped to `io`; `Serialize`/`Inconsistent` are
/// this crate's own bugs, not the caller's mistake, mapped to
/// `internal`. `SaveError` is `#[non_exhaustive]` (RFC 041), so a new
/// variant now falls into the wildcard arm as `Internal` rather than
/// failing to compile — this function stays hand-maintained and
/// separate from `SaveError::kind()`, the same way `kind_for_config_error`
/// stays separate from `ConfigError::kind()`.
pub fn kind_for_save_error(e: &apimock_config::SaveError) -> ErrorKind {
    use apimock_config::SaveError;
    match e {
        SaveError::Conflict { .. } => ErrorKind::Conflict,
        SaveError::Read { .. } | SaveError::Write { .. } => ErrorKind::Io,
        SaveError::Serialize { .. } | SaveError::Inconsistent { .. } => ErrorKind::Internal,
        _ => ErrorKind::Internal,
    }
}

/// Map a `RuleSet::new` failure to one of RFC 053's error kinds.
/// `match-test` loads its rule-set file directly via `apimock_routing`,
/// not through `apimock_config::Config`/`Workspace` the way `get`,
/// `validate` and `set` do — so it has no `ConfigError` to reuse
/// `kind_for_config_error` on. The judgement is the same one that
/// function already makes for `ConfigError::RuleSet(_)` (RFC 053 treats
/// a rule-set problem as a config-shaped failure): file genuinely
/// missing/unreadable is `config_unreadable`; read but invalid TOML is
/// `config_invalid`.
pub fn kind_for_routing_error(e: &apimock_routing::RoutingError) -> ErrorKind {
    use apimock_routing::RoutingError;
    match e {
        RoutingError::RuleSetRead { .. } => ErrorKind::ConfigUnreadable,
        RoutingError::RuleSetParse { .. } => ErrorKind::ConfigInvalid,
        _ => ErrorKind::Internal,
    }
}

/// Build a success envelope: `{ "schema": 1, "apimock": "<version>",
/// "result": <result> }`. `result` is a plain JSON value rather than a
/// generic `T: Serialize`, since every caller already has one
/// (`serde_json::json!` or a `Serialize` value converted with
/// `serde_json::to_value`) and a `Value` keeps this module trivially
/// reusable without threading a type parameter through every command.
///
/// # RFC 076 § 3: field order is now insertion order, by decision
///
/// Enabling `serde_json/preserve_order` for RFC 076 (byte-identical
/// `.json` file serving) is a workspace-wide switch — every `Value` in
/// every crate, this envelope included. Before it, `Value::Object`
/// serialised alphabetically, so the wire order was `apimock`,
/// `error`/`result`, `schema` — **not** the `schema`, `apimock`,
/// `result` order every example in `docs/src/reference/cli-reference.md`
/// already showed. **Decision: accept the change** — it makes the
/// actual output match the documented example instead of silently
/// disagreeing with it, which scoping `preserve_order` away from this
/// one `Value` (by rewriting it as a typed struct, whose field order
/// doesn't depend on the feature at all) would not have fixed. Pinned
/// by `field_order_is_schema_then_apimock_then_result_or_error` below.
pub fn ok(result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema": SCHEMA_VERSION,
        "apimock": env!("CARGO_PKG_VERSION"),
        "result": result,
    })
}

/// Build a failure envelope: `{ "schema": 1, "apimock": "<version>",
/// "error": { "kind": …, "message": … } }`.
pub fn err(kind: ErrorKind, message: impl Into<String>) -> serde_json::Value {
    serde_json::json!({
        "schema": SCHEMA_VERSION,
        "apimock": env!("CARGO_PKG_VERSION"),
        "error": {
            "kind": kind.as_str(),
            "message": message.into(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_has_schema_apimock_and_exactly_result() {
        let v = ok(serde_json::json!({"a": 1}));
        let obj = v.as_object().expect("envelope must be a JSON object");
        assert_eq!(obj["schema"], SCHEMA_VERSION);
        assert_eq!(obj["apimock"], env!("CARGO_PKG_VERSION"));
        assert!(obj.contains_key("result"));
        assert!(!obj.contains_key("error"));
        assert_eq!(obj["result"]["a"], 1);
    }

    #[test]
    fn err_has_schema_apimock_and_exactly_error() {
        let v = err(ErrorKind::Usage, "missing required flag --config");
        let obj = v.as_object().expect("envelope must be a JSON object");
        assert_eq!(obj["schema"], SCHEMA_VERSION);
        assert_eq!(obj["apimock"], env!("CARGO_PKG_VERSION"));
        assert!(obj.contains_key("error"));
        assert!(!obj.contains_key("result"));
        assert_eq!(obj["error"]["kind"], "usage");
        assert_eq!(obj["error"]["message"], "missing required flag --config");
    }

    /// RFC 053 § 2's evolution rule, exercised on our own parsing: an
    /// unrecognised field in an envelope must not fail deserialisation —
    /// we are the first consumer as well as the first producer, and
    /// should follow the rule we're asking others to.
    #[test]
    fn unknown_field_is_tolerated_when_parsed_back() {
        let mut v = ok(serde_json::json!({"a": 1}));
        v["future_field_from_a_later_schema"] = serde_json::json!("anything");
        let reparsed: serde_json::Value =
            serde_json::from_str(&v.to_string()).expect("must still parse");
        assert_eq!(reparsed["result"]["a"], 1);
    }

    /// RFC 076 § 3's decision, made executable: the serialised field
    /// order is `schema`, `apimock`, then `result`/`error` — insertion
    /// order, matching every example in `cli-reference.md` — not the
    /// alphabetical order `serde_json` produces without
    /// `preserve_order`. Compares serialised text, not parsed access,
    /// since parsing back into a `Value`/struct is exactly the step that
    /// would hide an order regression.
    #[test]
    fn field_order_is_schema_then_apimock_then_result_or_error() {
        let ok_json = ok(serde_json::json!({"a": 1})).to_string();
        let expected_ok_prefix = format!(
            "{{\"schema\":{},\"apimock\":\"{}\",\"result\":",
            SCHEMA_VERSION,
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            ok_json.starts_with(&expected_ok_prefix),
            "expected {ok_json:?} to start with {expected_ok_prefix:?}"
        );

        let err_json = err(ErrorKind::Usage, "x").to_string();
        let expected_err_prefix = format!(
            "{{\"schema\":{},\"apimock\":\"{}\",\"error\":",
            SCHEMA_VERSION,
            env!("CARGO_PKG_VERSION")
        );
        assert!(
            err_json.starts_with(&expected_err_prefix),
            "expected {err_json:?} to start with {expected_err_prefix:?}"
        );
    }

    #[test]
    fn is_never_a_bare_array() {
        assert!(ok(serde_json::json!([1, 2, 3])).is_object());
        assert!(err(ErrorKind::Internal, "x").is_object());
    }
}
