use serde::Deserialize;

/// Operators for body JSON conditions.
///
/// # String-style operators (5.7.0 baseline)
///
/// These are the original operators. [`Equal`] retains the 5.7.0
/// string-coercion behaviour for backwards compatibility — both the
/// JSON value at the dotted path and the configured `value` string are
/// coerced to strings before comparison. Use [`EqualString`] for an
/// explicit alias, or [`EqualTyped`] to require an exact JSON-type match.
///
/// # Numeric operators (RFC 008)
///
/// Coerce the JSON value at the path to an `f64`. A JSON `Number`
/// is used directly; a JSON `String` that parses as `f64` is also
/// accepted. Any other type — or a value that doesn't parse — causes
/// the condition to **not match** (returns `false`). The configured
/// `value` field undergoes the same coercion at match time.
///
/// # Type-aware equality (RFC 008)
///
/// [`EqualTyped`] matches only when the JSON value at the path is
/// *exactly* equal to the configured JSON value **including type**.
/// It distinguishes `42` (Number) from `"42"` (String). The configured
/// `value` string is parsed as JSON (`serde_json::from_str`) at match
/// time; a non-JSON `value` always fails.
///
/// # Presence operators (RFC 008)
///
/// [`Exists`] / [`Absent`] assert whether the dotted path resolves to
/// anything in the request JSON. The configured `value` field is
/// ignored for these operators.
///
/// # Array operators (RFC 008)
///
/// Require the value at the path to be a JSON array.
/// - [`ArrayLengthEqual`] / [`ArrayLengthAtLeast`]: compare array
///   length against the configured value (parsed as a non-negative
///   integer).
/// - [`ArrayContains`]: checks whether any element in the array equals
///   the configured value (parsed as JSON for typed comparison).
#[derive(Clone, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BodyOperator {
    // ── string-style (baseline) ─────────────────────────────────────
    /// String-coercion equality. Alias: [`EqualString`]. Kept for
    /// backwards compatibility with 5.7.0 rule files.
    #[default]
    Equal,
    /// Explicit alias for [`Equal`]. Prefer this in new rules for
    /// clarity when numeric or typed operators are also present.
    EqualString,
    /// Substring check (string-coercion semantics).
    Contains,
    /// Inverse substring check (string-coercion semantics).
    NotContains,
    /// Prefix check (string-coercion semantics).
    StartsWith,
    /// Inverse prefix check.
    NotStartsWith,
    /// Suffix check (string-coercion semantics).
    EndsWith,
    /// Inverse suffix check.
    NotEndsWith,
    /// Regex match (string-coercion semantics).
    Regex,
    /// Inverse regex match.
    NotRegex,

    // ── type-aware equality (RFC 008) ────────────────────────────────
    /// Exact JSON-type + value equality. Distinguishes `42` from `"42"`.
    EqualTyped,

    // ── numeric operators (RFC 008) ──────────────────────────────────
    /// Numeric equality (both sides coerced to `f64`).
    EqualNumber,
    /// Numeric greater-than.
    GreaterThan,
    /// Numeric less-than.
    LessThan,
    /// Numeric greater-than-or-equal.
    GreaterOrEqual,
    /// Numeric less-than-or-equal.
    LessOrEqual,

    // ── presence operators (RFC 008) ─────────────────────────────────
    /// Path resolves to a value (any type, including `null`).
    Exists,
    /// Path does not resolve to any value.
    Absent,

    // ── array operators (RFC 008) ────────────────────────────────────
    /// Value at path is an array whose length equals the configured value.
    ArrayLengthEqual,
    /// Value at path is an array whose length is ≥ the configured value.
    ArrayLengthAtLeast,
    /// Value at path is an array that contains the configured value
    /// (typed JSON equality).
    ArrayContains,

    // ── exact integer (RFC 010) ──────────────────────────────────────
    /// Exact integer equality using i64 arithmetic, avoiding f64 precision
    /// loss for integers above 2^53.
    EqualInteger,

    // ── object/map operators (RFC 022) ───────────────────────────────
    /// Value at path is a JSON object that contains the key named in
    /// the configured value string.
    MapHasKey,
    /// Value at path is a JSON object that does NOT contain the key
    /// named in the configured value string.
    MapDoesNotHaveKey,

    // ── structural operators (RFC 028) ────────────────────────────────
    /// Value at path is an array containing at least one element that is
    /// a *superset* of the configured JSON object. "Superset" means every
    /// key in the configured object is present in the element with equal
    /// value; the element may have additional keys.
    ///
    /// For non-object needle values, falls back to strict equality
    /// (identical to `ArrayContains`).
    StructuralContains,
}

