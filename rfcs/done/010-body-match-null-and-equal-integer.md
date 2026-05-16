# RFC 010 — Body match semantics: null/Exists clarification and equal_integer operator

**Status.** Implemented (v5.9.0)
**Tracks.** RFC 008 follow-up — resolving two open questions from
the body match language extension: (1) the treatment of JSON `null`
values under `Exists`/`Absent`, and (2) exact integer matching via
a new `equal_integer` operator that avoids `f64` precision loss.
**Touches.** `apimock-routing` (`body_operator.rs`, `body/tests.rs`),
`apimock-config` (`view.rs` — `BodyOp` enum), documentation.

## Summary

RFC 008 shipped 17 body match operators but left two edge cases
explicitly unresolved:

1. **`null` and `Exists`/`Absent`** (Unresolved §1): a JSON field
   that exists but holds `null` — does `Exists` match? The RFC
   said "the intuitive answer is yes, document and test" without
   doing so.
2. **`equal_integer`** (Future possibilities): integers above
   `2^53` lose precision when coerced through `f64`. RFC 008 noted
   "probably yes as a follow-up" for a dedicated i64 operator.

Both are small, independent additions with clear semantics. Bundling
them into one RFC keeps the RFC count down while addressing the
remaining body-language gaps before users encounter silent mismatch
bugs.

## Motivation

### null and Exists

Consider this rule:

```toml
[when.request.body.json]
"user_id" = { op = "exists" }
```

If the request body is `{"user_id": null}`, the current
implementation calls `json_value_by_jsonpath` which returns
`Some(Value::Null)`. `BodyOperator::Exists` then calls
`op.is_match(&Value::Null, "")` which returns `true` — but this
is implicit, not documented or tested. A future refactor that
returns `None` for null paths would silently break this rule.

The *intuitive* semantic — **a field that exists with value `null`
counts as Exists** — must be pinned with a test and a doc note.
The inverse, `Absent`, should match only when the path is truly
absent (not present-but-null). This also needs a test.

### equal_integer

`BodyOperator::EqualNumber` uses `f64` internally:

```
body.json."order_id" = { op = "equal_number", value = "9007199254740993" }
```

`9007199254740993` is `2^53 + 1`. When coerced to `f64`, it becomes
`9007199254740992.0` (precision loss). The rule silently matches
a different integer than intended — a subtle data-correctness bug
for services that use large integer IDs.

A dedicated `equal_integer` operator using `i64` arithmetic avoids
this. For most practical rules the difference is irrelevant, but
for services with snowflake IDs or database primary keys exceeding
`2^53`, it is essential.

## Guide-level explanation

### null semantics (clarification, no TOML change)

The routing crate's documented contract becomes:

> `Exists` matches when `json_value_by_jsonpath` returns `Some(_)`,
> **including `Some(Value::Null)`**. A field present with value
> `null` satisfies `Exists`. Use `Absent` to require that the
> field is not present at all.

No TOML or API change — this is a documentation and test
clarification of existing behaviour.

### equal_integer

New operator in TOML:

```toml
[when.request.body.json]
"order_id" = { op = "equal_integer", value = "9007199254740993" }
```

Match semantics:

- The JSON value at `path` is extracted.
- If it is a JSON `Number` that is an exact integer, compare as
  `i64`.
- If it is a JSON `String` that parses as `i64`, compare as `i64`.
- Otherwise: no match (false).
- The configured `value` string must parse as `i64`; a
  non-integer `value` is a validation error at load time.

`equal_integer` is strictly about **exact equality**. Inequality
comparisons (`greater_than`, etc.) remain `f64`-based — the
precision concern only manifests when equality is required.

## Reference-level explanation

### BodyOperator addition

```rust
pub enum BodyOperator {
    // … existing variants …

    /// Exact integer equality using i64 arithmetic.
    /// Avoids f64 precision loss for integers above 2^53.
    EqualInteger,
}
```

