# RFC 008 — Body match language extension

**Status.** Implemented (v5.8.0)
**Tracks.** Routing core — extending the body matching language from
string-equality-style operators on a single value to type-aware
comparison, presence assertions, and small predicate expressions.
**Touches.** `apimock-routing` (`Body`, body match implementation,
`util::json` path resolver), `apimock-config` (`BodyConditionPayload`
operator set per RFC 002), documentation, examples.

## Summary

Body matching today uses a dotted-path mini-syntax that resolves a
single value out of the request JSON and compares it (string-style:
`equal`, `contains`, `starts_with`, etc.) against a configured value.
This is too coarse for several common needs: numeric comparison,
presence/absence assertions, array-shape predicates, and type-aware
matching. This RFC extends the body language additively with new
operator variants and explicit type semantics, without breaking the
existing dotted-path resolver.

## Motivation

Three concrete pain points from real mock-server usage:

1. **Numeric thresholds.** "Match if `body.json.amount > 100`."
   Today: not possible. The closest workaround — `Contains "1"` — is
   wrong in many ways.
2. **Field presence.** "Match if `body.json.user_id` is missing"
   (e.g. to return a 400 for malformed requests). Today: not
   possible. Users hack around it by relying on header conditions
   or fallback respond logic.
3. **Type confusion.** A request with `"amount": 42` (number) vs
   `"amount": "42"` (string) currently both match the configured
   `equal "42"` because the routing engine coerces values to
   strings. This is a footgun for users testing strict APIs.

A small, well-chosen extension covers these without growing the
language into a full predicate calculus.

## Guide-level explanation

The body match operator set gains:

- **Numeric operators.** `equal_number`, `greater_than`,
  `less_than`, `greater_or_equal`, `less_or_equal`. These coerce
  the matched value to a number and fail to match if it can't be
  coerced.
- **Type-aware equality.** A new `equal_typed` operator that matches
  only if the JSON value is *the same type and the same value*.
  Distinguishes `42` from `"42"`.
