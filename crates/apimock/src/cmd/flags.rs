//! Minimal flag-parsing helpers shared by `match-test` and `get` (RFC 055).
//!
//! Extracted rather than duplicated a second time — `match-test` already
//! had its own private copies before `get` needed the identical logic.

/// The value following the first flag in `names` that appears in
/// `args`, if that value doesn't itself look like another flag.
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
    let Some(idx) = args.iter().position(|a| names.iter().any(|n| a == n)) else {
        return Ok(None);
    };
    match args.get(idx + 1) {
        Some(v) if !v.starts_with('-') => Ok(Some(v.clone())),
        _ => Err(format!("{} requires a value", names.join(" / "))),
    }
}

/// Every value following any occurrence of a flag in `names` — for
/// repeatable flags like `--header`. Errors on the first occurrence
/// with no value, even if a later occurrence would have had one
/// (RFC 064: a partial-acceptance rule for a repeated flag is harder
/// to explain than "every occurrence needs a value", for no real
/// benefit — the whole invocation is still available to fix and retry).
pub(super) fn flag_values_all(args: &[String], names: &[&str]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for (i, a) in args.iter().enumerate() {
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

/// RFC 059: an unrecognised flag must be a `usage` error (exit 2) with a
/// near-match suggestion, for every command that takes flags — not just
/// `set`, which already had this check (kept as its own copy; it predates
/// this one and there was no reason to touch working code to consolidate
/// it). `get`, `validate` and `match-test` share this one instead of each
/// growing its own private copy a fourth, fifth and sixth time.
///
/// `known` is every flag name (all aliases) this command recognises;
/// `no_value` is the subset that takes no value, so this never disagrees
/// with how the command's own parser decides whether the next token is a
/// value or the start of the next flag.
pub(super) fn reject_unknown_flags(
    args: &[String],
    known: &[&str],
    no_value: &[&str],
) -> Result<(), String> {
    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if known.contains(&arg.as_str()) {
            if !no_value.contains(&arg.as_str()) {
                skip_next = args.get(i + 1).is_some_and(|next| !next.starts_with('-'));
            }
            continue;
        }
        if arg.starts_with('-') {
            return Err(match crate::args::near_match(arg, known) {
                Some(suggestion) => {
                    format!("unknown option '{}'; did you mean '{}'?", arg, suggestion)
                }
                None => format!("unrecognized argument '{}'", arg),
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
}
