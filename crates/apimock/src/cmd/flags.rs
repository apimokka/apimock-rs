//! Minimal flag-parsing helpers shared by `match-test` and `get` (RFC 055).
//!
//! Extracted rather than duplicated a second time — `match-test` already
//! had its own private copies before `get` needed the identical logic.

/// The value following the first flag in `names` that appears in `args`,
/// if that value doesn't itself look like another flag.
pub(super) fn flag_value(args: &[String], names: &[&str]) -> Option<String> {
    let idx = args.iter().position(|a| names.iter().any(|n| a == n))?;
    args.get(idx + 1).filter(|v| !v.starts_with('-')).cloned()
}

/// Every value following any occurrence of a flag in `names` — for
/// repeatable flags like `--header`.
pub(super) fn flag_values_all(args: &[String], names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if names.iter().any(|n| a == n)
            && let Some(v) = args.get(i + 1)
            && !v.starts_with('-')
        {
            out.push(v.clone());
        }
    }
    out
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
            flag_value(&args, &["--rule-set", "-r"]).as_deref(),
            Some("foo.toml")
        );
        assert_eq!(
            flag_value(&args, &["--path", "-p"]).as_deref(),
            Some("/api")
        );
        assert_eq!(flag_value(&args, &["--method", "-m"]), None);
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
        let vals = flag_values_all(&args, &["--header", "-H"]);
        assert_eq!(vals.len(), 2);
        assert!(vals[0].contains("Content-Type"));
    }

    #[test]
    fn flag_present_detects_boolean_flags() {
        let args: Vec<String> = vec!["--quiet".to_owned()];
        assert!(flag_present(&args, &["--quiet", "-q"]));
        assert!(!flag_present(&args, &["--why"]));
    }
}
