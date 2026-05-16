# RFC 013 — RulePayload validation: url_path / url_path_op consistency

**Status.** Implemented (v5.9.0)
**Tracks.** RFC 001 follow-up — resolving Unresolved §3: what does
`url_path: None, url_path_op: Some(_)` mean at the payload layer,
and how should it be rejected cleanly rather than silently ignored.
**Touches.** `apimock-config` (`workspace/validate.rs`,
`workspace/edit/payload.rs`), documentation.

## Summary

RFC 001 added `url_path_op: Option<UrlPathOp>` to `RulePayload` but
left the following case unresolved:

> **Empty / None value with operator set.** What does
> `url_path: None`, `url_path_op: Some(Contains)` mean?
> Probably an error at validation time. To be confirmed.

Currently `build_rule_from_payload` silently drops `url_path_op`
when `url_path` is `None`: the operator is ignored, no rule is
created for the URL-path field, and the caller gets no diagnostic.
This is a hidden misconfiguration footgun — the GUI submits a
payload expecting `Contains` semantics but the resulting rule has
no URL constraint at all.

This RFC specifies the validation contract and implements it.

## Motivation

Two real scenarios that currently silently misbehave:

**Scenario A — typo in GUI form:**
A GUI user types a URL path, then clears the field by accident but
leaves the operator dropdown set to `StartsWith`. The payload sends
`url_path: None, url_path_op: Some(StartsWith)`. The rule saves
with no URL constraint and matches every request — a significant
deviation from intent with no error reported.

**Scenario B — programmatic API consumer:**
A tool building payloads programmatically forgets to set `url_path`
but does set `url_path_op`. The `apply()` call succeeds silently,
and the tool has no way to know the operator was discarded.

Both scenarios are currently impossible to detect from the caller's
perspective. A validation error at `apply()` time surfaces the
problem immediately.

## Guide-level explanation

The following payload combinations are well-defined:

| `url_path` | `url_path_op` | Meaning |
|---|---|---|
| `None` | `None` | No URL constraint. Preserved from existing rule (UpdateRule) or absent (AddRule). |
| `Some(path)` | `None` | URL constraint with default `Equal` operator. |
| `Some(path)` | `Some(op)` | URL constraint with explicit operator. |
| `None` | `Some(op)` | **Invalid** — operator without a path. |

The last row is now a validation error returned as `ApplyError::InvalidPayload`.

```rust
// GUI code (before)
ws.apply(EditCommand::UpdateRule {
    id,
    rule: RulePayload {
        url_path: None,
        url_path_op: Some(UrlPathOp::StartsWith),  // ERROR: no path
        ..Default::default()
    },
})?;
// returns Err(ApplyError::InvalidPayload { reason: "url_path_op requires url_path to be Some" })
```

## Reference-level explanation

### Validation location

The check lives in `build_rule_from_payload` (the earliest point
after the payload shape is known):

```rust
if payload.url_path.is_none() && payload.url_path_op.is_some() {
    return Err(ApplyError::InvalidPayload {
        reason: "url_path_op requires url_path to be set \
                 (received url_path: None, url_path_op: Some(_))".to_owned(),
    });
}
```

This is a pre-condition check, not a post-hoc validation pass —
the error fires before any routing model is touched.

### Interaction with UpdateRule preservation semantics

The preservation rule (RFC 001) says: `url_path: None` → preserve
the existing rule's URL constraint. This is unaffected — the new
check fires only when `url_path_op` is also `Some`. If both are
`None`, the existing behaviour is unchanged.

Table of `UpdateRule` cases after this RFC:

| `url_path` | `url_path_op` | `UpdateRule` result |
|---|---|---|
| `None` | `None` | URL constraint preserved from existing rule. |
| `Some(path)` | `None` | URL constraint replaced with `Equal`. |
| `Some(path)` | `Some(op)` | URL constraint replaced with `op`. |
| `None` | `Some(op)` | `Err(InvalidPayload)` — new behaviour. |

For `AddRule` the same check applies; there is no existing rule to
preserve from, so the case is even more clearly wrong.

### Tests

1. **`apply_add_rule_url_path_op_without_path_is_error`** — Assert
   `AddRule` with `url_path: None, url_path_op: Some(StartsWith)`
   returns `Err` containing "url_path_op requires url_path".
2. **`apply_update_rule_url_path_op_without_path_is_error`** — Same
   for `UpdateRule`.
3. **`apply_add_rule_url_path_op_none_url_path_none_is_ok`** — Both
   `None` is not an error (URL constraint simply absent/preserved).
4. **`apply_add_rule_url_path_some_op_some_is_ok`** — Both set;
   assert the resulting rule's operator matches.

## Drawbacks

1. **Tightens a previously permissive API.** Any caller that was
   accidentally sending `url_path: None, url_path_op: Some(_)` and
   ignoring the (wrong) result will now get an error. This is
   intentional, but it is technically a behaviour change. Because
   5.8.0 is recent, the change is low-risk.

## Rationale and alternatives

**Alternative A: silently ignore `url_path_op` when `url_path` is
`None`.** Current behaviour. Produces silent misconfiguration.

**Alternative B: treat `url_path: None, url_path_op: Some(op)` as
"preserve path but replace operator".** More nuanced, but allows
the caller to change the operator without knowing the current path
value. Attractive at first glance but (a) the GUI should always
know the current path, and (b) it conflates two concerns in a
single field.

**Alternative C (this RFC): explicit error.** Fail loudly; let the
caller fix the payload. Consistent with Rust's "make invalid states
unrepresentable" ethos applied at the API boundary.

## Unresolved questions

1. **Should `UrlPathOp` be a separate type from the zero-case?**
   An alternative representation is `url_path: Option<UrlPathCondition>`
   where `UrlPathCondition { value: String, op: UrlPathOp }` makes
   the dependency structural rather than validated. This is RFC 001's
   "Alternative A" (rejected for backward-compat reasons in stage-2).
   If there is a 6.0.0 major bump, this refactor is worth revisiting.
