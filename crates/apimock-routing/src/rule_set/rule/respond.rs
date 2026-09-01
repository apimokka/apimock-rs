//! Declarative response shape matched by a rule.
//!
//! # Why this struct is data-only (5.0 split)
//!
//! Pre-5.0, `Respond` carried both the data (file path / text / status)
//! and the logic to build an HTTP response from it. The 5.0 refactor
//! moved HTTP-response construction into `apimock-server`: the routing
//! crate must stay free of hyper body / response helpers so that it can
//! be a clean dependency target for a future GUI. `Respond` now just
//! describes what the user wrote in their TOML; the server consumes that
//! description and builds the actual HTTP response.

use hyper::StatusCode;
use serde::Deserialize;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// `#[non_exhaustive]` (RFC 041): every field is `Option`, so an empty
/// `Respond` — matching nothing until at least one is set — is a
/// meaningful `Default`, unlike `Rule` (see its own doc comment).
/// Construct with `Respond::default()` then assign, e.g.
/// `let mut r = Respond::default(); r.text = Some("ok".into());`.
///
/// # The body-source model (RFC 065)
///
/// `file_path`, `text` and `json` are **mutually exclusive** — a rule
/// declares exactly one body source, and `validate` rejects any
/// combination of more than one. Content-type is derived from *which*
/// field is set (`file_path` → from its extension, `text` →
/// `text/plain; charset=utf-8`, `json` → `application/json`, none →
/// unset) and an explicit `headers.content-type` always overrides that
/// default, on every source uniformly — see `apimock_server`'s
/// `respond_response`/`ResponseHandler` for where that derivation and
/// override actually happen; this type only declares the choice.
///
/// A plain `text` value that happens to *look* like JSON is still
/// served as `text/plain` — `json` is a distinct, explicit choice, not
/// inferred from content. This is deliberate: a body that looks like
/// JSON is not a JSON body.
#[derive(Clone, Default, Deserialize, Debug)]
#[non_exhaustive]
#[serde(deny_unknown_fields)]
pub struct Respond {
    pub file_path: Option<String>,
    pub csv_records_key: Option<String>,
    pub text: Option<String>,
    /// The response body, declared as JSON (RFC 065). Served as
    /// `application/json` unless an explicit `headers.content-type`
    /// overrides it. Mutually exclusive with `file_path` and `text`.
    /// Validated at load time with the same JSON5 parser
    /// `apimock_server`'s `json_response` uses to serve it, so a value
    /// accepted here is guaranteed to still parse at request time.
    pub json: Option<String>,
    pub status: Option<u16>,
    #[serde(skip)]
    pub status_code: Option<StatusCode>,
    pub headers: Option<HashMap<String, Option<String>>>,
    pub delay_response_milliseconds: Option<u32>,
}