- **Presence operators.** `exists`, `absent`. These don't read a
  value — they assert the dotted path resolves (or doesn't resolve)
  to something.
- **Array predicates.** `array_length_equal`, `array_length_at_least`,
  `array_contains`. The first two test array length; the third
  checks whether a value appears anywhere in the array.

The existing string-style operators (`equal`, `contains`,
`starts_with`, `ends_with`, `regex`) keep their current semantics.
A new explicit `equal_string` is added as a clearer alias for
`equal` (to disambiguate from `equal_number` / `equal_typed`); the
unsuffixed `equal` continues to work for backwards compatibility.

## Reference-level explanation

### Operator additions

```rust
pub enum BodyOp {
    // existing (5.7.0 baseline)
    Equal,
    Contains,
    StartsWith,
    EndsWith,
    Regex,

    // new — numeric
    EqualNumber,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,

    // new — type-aware
    EqualTyped,
    EqualString,        // explicit alias of Equal

    // new — presence
    Exists,
    Absent,

    // new — array
    ArrayLengthEqual,
    ArrayLengthAtLeast,
    ArrayContains,
}
```

### Semantics

For a body condition at dotted path `P` with operator `O` and
configured value `V` against incoming JSON `J`:

| Op                    | Match iff                                                  |
|-----------------------|------------------------------------------------------------|
| `Equal`               | resolve(J, P) coerced to string equals V coerced to string |
| `EqualString`         | same as `Equal`                                            |
| `EqualNumber`         | resolve(J, P) parses as number AND equals V as number      |
| `GreaterThan`         | resolve(J, P) parses as number AND > V                     |
| `LessThan`            | resolve(J, P) parses as number AND < V                     |
| `GreaterOrEqual`      | as above with `>=`                                         |
| `LessOrEqual`         | as above with `<=`                                         |
| `EqualTyped`          | resolve(J, P) is the same JSON type as V AND `==` to V     |
| `Exists`              | resolve(J, P) returns Some — V ignored                     |
| `Absent`              | resolve(J, P) returns None — V ignored                     |
| `ArrayLengthEqual`    | resolve(J, P) is array AND length == V (V coerced int)     |
| `ArrayLengthAtLeast`  | resolve(J, P) is array AND length >= V                     |
| `ArrayContains`       | resolve(J, P) is array AND contains V (typed comparison)   |

### Numeric coercion rules

- JSON `Number` → use the number directly.
- JSON `String` that parses as f64 → use the parsed value.
- Other types → no match (return `false`).

For operators that take a configured value `V`:
- TOML `Integer` / `Float` → number.
- TOML `String` that parses as f64 → number.
- Other → validation error at load time.

The "string-that-parses-as-number" coercion is deliberate — TOML
authors sometimes write `"42"` to force a string type, and the
common-sense expectation is that it still works as a numeric
comparison.

### Path resolution unchanged

The dotted-path resolver in `apimock_routing::util::json` is
unchanged. The 5.7.0 cosmetic hygiene work (clarifying that this
is *not* canonical JSONPath) stands. This RFC only adds operators
that consume the resolver's output.

### Validation

- Numeric operators: the configured value must be a TOML number or a
  string parseable as one. Caught at config-load / validation time.
- `ArrayLengthEqual` / `ArrayLengthAtLeast`: configured value must
  be a non-negative integer.
- `Exists` / `Absent`: configured value must be `None`.

`BodyConditionPayload` from RFC 002 already encodes `value:
serde_json::Value`; validation translates the constraints above.

### Backwards compatibility

Every rule that works under 5.7.0's body matching continues to work
under this RFC. The new operators are additive enum variants.
Specifically:

- A rule using `op = "equal"` keeps its 5.7.0 semantics
  (string-style equality).
- Authors who want strict type matching upgrade to `equal_typed`.
- Authors who want numeric comparison upgrade to `equal_number` /
  `greater_than` / etc.

The `equal` → `equal_string` rename is *cosmetic* — both serialise
to the same variant. Documentation should prefer the explicit
`equal_string` for clarity in new rules.

## Drawbacks

1. **Operator count grows substantially.** From 5 to 16. Each one
   needs tests and documentation. The cognitive load on users
   browsing the operator list goes up.
2. **Numeric coercion has edge cases.** `NaN`, `Infinity`,
   precision-limited f64 comparison. The recommended path
   ("use f64 throughout, treat any parse failure as no-match,
   document `Infinity` as an undefined-behaviour case") is
   pragmatic but not theoretically clean. Users matching very
   large integers (>2^53) will hit precision issues.
3. **The "coerce string-numbers" rule is convenient but fuzzy.**
   `equal_number` on a value that *looks* like a number but is
   semantically a string (e.g. a stringified user ID) may give
   surprising results. Users who want strict typing should use
   `equal_typed`.
4. **`ArrayContains` typed comparison.** Defining "contains" for
   arrays of objects requires a deep-equality rule. This RFC
   specifies it as JSON-value equality (`serde_json::Value::eq`),
   which is well-defined but does not match the more permissive
   notion of "contains" the user might expect.

## Rationale and alternatives

**Alternative A: don't extend — direct users to Rhai middleware for
anything beyond string matching.** Smallest core. Loses ergonomics
for the three pain cases above.

**Alternative B (this RFC): targeted operator additions.** Solves
the cases without adding a query language.

**Alternative C: replace the mini-syntax with canonical JSONPath
(RFC 9535).** Maximum expressiveness; significant complexity to
implement and to document. Rejected — the mini-syntax was a
deliberate choice; broadening to full JSONPath should be a
separate RFC.

**Alternative D: small embedded predicate language ("CEL-lite").**
A flexible expression DSL would replace operators entirely. Too
ambitious; rejected for stage-2 scope.

We pick B. A leaves the pain unsolved. C and D are separate
discussions, deferrable.

## Prior art

- WireMock's body matchers include `matchesJsonPath`,
  `equalToJson`, `containsString`, `matches` (regex). Its JSONPath
  matcher is the canonical RFC 9535 syntax; we explicitly chose not
  to follow it for body conditions in apimock for the reasons
  given in the 5.7.0 docs work.
- Postman's response-matching uses ChaiJS expectations; closer to
  CEL than to a fixed operator set. Beyond scope for now.
- Mountebank's `deepEquals` operator handles object equality
  similarly to this RFC's `EqualTyped` (though not identical for
  arrays).

## Unresolved questions

1. **Treatment of `null` JSON values.** Does `body.json.field = null`
   in incoming JSON count as `Exists`? The intuitive answer is "yes,
   the key exists; its value is null." Document and test.
2. **Big-integer matching.** As noted in drawbacks, f64 precision
   limits matching for integers above `2^53`. Should there be a
   separate `equal_integer` operator using i64? Probably yes as a
   follow-up; not in v1.
3. **Regex on numeric values.** `op = "regex"` on a numeric value:
   coerce to string and apply regex, or no-match? Current behaviour
   (coerce) is convenient; this RFC keeps it.
4. **`ArrayContains` for objects.** Strict JSON-value equality is
   the safe default; a future "structurally-contains" operator
   (subset matching for objects) is a known follow-up need.

## Future possibilities

- `MapHasKey` for object-keyed predicates.
- "Structural contains" for object subset matching.
- Separate `equal_integer` operator for exact-integer matching.
- Migrating the body language to a small expression DSL (CEL-lite)
  once the operator count crosses a usability threshold.
- An `apimock match-test` CLI subcommand that lets users dry-run a
  body condition against a JSON file. Tooling support is the
  obvious force-multiplier once the operator set is rich.
