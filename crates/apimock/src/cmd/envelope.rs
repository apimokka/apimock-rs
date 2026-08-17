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

/// Build a success envelope: `{ "schema": 1, "apimock": "<version>",
/// "result": <result> }`. `result` is a plain JSON value rather than a
/// generic `T: Serialize`, since every caller already has one
/// (`serde_json::json!` or a `Serialize` value converted with
/// `serde_json::to_value`) and a `Value` keeps this module trivially
/// reusable without threading a type parameter through every command.
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

    #[test]
    fn is_never_a_bare_array() {
        assert!(ok(serde_json::json!([1, 2, 3])).is_object());
        assert!(err(ErrorKind::Internal, "x").is_object());
    }
}