impl Respond {
    /// Startup-time validation of a response declaration.
    ///
    /// Checks exactly the constraints the `respond` TOML form imposes:
    /// at least one of `file_path` / `text` / `json` / `status`;
    /// `file_path`, `text` and `json` are mutually exclusive (at most
    /// one); `file_path` can't be combined with `status`; inline `json`
    /// must parse; and when `file_path` is set, the file must exist on
    /// disk under `dir_prefix` and, if its extension is `json`/`json5`,
    /// its content must parse too (RFC 065 D3 — previously only
    /// existence was checked, so a malformed `.json` file loaded fine
    /// and 500'd on every request instead of failing to load).
    ///
    /// # Why `Result<(), String>` and not `bool`
    ///
    /// A bare `bool` can log a detailed reason at the call site, but
    /// has no way to hand that reason back to a caller that isn't
    /// reading the log — and for `apimock validate`/`get`/`set`/
    /// `match-test`, nothing installs a logger at all, so a `bool`
    /// failure reached the CLI as a bare "configuration validation
    /// failed," with the actual reason nowhere the caller could see it
    /// (RFC 065 review: "the error must name the file and the parse
    /// position"). The `Err` string here is that reason, threaded
    /// through `Rule::validate` → `ServiceConfig::validate` →
    /// `ConfigError::Validation { reason }`, so it reaches whichever
    /// caller actually asked.
    ///
    /// # Why this stays with the struct rather than moving to server
    ///
    /// All the checks here are semantic validity checks on the
    /// *definition* — they don't produce HTTP responses or touch hyper
    /// at all. They belong on the data type so `apimock-config` can
    /// call `.validate()` at startup without pulling in the server
    /// crate. `json5` (also `apimock-server`'s request-time parser) is
    /// a plain data-parsing dependency, not a hyper/server one, so this
    /// doesn't cross that boundary.
    pub fn validate(
        &self,
        dir_prefix: &str,
        rule_idx: usize,
        rule_set_idx: usize,
    ) -> Result<(), String> {
        let all_missing = self.file_path.is_none()
            && self.text.is_none()
            && self.json.is_none()
            && self.status.is_none();
        if all_missing {
            return Err(format!(
                "at least one of file_path, text, json or status is required (rule #{} in rule set #{})",
                rule_idx + 1,
                rule_set_idx + 1
            ));
        }

        let body_sources_set = [
            self.file_path.is_some(),
            self.text.is_some(),
            self.json.is_some(),
        ]
        .into_iter()
        .filter(|&set| set)
        .count();
        if body_sources_set > 1 {
            return Err(format!(
                "file_path, text and json are mutually exclusive; exactly one may be set (rule #{} in rule set #{})",
                rule_idx + 1,
                rule_set_idx + 1
            ));
        }

        if self.file_path.is_some() && self.status.is_some() {
            return Err(format!(
                "cannot use status with file_path; only with text or json (rule #{} in rule set #{})",
                rule_idx + 1,
                rule_set_idx + 1
            ));
        }

        if let Some(json_str) = self.json.as_ref() {
            json5::from_str::<serde_json::Value>(json_str).map_err(|e| {
                format!(
                    "invalid `json` (rule #{} in rule set #{}): {}",
                    rule_idx + 1,
                    rule_set_idx + 1,
                    e
                )
            })?;
        }

        if let Some(file_path) = self.file_path.as_ref() {
            file_path_validate(file_path.as_str(), dir_prefix, rule_idx, rule_set_idx)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Respond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(status_code) = self.status_code {
            let _ = writeln!(f, "status_code = {} ", status_code);
        }
        if let Some(text) = self.text.as_ref() {
            let _ = writeln!(f, "text = `{}` ", text);
        }
        if let Some(json) = self.json.as_ref() {
            let _ = writeln!(f, "json = `{}` ", json);
        }
        if let Some(file_path) = self.file_path.as_ref() {
            let _ = writeln!(f, "file_path = `{}` ", file_path);
        }

        Ok(())
    }
}

/// `Path::join` performs no normalisation — `Path::new(dir_prefix)`,
/// where `dir_prefix` (`RuleSet::dir_prefix`) is itself already `"./."`
/// for the common case of no `[prefix]` block in a config sitting in
/// the current directory (the config-relative `.` joined with the
/// respond-dir default `.`), joined with a bare `file_path` renders as
/// `"././bad.json"` — correct for `fs::read_to_string` (redundant `.`
/// components are inert to any OS), but confusing in a message a
/// person reads (RFC 065 review: this RFC is what first put this join
/// in front of a user, in `apimock validate`'s own output, rather than
/// only feeding it to a syscall that didn't care). Filters out
/// `Component::CurDir` entirely — bare `"bad.json"` is already this
/// codebase's own convention for "relative to the current directory,
/// no further context needed" (e.g. a missing `--config` names the
/// bare file, no `./` prefix).
fn display_path(p: &Path) -> String {
    use std::path::Component;
    let cleaned: PathBuf = p
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    if cleaned.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        cleaned.to_string_lossy().into_owned()
    }
}