`is_match` implementation:

```rust
Self::EqualInteger => {
    let lhs: i64 = match resolved {
        Value::Number(n) => match n.as_i64() {
            Some(i) => i,
            None => return false,   // float or out-of-i64-range
        },
        Value::String(s) => match s.parse::<i64>() {
            Ok(i) => i,
            Err(_) => return false,
        },
        _ => return false,
    };
    let rhs: i64 = match configured_value.parse::<i64>() {
        Ok(i) => i,
        Err(_) => return false,
    };
    lhs == rhs
}
```

### Null/Exists test additions (`body/tests.rs`)

```rust
#[test]
fn exists_matches_null_value() {
    // A field present with value null satisfies Exists.
    let body = parse_body(r#"json."user_id" = { op = "exists", value = "" }"#);
    let req = make_parsed_request(Some(json!({"user_id": null})));
    assert!(body.is_match(&req));
}

#[test]
fn absent_does_not_match_null_value() {
    // Absent requires the field to be truly missing, not null.
    let body = parse_body(r#"json."user_id" = { op = "absent", value = "" }"#);
    let req = make_parsed_request(Some(json!({"user_id": null})));
    assert!(!body.is_match(&req));
}

#[test]
fn absent_matches_missing_field() {
    let body = parse_body(r#"json."missing" = { op = "absent", value = "" }"#);
    let req = make_parsed_request(Some(json!({"present": 1})));
    assert!(body.is_match(&req));
}
```

### equal_integer test additions

```rust
#[test]
fn equal_integer_exact_large_int() {
    // 2^53 + 1 — would lose precision as f64.
    let body = parse_body(
        r#"json."id" = { op = "equal_integer", value = "9007199254740993" }"#
    );
    let req = make_parsed_request(Some(json!({"id": 9007199254740993i64})));
    assert!(body.is_match(&req));
}

#[test]
fn equal_integer_no_match_adjacent_int() {
    let body = parse_body(
        r#"json."id" = { op = "equal_integer", value = "9007199254740993" }"#
    );
    // 9007199254740992 — one less
    let req = make_parsed_request(Some(json!({"id": 9007199254740992i64})));
    assert!(!body.is_match(&req));
}

#[test]
fn equal_integer_rejects_float_json_value() {
    let body = parse_body(r#"json."x" = { op = "equal_integer", value = "42" }"#);
    let req = make_parsed_request(Some(json!({"x": 42.5})));
    assert!(!body.is_match(&req));
}
```

### Config crate (`BodyOp` enum)

`BodyOp::EqualInteger` added. The payload→routing conversion maps
it to `BodyOperator::EqualInteger`.

### Validation

At config load time, for any body condition using `equal_integer`,
validate that the configured `value` parses as `i64`. Return a
`ValidationIssue` if it does not, before the server starts.

### Documentation

`body_operator.rs` doc block updated with a "Null values" section
explicitly stating the `Exists`/`Absent` contract for `null`.

## Drawbacks

1. **One more operator.** The operator list grows from 17 to 18.
   Each addition raises the cognitive load of the docs.
2. **`equal_integer` does not cover `u64`.** Unsigned 64-bit
   integers above `i64::MAX` are not representable. This is an
   edge case in JSON APIs; if it arises, a follow-up
   `equal_unsigned_integer` can handle it.

## Unresolved questions

1. **`equal_integer` on a JSON Number that is a valid float but
   not an exact integer** (e.g. `42.0`): should `serde_json`'s
   `as_i64()` succeed (it does for `42.0` on most
   implementations) or should we require the number to have no
   fractional part? Recommendation: accept `42.0` as `42` —
   `as_i64()` semantics already handle this.
2. **String `value` coercion in `equal_integer`:** `"042"` with a
   leading zero — parse as `i64` (42) or reject? Recommendation:
   accept (standard `str::parse::<i64>` strips leading zeros).
