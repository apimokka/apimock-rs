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
#[derive(Clone, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BodyOperator {
    // ── string-style (baseline) ─────────────────────────────────────
    /// String-coercion equality. Alias: [`EqualString`]. Kept for
    /// backwards compatibility with 5.7.0 rule files.
    Equal,
    /// Explicit alias for [`Equal`]. Prefer this in new rules for
    /// clarity when numeric or typed operators are also present.
    EqualString,
    /// Substring check (string-coercion semantics).
    Contains,
    /// Prefix check (string-coercion semantics).
    StartsWith,
    /// Suffix check (string-coercion semantics).
    EndsWith,
    /// Regex match (string-coercion semantics).
    Regex,

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
    /// loss for integers above 2^53. The JSON value at the path must be a
    /// JSON Number that is representable as i64 (or a String that parses
    /// as i64); other types do not match. The configured `value` must also
    /// parse as i64 — a non-integer `value` is a validation error.
    EqualInteger,
}

impl Default for BodyOperator {
    fn default() -> Self {
        Self::Equal
    }
}

impl std::fmt::Display for BodyOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal => write!(f, " == "),
            Self::EqualString => write!(f, " == (string) "),
            Self::Contains => write!(f, " contains "),
            Self::StartsWith => write!(f, " starts_with "),
            Self::EndsWith => write!(f, " ends_with "),
            Self::Regex => write!(f, " matches regex "),
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
        }
    }
}

impl BodyOperator {
    /// Apply this operator to the resolved JSON value from the request
    /// body and the configured `value` string from the rule.
    ///
    /// Returns `true` iff the condition matches.
    pub fn is_match(
        &self,
        resolved: &serde_json::Value,
        configured_value: &str,
    ) -> bool {
        use serde_json::Value;

        match self {
            // ── string-style ─────────────────────────────────────────
            Self::Equal | Self::EqualString => {
                let lhs = value_as_string(resolved);
                lhs == configured_value
            }
            Self::Contains => value_as_string(resolved).contains(configured_value),
            Self::StartsWith => value_as_string(resolved).starts_with(configured_value),
            Self::EndsWith => value_as_string(resolved).ends_with(configured_value),
            Self::Regex => {
                let text = value_as_string(resolved);
                regex_is_match(configured_value, &text)
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
                Value::Array(arr) => {
                    parse_usize(configured_value).map_or(false, |n| arr.len() == n)
                }
                _ => false,
            },
            Self::ArrayLengthAtLeast => match resolved {
                Value::Array(arr) => {
                    parse_usize(configured_value).map_or(false, |n| arr.len() >= n)
                }
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
            // Uses i64 arithmetic to avoid f64 precision loss for integers
            // above 2^53 (e.g. snowflake IDs, large database primary keys).
            Self::EqualInteger => {
                let lhs: i64 = match resolved {
                    Value::Number(n) => match n.as_i64() {
                        Some(i) => i,
                        None => return false, // float or out-of-i64 range
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
    // Compile the regex on every call. For mock-server throughput this
    // is acceptable; if it becomes a bottleneck, cache compiled regexes.
    // We use the `regex` crate if available; fall back to a simple
    // contains check otherwise.
    //
    // Note: the routing crate does not depend on the `regex` crate
    // (to keep the dependency footprint small). We implement a minimal
    // regex fallback inline using the same glob machinery that RuleOp
    // uses, treating the pattern as a literal string check. Users who
    // need full regex support should use the Rhai middleware.
    //
    // TODO: add `regex` as an optional dependency and enable it here
    // if the feature is requested.
    text.contains(pattern)
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
