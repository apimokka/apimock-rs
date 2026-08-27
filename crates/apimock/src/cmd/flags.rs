//! Minimal flag-parsing helpers shared by `match-test` and `get` (RFC 055).
//!
//! Extracted rather than duplicated a second time — `match-test` already
//! had its own private copies before `get` needed the identical logic.

/// If `arg` looks like `<flag>=<value>` (RFC 064 Amendment 1), splits
/// at the **first** `=` and returns the two halves. Only ever
/// considered for a token starting with `-` — a value that
/// legitimately contains `=` and isn't itself a flag token
/// (`-H "Authorization: Basic YWJj=="`, a positional
/// `get "/a?x=1&y=2"`) never reaches this: neither of those tokens
/// starts with `-`, so this returns `None` immediately and the
/// caller's existing next-token lookup handles them exactly as it
/// always has. `arg.split_once('=')` splits at the first `=` only, so
/// a value that itself contains `=` (`--json={"a":"b=c"}`) keeps every
/// `=` after the first as part of the value.
pub(crate) fn split_equals_form(arg: &str) -> Option<(&str, &str)> {
    if !arg.starts_with('-') {
        return None;
    }
    arg.split_once('=')
}

/// The value following the first flag in `names` that appears in
/// `args`, if that value doesn't itself look like another flag — in
/// either the space form (`--config path`) or the `=` form
/// (`--config=path`, RFC 064 Amendment 1).
///
/// # Why this returns a `Result`, not an `Option`
///
/// An `Option<String>` cannot tell "the flag wasn't given" from "the
/// flag was given with no value" (dangling at the end of `args`, or
/// immediately followed by another flag) — both collapsed to `None`,
/// and every caller silently fell back to its default (RFC 064).
/// Forcing a `Result` return makes every one of the 26 call sites
/// across `get`/`set`/`validate`/`match-test` handle the dangling case
/// explicitly, via the same `?` that already threads a `String` error
/// up to each command's `usage_error`, rather than inviting the same
/// silent-default bug a 27th time. `flag_present` is untouched — a
/// boolean flag's presence is never ambiguous the way a value-taking
/// one's is.
pub(super) fn flag_value(args: &[String], names: &[&str]) -> Result<Option<String>, String> {
    for (idx, a) in args.iter().enumerate() {
        if let Some((name, value)) = split_equals_form(a) {
            if names.contains(&name) {
                return Ok(Some(value.to_owned()));
            }
            continue;
        }
        if !names.iter().any(|n| a == n) {
            continue;
        }
        return match args.get(idx + 1) {
            Some(v) if !v.starts_with('-') => Ok(Some(v.clone())),
            _ => Err(format!("{} requires a value", names.join(" / "))),
        };
    }
    Ok(None)
}

/// Every value following any occurrence of a flag in `names` — for
/// repeatable flags like `--header`, in either form
/// (`--header=A: 1`, RFC 064 Amendment 1, works the same as
/// `--header "A: 1"`). Errors on the first occurrence with no value,
/// even if a later occurrence would have had one (RFC 064: a
/// partial-acceptance rule for a repeated flag is harder to explain
/// than "every occurrence needs a value", for no real benefit — the
/// whole invocation is still available to fix and retry).
pub(super) fn flag_values_all(args: &[String], names: &[&str]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if let Some((name, value)) = split_equals_form(a) {
            if names.contains(&name) {
                out.push(value.to_owned());
            }
            continue;
        }
        if names.iter().any(|n| a == n) {
            match args.get(i + 1) {
                Some(v) if !v.starts_with('-') => out.push(v.clone()),
                _ => return Err(format!("{} requires a value", names.join(" / "))),
            }
        }
    }
    Ok(out)
}

/// Whether any flag in `names` is present at all, regardless of value.
pub(super) fn flag_present(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|a| names.iter().any(|n| a == n))
}