impl std::fmt::Display for BodyOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => write!(f, " == "),
            Self::EqualString => write!(f, " == (string) "),
            Self::Contains => write!(f, " contains "),
            Self::NotContains => write!(f, " does not contain "),
            Self::StartsWith => write!(f, " starts_with "),
            Self::NotStartsWith => write!(f, " does not start_with "),
            Self::EndsWith => write!(f, " ends_with "),
            Self::NotEndsWith => write!(f, " does not end_with "),
            Self::Regex => write!(f, " matches regex "),
            Self::NotRegex => write!(f, " does not match regex "),
            Self::EqualTyped => write!(f, " == (typed) "),
            Self::EqualNumber => write!(f, " == (number) "),
            Self::GreaterThan => write!(f, " > "),
            Self::LessThan => write!(f, " < "),
            Self::GreaterOrEqual => write!(f, " >= "),
            Self::LessOrEqual => write!(f, " <= "),
            Self::Exists => write!(f, " exists"),
            Self::Absent => write!(f, " absent"),
            Self::ArrayLengthEqual => write!(f, " array_length == "),
            Self::ArrayLengthAtLeast => write!(f, " array_length >= "),
            Self::ArrayContains => write!(f, " array_contains "),
            Self::EqualInteger => write!(f, " == (integer) "),
            Self::MapHasKey => write!(f, " map_has_key "),
            Self::MapDoesNotHaveKey => write!(f, " map_does_not_have_key "),
            Self::StructuralContains => write!(f, " structural_contains "),
        }
    }
}

impl BodyOperator {
    /// Apply this operator to the resolved JSON value from the request
    /// body and the configured `value` string from the rule.
    ///
    /// Returns `true` iff the condition matches.
    pub fn is_match(&self, resolved: &serde_json::Value, configured_value: &str) -> bool {
        use serde_json::Value;

        match self {
            // ── string-style ─────────────────────────────────────────
            Self::Equal | Self::EqualString => {
                let lhs = value_as_string(resolved);
                lhs == configured_value
            }
            Self::Contains => value_as_string(resolved).contains(configured_value),
            Self::NotContains => !value_as_string(resolved).contains(configured_value),
            Self::StartsWith => value_as_string(resolved).starts_with(configured_value),
            Self::NotStartsWith => !value_as_string(resolved).starts_with(configured_value),
            Self::EndsWith => value_as_string(resolved).ends_with(configured_value),
            Self::NotEndsWith => !value_as_string(resolved).ends_with(configured_value),
            Self::Regex => {
                let text = value_as_string(resolved);
                regex_is_match(configured_value, &text)
            }
            Self::NotRegex => {
                let text = value_as_string(resolved);
                !regex_is_match(configured_value, &text)
            }

            // ── type-aware equality ──────────────────────────────────
            Self::EqualTyped => {
                let expected: Value = match serde_json::from_str(configured_value) {
                    Ok(v) => v,
                    Err(_) => return false,
                };
                resolved == &expected
            }

            // ── numeric operators ────────────────────────────────────
            Self::EqualNumber => match (to_f64(resolved), parse_f64(configured_value)) {
                (Some(l), Some(r)) => (l - r).abs() < f64::EPSILON,
                _ => false,
            },
            Self::GreaterThan => match (to_f64(resolved), parse_f64(configured_value)) {
                (Some(l), Some(r)) => l > r,
                _ => false,
            },
            Self::LessThan => match (to_f64(resolved), parse_f64(configured_value)) {
                (Some(l), Some(r)) => l < r,
                _ => false,
            },
            Self::GreaterOrEqual => match (to_f64(resolved), parse_f64(configured_value)) {
                (Some(l), Some(r)) => l >= r,
                _ => false,
            },
            Self::LessOrEqual => match (to_f64(resolved), parse_f64(configured_value)) {
                (Some(l), Some(r)) => l <= r,
                _ => false,
            },

            // ── presence ────────────────────────────────────────────
            // For Exists/Absent the call site checks path resolution;
            // this variant is reached only when the path resolved
            // (Exists → match, Absent → no match).
            Self::Exists => true,
            Self::Absent => false,

            // ── array operators ──────────────────────────────────────
            Self::ArrayLengthEqual => match resolved {
                Value::Array(arr) => parse_usize(configured_value) == Some(arr.len()),
                _ => false,
            },
            Self::ArrayLengthAtLeast => match resolved {
                Value::Array(arr) => parse_usize(configured_value).is_some_and(|n| arr.len() >= n),
                _ => false,
            },
            Self::ArrayContains => match resolved {
                Value::Array(arr) => {
                    let expected: Value = match serde_json::from_str(configured_value) {
                        Ok(v) => v,
                        // fall back to string comparison
                        Err(_) => Value::String(configured_value.to_owned()),
                    };
                    arr.contains(&expected)
                }
                _ => false,
            },

            // ── exact integer (RFC 010) ──────────────────────────────────
            Self::EqualInteger => {
                let lhs: i64 = match resolved {
                    Value::Number(n) => match n.as_i64() {
                        Some(i) => i,
                        None => return false,
                    },
                    Value::String(s) => match s.parse::<i64>() {
                        Ok(i) => i,
                        Err(_) => return false,
                    },
                    _ => return false,
                };
                match configured_value.parse::<i64>() {
                    Ok(rhs) => lhs == rhs,
                    Err(_) => false,
                }
            }

            // ── map/object operators (RFC 022) ────────────────────────────
            Self::MapHasKey => match resolved {
                Value::Object(map) => map.contains_key(configured_value),
                _ => false,
            },
            Self::MapDoesNotHaveKey => match resolved {
                Value::Object(map) => !map.contains_key(configured_value),
                _ => false,
            },

            // ── structural operators (RFC 028) ────────────────────────────
            Self::StructuralContains => {
                let needle: Value = serde_json::from_str(configured_value)
                    .unwrap_or_else(|_| Value::String(configured_value.to_owned()));
                match resolved {
                    Value::Array(arr) => arr.iter().any(|el| is_subset(&needle, el)),
                    _ => false,
                }
            }
        }
    }
}

