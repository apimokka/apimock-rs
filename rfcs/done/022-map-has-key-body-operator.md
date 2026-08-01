# RFC 022 — MapHasKey / MapDoesNotHaveKey body operators

**Status.** Implemented (v5.12.0)
**Tracks.** RFC 008 Future possibilities — "MapHasKey for
object-keyed predicates."
**Touches.** `apimock-routing` (`BodyOperator`, body match logic,
tests), `apimock-config` (`BodyOp`, converter), documentation.

## Summary

`BodyOperator::MapHasKey` checks whether the JSON value at a given
dotted path is an object that contains a specific key. Its negation,
`MapDoesNotHaveKey`, checks that the object is missing the key.
These complement the existing `Exists`/`Absent` path-level operators:
`Exists` checks whether a path resolves to any value; `MapHasKey`
checks whether a specific child key exists within an already-resolved
object.

## Motivation

```toml
# Match only if the nested "permissions" object has an "admin" key.
when.request.body.json.user = { op = "map_has_key", value = "permissions" }
# (matches { "user": { "permissions": { "admin": true, ... }, ... } })

# Or the inverse:
when.request.body.json.config = { op = "map_does_not_have_key", value = "override" }
```

`Exists` on `user.permissions.admin` would also work for the first
case, but `MapHasKey` is cleaner when the key name is dynamic or
when you want to verify schema shape (e.g. "the response has a
metadata field but no internal_id field").

## Reference-level explanation

```rust
// BodyOperator new variants:
MapHasKey,
MapDoesNotHaveKey,
```

Match logic (in `body_operator.rs` `is_match`):

```rust
BodyOperator::MapHasKey => match resolved {
    Some(Value::Object(map)) => map.contains_key(configured_value),
    _ => false,
},
BodyOperator::MapDoesNotHaveKey => match resolved {
    Some(Value::Object(map)) => !map.contains_key(configured_value),
    _ => false,   // path missing or not an object → no match
},
```

Both operators return `false` when the resolved value is not a JSON
object. `MapDoesNotHaveKey` returning `false` on a non-object is the
conservative choice: the rule is checking object structure, so if the
value isn't an object the condition is not satisfied.

`configured_value` is the raw string comparison key (not a dotted
path); it checks one level of nesting within the resolved object.

## Unresolved questions

None.