/// Reject an explicit empty value (`--flag=`, RFC 064 Amendment 1) for
/// a **path-valued** flag — `--config`/`-c`, `--rule-set`/`-r`,
/// `--body-file`, `--file`. An empty value is meaningful for a
/// *content* flag (`--text=` is a real empty response body — RFC 064
/// Amendment 1 § 4 explicitly wants that kept), but never for one whose
/// value becomes a filesystem path: passed through, an empty path
/// fails downstream by naming an empty string rather than the flag
/// that produced it (`"workspace root '' is not a valid apimock
/// workspace"`, `"cannot read --body-file : No such file or
/// directory"`) — found in review of this amendment, the exact "blames
/// the file when the real problem is the value" shape RFC 064 exists
/// to delete. Checked at the same point `--status`/`--delay` already
/// validate their own values, in the same "`X` must be `Y`, got `Z`"
/// style.
pub(super) fn reject_empty_path_value(
    names: &[&str],
    value: Option<String>,
) -> Result<Option<String>, String> {
    match value {
        Some(v) if v.is_empty() => Err(format!(
            "{} must be a non-empty path, got ''",
            names.join(" / ")
        )),
        other => Ok(other),
    }
}

/// RFC 059: an unrecognised flag must be a `usage` error (exit 2) with a
/// near-match suggestion, for every command that takes flags. RFC 064
/// Amendment 1 folded `set`'s own private copy (which additionally
/// rejected *any* leftover token, not just a dash-prefixed one — `set`
/// has no positional arguments once its leading `rule` noun is
/// stripped, unlike `get`'s `<path>`) and the root command's own copy
/// into this one, so `--flag=value` handling exists in exactly one
/// place rather than being added to three parsers separately — the
/// same kind of divergence that caused RFC 064's original Defect 1.
///
/// `known` is every flag name (all aliases) this command recognises;
/// `no_value` is the subset that takes no value, so this never disagrees
/// with how the command's own parser decides whether the next token is a
/// value or the start of the next flag.
///
/// `strict_bare_tokens`: when `true`, *any* token that isn't a
/// recognised flag (or a value consumed by one) is an error, even if it
/// doesn't start with `-` — `set`'s own pre-existing behaviour,
/// preserved exactly rather than loosened by consolidation. `false` for
/// every other caller, which either have a real positional argument
/// (`get`'s `<path>`) or simply never had this stricter check and are
/// left exactly as they were.
///
/// `unmatched_phrase`: the lead phrase used when no near-match
/// suggestion exists (`"unrecognized argument"` for
/// `get`/`validate`/`match-test`/`set`, `"unknown option"` for the root
/// command) — the one piece of wording that already differed between
/// the root command and everyone else before this amendment, pinned by
/// existing tests on both sides, so consolidating the *detection* logic
/// must not also collapse that difference. The "did you mean" branch is
/// identical everywhere and is not parameterised.
pub(crate) fn reject_unknown_flags(
    args: &[String],
    known: &[&str],
    no_value: &[&str],
    strict_bare_tokens: bool,
    unmatched_phrase: &'static str,
) -> Result<(), String> {
    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if let Some((name, _value)) = split_equals_form(arg) {
            if !known.contains(&name) {
                return Err(match crate::args::near_match(name, known) {
                    Some(suggestion) => {
                        format!("unknown option '{}'; did you mean '{}'?", arg, suggestion)
                    }
                    None => format!("{} '{}'", unmatched_phrase, arg),
                });
            }
            // RFC 064 Amendment 1 § 2 — a hard acceptance gate: a
            // no-value (boolean) flag given *any* `=` form is always a
            // usage error. `=true` exactly like `=false` and `=` —
            // never "present". This is what keeps `--allow-outside=false`
            // from being silently read as `--allow-outside` present
            // (RFC 062's write-path confinement staying OFF while the
            // caller wrote `false` to keep it on).
            if no_value.contains(&name) {
                return Err(format!("{} does not take a value", name));
            }
            // Known, value-taking flag in `=` form: the value is
            // embedded in this one token: nothing to skip.
            continue;
        }
        if known.contains(&arg.as_str()) {
            if !no_value.contains(&arg.as_str()) {
                skip_next = args.get(i + 1).is_some_and(|next| !next.starts_with('-'));
            }
            continue;
        }
        if arg.starts_with('-') || strict_bare_tokens {
            return Err(match crate::args::near_match(arg, known) {
                Some(suggestion) => {
                    format!("unknown option '{}'; did you mean '{}'?", arg, suggestion)
                }
                None => format!("{} '{}'", unmatched_phrase, arg),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_value_parses_correctly() {
        let args: Vec<String> = ["--rule-set", "foo.toml", "--path", "/api"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            flag_value(&args, &["--rule-set", "-r"]).unwrap().as_deref(),
            Some("foo.toml")
        );
        assert_eq!(
            flag_value(&args, &["--path", "-p"]).unwrap().as_deref(),
            Some("/api")
        );
        assert_eq!(flag_value(&args, &["--method", "-m"]).unwrap(), None);
    }

    #[test]
    fn flag_value_errors_when_dangling_at_end_of_args() {
        let args: Vec<String> = ["--path", "/api", "--format"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = flag_value(&args, &["--format"]).unwrap_err();
        assert!(err.contains("--format"), "message was: {err}");
    }

    #[test]
    fn flag_value_errors_when_immediately_followed_by_another_flag() {
        let args: Vec<String> = ["--format", "--why"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = flag_value(&args, &["--format"]).unwrap_err();
        assert!(err.contains("--format"), "message was: {err}");
    }

    #[test]
    fn flag_values_all_collects_multiple() {
        let args: Vec<String> = [
            "--header",
            "Content-Type: application/json",
            "--header",
            "X-Api-Key: secret",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let vals = flag_values_all(&args, &["--header", "-H"]).unwrap();
        assert_eq!(vals.len(), 2);
        assert!(vals[0].contains("Content-Type"));
    }

    #[test]
    fn flag_values_all_errors_on_a_dangling_occurrence_even_after_a_valid_one() {
        let args: Vec<String> = ["--header", "Content-Type: application/json", "--header"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let err = flag_values_all(&args, &["--header", "-H"]).unwrap_err();
        assert!(err.contains("--header"), "message was: {err}");
    }

    #[test]
    fn flag_present_detects_boolean_flags() {
        let args: Vec<String> = vec!["--quiet".to_owned()];
        assert!(flag_present(&args, &["--quiet", "-q"]));
        assert!(!flag_present(&args, &["--why"]));
    }

    #[test]
    fn reject_empty_path_value_rejects_an_explicit_empty_string() {
        let err = reject_empty_path_value(&["--config", "-c"], Some(String::new())).unwrap_err();
        assert!(err.contains("--config / -c"), "message was: {err}");
        assert!(err.contains("non-empty path"), "message was: {err}");
    }

    #[test]
    fn reject_empty_path_value_passes_through_a_non_empty_value() {
        assert_eq!(
            reject_empty_path_value(&["--config"], Some("apimock.toml".to_owned())).unwrap(),
            Some("apimock.toml".to_owned())
        );
    }

    #[test]
    fn reject_empty_path_value_passes_through_absent() {
        assert_eq!(reject_empty_path_value(&["--config"], None).unwrap(), None);
    }

    // ── RFC 064 Amendment 1: `--flag=value` ────────────────────────────

    #[test]
    fn split_equals_form_splits_at_the_first_equals_only() {
        assert_eq!(
            split_equals_form("--json={\"a\":\"b=c\"}"),
            Some(("--json", "{\"a\":\"b=c\"}"))
        );
        assert_eq!(split_equals_form("--text="), Some(("--text", "")));
        assert_eq!(
            split_equals_form("-c=./apimock.toml"),
            Some(("-c", "./apimock.toml"))
        );
    }

    #[test]
    fn split_equals_form_ignores_tokens_not_starting_with_a_dash() {
        // A positional value legitimately containing `=` must never be
        // mistaken for a flag token.
        assert_eq!(split_equals_form("/a?x=1&y=2"), None);
        assert_eq!(split_equals_form("Authorization: Basic YWJj=="), None);
    }

    #[test]
    fn split_equals_form_returns_none_without_an_equals_sign() {
        assert_eq!(split_equals_form("--text"), None);
        assert_eq!(split_equals_form("-c"), None);
    }

    #[test]
    fn flag_value_reads_the_equals_form() {
        let args: Vec<String> = ["--config=./apimock.toml"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            flag_value(&args, &["--config", "-c"]).unwrap().as_deref(),
            Some("./apimock.toml")
        );
    }

    #[test]
    fn flag_value_equals_form_accepts_a_value_starting_with_a_dash() {
        let args: Vec<String> = ["--text=-hello"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            flag_value(&args, &["--text"]).unwrap().as_deref(),
            Some("-hello")
        );
    }

    #[test]
    fn flag_value_equals_form_empty_value_is_explicit_empty_not_dangling() {
        let args: Vec<String> = ["--text="].iter().map(|s| s.to_string()).collect();
        assert_eq!(flag_value(&args, &["--text"]).unwrap().as_deref(), Some(""));
    }

    #[test]
    fn flag_values_all_collects_equals_form_occurrences() {
        let args: Vec<String> = ["--header=A: 1", "--header=B: 2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let vals = flag_values_all(&args, &["--header", "-H"]).unwrap();
        assert_eq!(vals, vec!["A: 1".to_owned(), "B: 2".to_owned()]);
    }

    #[test]
    fn reject_unknown_flags_rejects_a_no_value_flag_given_equals_true() {
        let args: Vec<String> = vec!["--allow-outside=true".to_owned()];
        let err = reject_unknown_flags(
            &args,
            &["--allow-outside"],
            &["--allow-outside"],
            true,
            "unrecognized argument",
        )
        .unwrap_err();
        assert!(err.contains("--allow-outside"), "message was: {err}");
    }

    #[test]
    fn reject_unknown_flags_rejects_a_no_value_flag_given_equals_false() {
        let args: Vec<String> = vec!["--allow-outside=false".to_owned()];
        let err = reject_unknown_flags(
            &args,
            &["--allow-outside"],
            &["--allow-outside"],
            true,
            "unrecognized argument",
        )
        .unwrap_err();
        assert!(err.contains("--allow-outside"), "message was: {err}");
    }

    #[test]
    fn reject_unknown_flags_rejects_a_no_value_flag_given_equals_empty() {
        let args: Vec<String> = vec!["--dry-run=".to_owned()];
        let err = reject_unknown_flags(
            &args,
            &["--dry-run"],
            &["--dry-run"],
            true,
            "unrecognized argument",
        )
        .unwrap_err();
        assert!(err.contains("--dry-run"), "message was: {err}");
    }

    #[test]
    fn reject_unknown_flags_accepts_a_value_flag_in_equals_form() {
        let args: Vec<String> = vec!["--config=./apimock.toml".to_owned()];
        assert!(
            reject_unknown_flags(&args, &["--config"], &[], false, "unrecognized argument").is_ok()
        );
    }

    #[test]
    fn reject_unknown_flags_unknown_equals_form_still_suggests_a_near_match() {
        let args: Vec<String> = vec!["--txt=x".to_owned()];
        let err = reject_unknown_flags(&args, &["--text"], &[], false, "unrecognized argument")
            .unwrap_err();
        assert!(err.contains("--txt=x"), "message was: {err}");
        assert!(err.contains("--text"), "message was: {err}");
    }

    #[test]
    fn reject_unknown_flags_strict_bare_tokens_rejects_a_leftover_non_dash_argument() {
        let args: Vec<String> = vec!["garbage".to_owned()];
        assert!(reject_unknown_flags(&args, &[], &[], true, "unrecognized argument").is_err());
    }

    #[test]
    fn reject_unknown_flags_non_strict_allows_a_leftover_non_dash_argument() {
        // `get`'s positional `<path>` relies on this: a bare token that
        // isn't a known flag must not be rejected here.
        let args: Vec<String> = vec!["/x".to_owned()];
        assert!(reject_unknown_flags(&args, &[], &[], false, "unrecognized argument").is_ok());
    }
}