// ── helpers ──────────────────────────────────────────────────────────

fn value_as_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn to_f64(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

fn parse_f64(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}

fn parse_usize(s: &str) -> Option<usize> {
    s.parse::<usize>().ok()
}

fn regex_is_match(pattern: &str, text: &str) -> bool {
    // The `regex` crate is a direct dependency of `apimock-routing` since
    // RFC 017. Compile the regex on every call; cache if profiling shows
    // this is a bottleneck.
    regex::Regex::new(pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Returns `true` when every (key, value) pair in `needle` is present
/// in `haystack` with an equal value. Recursively applied to nested
/// objects. Non-object needles fall back to strict equality.
/// Used by `StructuralContains` (RFC 028).
fn is_subset(needle: &serde_json::Value, haystack: &serde_json::Value) -> bool {
    use serde_json::Value;
    match needle {
        Value::Object(needle_map) => match haystack {
            Value::Object(haystack_map) => needle_map
                .iter()
                .all(|(k, v)| haystack_map.get(k).is_some_and(|hv| is_subset(v, hv))),
            _ => false,
        },
        // For non-object needles, strict equality.
        other => other == haystack,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn equal_string_coercion() {
        let op = BodyOperator::Equal;
        assert!(op.is_match(&json!("hello"), "hello"));
        assert!(op.is_match(&json!(42), "42"));
        assert!(!op.is_match(&json!("hello"), "world"));
    }

    #[test]
    fn equal_string_explicit() {
        let op = BodyOperator::EqualString;
        assert!(op.is_match(&json!("hello"), "hello"));
        assert!(op.is_match(&json!(42), "42"));
    }

    #[test]
    fn equal_typed_distinguishes_types() {
        let op = BodyOperator::EqualTyped;
        assert!(op.is_match(&json!(42), "42"));
        assert!(!op.is_match(&json!("42"), "42")); // string vs number
        assert!(op.is_match(&json!("42"), "\"42\"")); // string == string
        assert!(op.is_match(&json!(true), "true"));
    }

    #[test]
    fn numeric_operators() {
        assert!(BodyOperator::EqualNumber.is_match(&json!(42), "42"));
        assert!(BodyOperator::GreaterThan.is_match(&json!(43), "42"));
        assert!(!BodyOperator::GreaterThan.is_match(&json!(41), "42"));
        assert!(BodyOperator::LessThan.is_match(&json!(41), "42"));
        assert!(BodyOperator::GreaterOrEqual.is_match(&json!(42), "42"));
        assert!(BodyOperator::LessOrEqual.is_match(&json!(42), "42"));
    }

    #[test]
    fn numeric_string_coercion() {
        // A JSON string that parses as a number should work with numeric ops.
        assert!(BodyOperator::EqualNumber.is_match(&json!("42"), "42"));
        assert!(BodyOperator::GreaterThan.is_match(&json!("100"), "42"));
    }

    #[test]
    fn numeric_non_number_returns_false() {
        assert!(!BodyOperator::GreaterThan.is_match(&json!("hello"), "42"));
        assert!(!BodyOperator::EqualNumber.is_match(&json!(null), "0"));
    }

    #[test]
    fn exists_always_true() {
        // is_match for Exists is reached only when the path resolved.
        assert!(BodyOperator::Exists.is_match(&json!("anything"), "ignored"));
        assert!(BodyOperator::Exists.is_match(&json!(null), "ignored"));
    }

    #[test]
    fn absent_always_false() {
        assert!(!BodyOperator::Absent.is_match(&json!("anything"), "ignored"));
    }

    #[test]
    fn array_length_equal() {
        assert!(BodyOperator::ArrayLengthEqual.is_match(&json!([1, 2, 3]), "3"));
        assert!(!BodyOperator::ArrayLengthEqual.is_match(&json!([1, 2]), "3"));
        assert!(!BodyOperator::ArrayLengthEqual.is_match(&json!("not_array"), "1"));
    }

    #[test]
    fn array_length_at_least() {
        assert!(BodyOperator::ArrayLengthAtLeast.is_match(&json!([1, 2, 3]), "3"));
        assert!(BodyOperator::ArrayLengthAtLeast.is_match(&json!([1, 2, 3, 4]), "3"));
        assert!(!BodyOperator::ArrayLengthAtLeast.is_match(&json!([1, 2]), "3"));
    }

    #[test]
    fn array_contains() {
        assert!(BodyOperator::ArrayContains.is_match(&json!([1, 2, 3]), "2"));
        assert!(!BodyOperator::ArrayContains.is_match(&json!([1, 2, 3]), "4"));
        assert!(BodyOperator::ArrayContains.is_match(&json!(["a", "b"]), "\"a\""));
        assert!(!BodyOperator::ArrayContains.is_match(&json!("not_array"), "1"));
    }

    // ── RFC 010: equal_integer ────────────────────────────────────────

    #[test]
    fn equal_integer_normal() {
        assert!(BodyOperator::EqualInteger.is_match(&json!(42), "42"));
        assert!(!BodyOperator::EqualInteger.is_match(&json!(42), "43"));
    }

    #[test]
    fn equal_integer_large_value_above_f64_precision() {
        // 2^53 + 1 — would lose precision as f64.
        let large = 9_007_199_254_740_993i64;
        let v = serde_json::Value::Number(serde_json::Number::from(large));
        assert!(
            BodyOperator::EqualInteger.is_match(&v, "9007199254740993"),
            "large integer must match exactly"
        );
        // One less should NOT match.
        assert!(
            !BodyOperator::EqualInteger.is_match(&v, "9007199254740992"),
            "adjacent integer must not match"
        );
    }

    #[test]
    fn equal_integer_string_value_in_body() {
        // JSON string that parses as i64 is accepted.
        assert!(BodyOperator::EqualInteger.is_match(&json!("42"), "42"));
        assert!(!BodyOperator::EqualInteger.is_match(&json!("42"), "99"));
    }

    #[test]
    fn equal_integer_rejects_float() {
        assert!(!BodyOperator::EqualInteger.is_match(&json!(42.5), "42"));
    }

    #[test]
    fn equal_integer_rejects_non_numeric_json() {
        assert!(!BodyOperator::EqualInteger.is_match(&json!("hello"), "42"));
        assert!(!BodyOperator::EqualInteger.is_match(&json!(null), "0"));
        assert!(!BodyOperator::EqualInteger.is_match(&json!(true), "1"));
    }

    #[test]
    fn equal_integer_invalid_configured_value_returns_false() {
        // A non-integer configured value never matches (no panic).
        assert!(!BodyOperator::EqualInteger.is_match(&json!(42), "not_a_number"));
        assert!(!BodyOperator::EqualInteger.is_match(&json!(42), "42.5"));
    }
}