/// Check that a configured file path exists under `dir_prefix` and,
/// when its extension is `json`/`json5`, that its content parses (RFC
/// 065 D3) — the same JSON5 parser `apimock_server::json_response`
/// uses at request time, so a config that loads is guaranteed to still
/// serve, not fail with a 500 on the first request.
fn file_path_validate(
    file_path: &str,
    dir_prefix: &str,
    rule_idx: usize,
    rule_set_idx: usize,
) -> Result<(), String> {
    let p = Path::new(dir_prefix).join(file_path);
    if !p.exists() {
        return Err(format!(
            "file not found (rule #{} in rule set #{}): `{}`",
            rule_idx + 1,
            rule_set_idx + 1,
            display_path(&p),
        ));
    }

    let is_json_like = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| e == "json" || e == "json5");
    if is_json_like {
        let content = std::fs::read_to_string(&p).map_err(|e| {
            format!(
                "failed to read `{}` (rule #{} in rule set #{}): {}",
                display_path(&p),
                rule_idx + 1,
                rule_set_idx + 1,
                e
            )
        })?;
        json5::from_str::<serde_json::Value>(content.as_str()).map_err(|e| {
            format!(
                "`{}` is not valid JSON (rule #{} in rule set #{}): {}",
                display_path(&p),
                rule_idx + 1,
                rule_set_idx + 1,
                e
            )
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn respond_with(f: impl FnOnce(&mut Respond)) -> Respond {
        let mut r = Respond::default();
        f(&mut r);
        r
    }

    // ── At least one source, or status alone ────────────────────────

    #[test]
    fn empty_respond_is_rejected() {
        let r = Respond::default();
        let err = r.validate(".", 0, 0).unwrap_err();
        assert!(err.contains("at least one"), "message was: {err}");
    }

    #[test]
    fn status_alone_is_accepted() {
        let r = respond_with(|r| r.status = Some(204));
        assert!(r.validate(".", 0, 0).is_ok());
    }

    // ── Exactly one body source (D3 §3, extended for `json`) ────────

    #[test]
    fn text_alone_is_accepted() {
        let r = respond_with(|r| r.text = Some("hi".to_owned()));
        assert!(r.validate(".", 0, 0).is_ok());
    }

    #[test]
    fn json_alone_is_accepted() {
        let r = respond_with(|r| r.json = Some(r#"{"a":1}"#.to_owned()));
        assert!(r.validate(".", 0, 0).is_ok());
    }

    #[test]
    fn json_and_text_together_are_rejected() {
        let r = respond_with(|r| {
            r.json = Some(r#"{"a":1}"#.to_owned());
            r.text = Some("hi".to_owned());
        });
        let err = r.validate(".", 0, 0).unwrap_err();
        assert!(err.contains("mutually exclusive"), "message was: {err}");
    }

    #[test]
    fn json_and_file_path_together_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        std::fs::write(&path, "{}").unwrap();
        let r = respond_with(|r| {
            r.json = Some(r#"{"a":1}"#.to_owned());
            r.file_path = Some("data.json".to_owned());
        });
        let err = r.validate(dir.path().to_str().unwrap(), 0, 0).unwrap_err();
        assert!(err.contains("mutually exclusive"), "message was: {err}");
    }

    #[test]
    fn text_and_file_path_together_are_still_rejected() {
        // Pre-existing (RFC 065 predates neither introducing nor fixing
        // this) — pinned here alongside the two new pairs so the three
        // combinations are asserted by one consistent table, not one
        // old test elsewhere plus two new ones here.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "hi").unwrap();
        let r = respond_with(|r| {
            r.text = Some("hi".to_owned());
            r.file_path = Some("f.txt".to_owned());
        });
        let err = r.validate(dir.path().to_str().unwrap(), 0, 0).unwrap_err();
        assert!(err.contains("mutually exclusive"), "message was: {err}");
    }

    #[test]
    fn file_path_with_status_is_still_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "hi").unwrap();
        let r = respond_with(|r| {
            r.file_path = Some("f.txt".to_owned());
            r.status = Some(200);
        });
        let err = r.validate(dir.path().to_str().unwrap(), 0, 0).unwrap_err();
        assert!(err.contains("status"), "message was: {err}");
    }

    #[test]
    fn json_with_status_is_accepted() {
        // Unlike `file_path`, `json` may pair with `status` — the same
        // way `text` already could (a custom-status JSON error body).
        let r = respond_with(|r| {
            r.json = Some(r#"{"error":"nope"}"#.to_owned());
            r.status = Some(404);
        });
        assert!(r.validate(".", 0, 0).is_ok());
    }

    // ── D3: inline `json` must parse ─────────────────────────────────

    #[test]
    fn malformed_inline_json_is_rejected_naming_the_rule() {
        let r = respond_with(|r| r.json = Some("{not json".to_owned()));
        let err = r.validate(".", 2, 1).unwrap_err();
        assert!(err.contains("rule #3"), "message was: {err}");
        assert!(err.contains("rule set #2"), "message was: {err}");
    }

    // ── D3: a referenced `.json`/`.json5` file must parse ─────────────

    #[test]
    fn malformed_referenced_json_file_is_rejected_naming_the_file_and_position() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{\"a\": ,,,BROKEN").unwrap();
        let r = respond_with(|r| r.file_path = Some("bad.json".to_owned()));
        let err = r.validate(dir.path().to_str().unwrap(), 0, 0).unwrap_err();
        assert!(err.contains("bad.json"), "message was: {err}");
        // json5's own Display already includes "at line N column N" —
        // asserting that shape here, rather than a specific number,
        // keeps this test from being tied to json5's exact wording.
        assert!(err.contains("line"), "message was: {err}");
        assert!(err.contains("column"), "message was: {err}");
    }

    // ── REVIEW-001 F-1: no doubled `./` in a displayed path ────────────

    #[test]
    fn a_dot_dir_prefix_does_not_double_up_in_the_displayed_path() {
        // `dir_prefix = "."` (the literal value `RuleSet::dir_prefix()`
        // resolves to for a config with no `[prefix]` block, sitting in
        // the current directory: the config-relative `.` joined with
        // the respond-dir default `.`, i.e. `Path::new(".").join(".")`
        // — itself already `"./."`) — `Path::new(dir_prefix).join(file_path)`
        // performs no normalisation, so an unfixed display would show
        // `"././bad.json"`. Reproduced directly with `dir_prefix = "./."`
        // rather than routing through `RuleSet` construction, since that
        // string is the one `Respond::validate`'s own caller actually
        // passes.
        let r = respond_with(|r| r.file_path = Some("bad.json".to_owned()));
        let err = r.validate("./.", 0, 0).unwrap_err();
        assert!(
            !err.contains("././bad.json") && !err.contains(".//./bad.json"),
            "message should not show a doubled './': {err}"
        );
        assert!(err.contains("bad.json"), "message was: {err}");
    }

    #[test]
    fn display_path_strips_current_dir_components() {
        assert_eq!(
            display_path(Path::new(".").join(".").join("bad.json").as_path()),
            "bad.json"
        );
        // Built via `Path::join`, not a hardcoded `"data/bad.json"`
        // literal — a bare string literal bakes in `/`, which isn't
        // `data\bad.json` on Windows (caught by CI: RFC 061's matrix).
        let with_dir = Path::new("data").join("bad.json");
        assert_eq!(display_path(&with_dir), with_dir.to_string_lossy());
        assert_eq!(display_path(Path::new(".")), ".");
    }

    #[test]
    fn malformed_referenced_json5_file_is_also_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.json5"), "not json at all").unwrap();
        let r = respond_with(|r| r.file_path = Some("bad.json5".to_owned()));
        assert!(
            r.validate(dir.path().to_str().unwrap(), 0, 0).is_err(),
            "a malformed .json5 file must be rejected the same way as .json"
        );
    }

    #[test]
    fn valid_referenced_json_file_loads() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.json"), r#"{"a":1}"#).unwrap();
        let r = respond_with(|r| r.file_path = Some("good.json".to_owned()));
        assert!(r.validate(dir.path().to_str().unwrap(), 0, 0).is_ok());
    }

    #[test]
    fn a_non_json_file_path_is_never_content_checked() {
        // `.csv`, `.html`, etc. are not JSON — D3 only extends the
        // check to `.json`/`.json5`, never to every `file_path`.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.csv"), "not,json,at,all\n1,2,3").unwrap();
        let r = respond_with(|r| r.file_path = Some("data.csv".to_owned()));
        assert!(r.validate(dir.path().to_str().unwrap(), 0, 0).is_ok());
    }

    #[test]
    fn a_missing_file_path_is_still_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let r = respond_with(|r| r.file_path = Some("does-not-exist.json".to_owned()));
        let err = r.validate(dir.path().to_str().unwrap(), 0, 0).unwrap_err();
        assert!(err.contains("does-not-exist.json"), "message was: {err}");
    }

    // ── Same parser, load and serve (checklist §3's own evidence bar) ─

    #[test]
    fn load_time_and_the_json5_crate_agree_on_a_value_with_a_trailing_comma() {
        // JSON5 (unlike strict JSON) permits a trailing comma — this is
        // deliberately *not* valid JSON, to prove `Respond::validate`
        // really does use `json5::from_str`, not `serde_json`, the same
        // parser `apimock_server::json_response` calls at request time.
        let r = respond_with(|r| r.json = Some(r#"{"a":1,}"#.to_owned()));
        assert!(
            r.validate(".", 0, 0).is_ok(),
            "a JSON5-legal trailing comma must be accepted, proving the JSON5 parser is in use"
        );
    }
}
