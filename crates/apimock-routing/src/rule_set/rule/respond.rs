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

use std::{collections::HashMap, path::Path};

/// `#[non_exhaustive]` (RFC 041): every field is `Option`, so an empty
/// `Respond` — matching nothing until at least one is set — is a
/// meaningful `Default`, unlike `Rule` (see its own doc comment).
/// Construct with `Respond::default()` then assign, e.g.
/// `let mut r = Respond::default(); r.text = Some("ok".into());`.
#[derive(Clone, Default, Deserialize, Debug)]
#[non_exhaustive]
pub struct Respond {
    pub file_path: Option<String>,
    pub csv_records_key: Option<String>,
    pub text: Option<String>,
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
    /// at least one of `file_path` / `text` / `status`; `file_path` and
    /// `text` can't both be set; `file_path` can't be combined with
    /// `status`; and when `file_path` is set, the file must exist on
    /// disk under `dir_prefix`.
    ///
    /// # Why this stays with the struct rather than moving to server
    ///
    /// All four checks are semantic validity checks on the *definition* —
    /// they don't produce HTTP responses or touch hyper at all. They
    /// belong on the data type so `apimock-config` can call `.validate()`
    /// at startup without pulling in the server crate.
    pub fn validate(&self, dir_prefix: &str, rule_idx: usize, rule_set_idx: usize) -> bool {
        let all_missing_of_file_path_text_status =
            self.file_path.is_none() && self.text.is_none() && self.status.is_none();
        if all_missing_of_file_path_text_status {
            log::error!(
                "{} at least either of: file_path, text or status (rule #{} in rule set #{})",
                console::style("required").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        let duplicate_file_path_text = self.file_path.is_some() && self.text.is_some();
        if duplicate_file_path_text {
            log::error!(
                "{} set at both file_path and text (rule #{} in rule set #{})",
                console::style("cannot").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        let file_path_with_status = self.file_path.is_some() && self.status.is_some();
        if file_path_with_status {
            log::error!(
                "{} use status with file_path. only with text (rule #{} in rule set #{})",
                console::style("cannot").red(),
                rule_idx + 1,
                rule_set_idx + 1
            );
            return false;
        }

        if let Some(file_path) = self.file_path.as_ref() {
            file_path_validate(file_path.as_str(), dir_prefix, rule_idx, rule_set_idx)
        } else {
            true
        }
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
        if let Some(file_path) = self.file_path.as_ref() {
            let _ = writeln!(f, "file_path = `{}` ", file_path);
        }

        Ok(())
    }
}

/// Check whether a configured file path actually exists under `dir_prefix`.
fn file_path_validate(
    file_path: &str,
    dir_prefix: &str,
    rule_idx: usize,
    rule_set_idx: usize,
) -> bool {
    let p = Path::new(dir_prefix).join(file_path);
    let ret = p.exists();
    if !ret {
        log::error!(
            "{} (rule #{} in rule set #{}):\n`{}`",
            console::style("file not found").red(),
            rule_idx + 1,
            rule_set_idx + 1,
            p.to_str().unwrap_or_default(),
        );
    }
    ret
}
