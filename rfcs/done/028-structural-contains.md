# RFC 028 — StructuralContains body operator

**Status.** Implemented (v5.14.0)
**Tracks.** RFC 008 Future possibilities — "Structural contains for
object subset matching." `ArrayContains` checks for a value in an
array using strict JSON equality; it cannot match "array contains an
object that has at least these fields with these values".
**Touches.** `apimock-routing` (`BodyOperator`, body match logic,
tests), `apimock-config` (`BodyOp`, converter), documentation.

## Summary

`BodyOperator::StructuralContains` checks whether the value at the
configured path is an array that contains at least one element which
is a JSON object that is a *superset* of the configured object.
"Superset" means: every key in the configured object is present in
the array element and has an equal value; the element may have
additional keys.

## Motivation

```toml
# Match requests whose body.items array contains at least one
# element with type = "admin" (the element may have other fields).
when.request.body.json.items = { op = "structural_contains", value = "{\"type\":\"admin\"}" }
```

`ArrayContains` with `value = "{\"type\":\"admin\"}"` would check for
exact equality — an object that is *only* `{"type":"admin"}`. Any
element with additional fields would not match.

## Reference-level explanation

### `BodyOperator::StructuralContains`

```rust
StructuralContains,
```

Match logic:

```rust
BodyOperator::StructuralContains => {
    let needle: Value = serde_json::from_str(configured_value)
        .unwrap_or(Value::String(configured_value.to_owned()));
    match resolved {
        Value::Array(arr) => arr.iter().any(|el| is_subset(&needle, el)),
        _ => false,
    }
},
```

`is_subset(needle, haystack)`:
- If `needle` is an object: every (k, v) in `needle` must appear in
  `haystack` with equal value. Recursive for nested objects.
- Otherwise: strict equality (`needle == haystack`), making
  `StructuralContains` a superset of `ArrayContains` for scalars.

## Unresolved questions

None.
