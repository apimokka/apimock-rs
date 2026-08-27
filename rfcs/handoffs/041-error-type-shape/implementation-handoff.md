# Implementation Handoff — RFC 041, error type shape

**Governing RFC.** [RFC 041](../../done/041-error-type-shape.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0 — **breaking**, deliberately
**Self-contained.** Every fact you need is restated here. RFC 041 is the
authority; if the two disagree, report it rather than following this.

> ### ⚠️ Amended 2026-08-20 — scope broadened before you started
>
> The owner decided `#[non_exhaustive]` is the **default for the whole
> re-exported public API**, not just the six error enums. If you read an
> earlier copy of this handoff, **§ 3 has changed materially** and it
> told you to stop if you found yourself marking a non-error type. That
> instruction is withdrawn — marking them is now the job.
>
> The boxing work (§ 1, § 2) is unchanged.

---

## 1. Why now, and why it was not done before

`clippy::result_large_err` is warn-by-default and CI runs `-D warnings`.
The lint fires nowhere today because it is suppressed at **15 sites
across 8 files**:

```
crates/apimock-config/src/config.rs                         ×5
crates/apimock-config/src/workspace.rs                      ×3
crates/apimock-config/src/workspace/path_helpers.rs         ×1
crates/apimock-routing/src/rule_set.rs                      ×1
crates/apimock-server/src/middleware.rs                     ×1
crates/apimock-server/src/middleware/middleware_handler.rs  ×1
crates/apimock-server/src/server.rs                         ×1
crates/apimock-server/src/tls.rs                            ×2
```

**These are not sloppiness.** They carry a comment saying the team
escalated and was told no:

> clippy: `WorkspaceError` is a public error type (RFC 030 §6 escalation
> trigger); boxing its large variant would change that type's shape.
> See ESCALATION-002 in the RFC 030 review-request package.

The answer at the time was "not without a major version". **6.0.0 is
that major version.** You are spending it, not working around it.

Verify the count with:

```sh
cargo clippy --workspace --all-features -- --force-warn clippy::result_large_err
```

`--force-warn` overrides the `#[allow]`s. It reports 136 bytes at seven
sites and 144 at eight.

## 2. The cause is one type, and it is not the obvious one

Measured against the pinned `toml` 1.1.4:

| Type | Size |
|---|---|
| `toml::de::Error` | **88 bytes** |
| `PathBuf` | 24 |
| `Option<PathBuf>` | 24 |
| `std::io::Error` | **8** |
| `Box<toml::de::Error>` | 8 |

`RoutingError::RuleSetParse { path, canonical, source }` is
24 + 24 + 88 = **136**, exactly what clippy reports.

**`std::io::Error` is 8 bytes** — it is already boxed internally. So the
`ConfigRead` / `RuleSetRead` / `PathResolve` / `Write` variants are
**innocent, and you must not box them**. There is nothing to win and it
would churn every construction site for no reason.

**Box exactly two variants:**

- `ConfigError::ConfigParse.source: toml::de::Error`
- `RoutingError::RuleSetParse.source: toml::de::Error`

24 + 24 + 8 = 56 plus discriminant. All 15 suppressions should then be
deletable. **If any survives, the cause was not fully fixed** — report
which, rather than leaving the `#[allow]` in place.

### Mechanics

```rust
RuleSetParse {
    path: PathBuf,
    canonical: Option<PathBuf>,
    #[source]
    source: Box<toml::de::Error>,
},
```

`#[source]` works through the box, so `Display` and `Error::source()` are
unchanged — this is a representation change, not a behavioural one.

`#[from]` **cannot box on its own**. Wherever a `#[from]` currently
builds one of these two variants, replace it with a hand-written `From`
impl that calls `Box::new`. Expect that to be the main mechanical cost.

## 3. `#[non_exhaustive]` and `kind()`

**Mark every re-exported public type `#[non_exhaustive]`** — about 43 of
them, the six error enums among them.

### Why this is the whole surface and not a shortlist

Adding the attribute is itself breaking, so **6.0.0 is the last free
window**. Any public type left bare after it means adding a field to
that type is a breaking change — a new diagnostic field, a new respond
option, each forcing 7.0.0. RFC 052 fixed five types, RFC 058 fixed
`Prefix`; each was a response to one instance. This is the change that
stops the pattern.

### How to derive the list — do not trust my count

Work from each crate's `lib.rs` re-exports, because that is what a
consumer can actually name:

1. Collect every type in a `pub use` in `apimock-config`,
   `apimock-routing`, `apimock-server` and `apimock`.
2. Drop any that already has the attribute (RFC 052's five, plus
   `Prefix` from RFC 058).
3. Drop **structs with no public fields** — they cannot be built by
   literal from outside the crate anyway, so the attribute buys nothing.
4. What remains is the work list. **Report it**; my sweep said ~43 and
   that number is approximate, the method is not.

Types that are `pub` but *not* re-exported are out of scope — nothing
outside can name them.

### The six error enums, which also get `kind()`

| Enum | Crate | Variants |
|---|---|---|
| `ConfigError` | `apimock-config` | `ConfigRead`, `ConfigParse`, `PathResolve`, `Validation`, `RuleSet` |
| `WorkspaceError` | `apimock-config` | `Config`, `InvalidRoot` |
| `ApplyError` | `apimock-config` | `UnknownNode`, `WrongNodeKind`, `InvalidPayload` |
| `SaveError` | `apimock-config` | `Serialize`, `Write`, `Inconsistent`, `Conflict`, `Read` |
| `RoutingError` | `apimock-routing` | `RuleSetRead`, `RuleSetParse` |
| `ServerError` | `apimock-server` | `TlsLoad`, `ListenerAddress`, `MiddlewareMissing`, `MiddlewareCompile`, `Io`, `Config` |

Error enums are where variants get added most often — every new failure
mode is one — so these are the types most likely to spring the trap
`#[non_exhaustive]` exists to prevent. **`kind()` is only for these six**;
no other type gains an accessor.

### Why `kind()` is not optional

`#[non_exhaustive]` obliges every downstream `match` to carry a wildcard
arm. Without a stable way to ask *"what class of failure is this?"*, that
wildcard is a dead end and callers fall back to string-matching on
`Display` — **worse than what they had**. So each enum gains a kind enum
and an accessor:

```rust
#[non_exhaustive]
pub enum ConfigErrorKind { Read, Parse, PathResolve, Validation, RuleSet }

impl ConfigError {
    pub fn kind(&self) -> ConfigErrorKind { … }
}
```

One kind per variant, derived mechanically — no independent state, no
judgement calls. The kind enums are themselves `#[non_exhaustive]`, for
the same reason the errors are.

### This is NOT RFC 053's `ErrorKind`

`crates/apimock/src/cmd/envelope.rs` has an `ErrorKind` with a closed set
(`usage`, `config_invalid`, `config_unreadable`, `io`, `conflict`,
`internal`) that is the **CLI's public contract**, with a schema version
and a stability promise to agents.

The `kind()` you are adding describes **library** failures. They are
different taxonomies in different crates on purpose. **Do not fuse
them, and do not make one delegate to the other** — that would tie a
published CLI contract to internal error refactoring.

## 3b. Construction — the part that can go badly wrong

For a type a consumer only **reads**, the attribute costs nothing. For a
type it must **build**, it forbids struct-literal syntax from outside the
crate — *including* `Foo { x, ..Default::default() }`, which is still a
struct expression. Fields stay public and assignable, so the supported
pattern is:

```rust
let mut p = RulePayload::default();
p.url_path = Some(…);
```

The GUI builds four `EditCommand` payload types:

| Type | Derives `Default`? | After the attribute |
|---|---|---|
| `RulePayload` | yes | fine |
| `RespondPayload` | yes | fine |
| `HeaderConditionPayload` | **no** | **unconstructable** |
| `BodyConditionPayload` | **no** | **unconstructable** |

**Those last two must gain construction in this same change** — a
`Default` derive, or a `new()` taking the genuinely required fields.
Derive `Default` where a default is meaningful; write `new()` where it
is not. Say which you chose, and why, for any type where it was not
obvious.

**This is the rule for the whole sweep, not just those two:** every type
that could be constructed from outside before must still be
constructible after. Shipping a type nobody can build is worse than
leaving it bare — check each one as you go rather than at the end.

## 4. The two open questions, decided

**Where do the kind enums live?** *Beside their errors* — in the same
`error.rs` as the enum they describe. A shared module would couple three
crates that today share only a dependency direction.

**Does `WorkspaceError` delegate to `ConfigError`'s kinds?** *No.* Give
it its own `WorkspaceErrorKind { Config, InvalidRoot }`. `WorkspaceError`
exists, per its own doc comment, so the `Workspace` API signals intent at
the type level; delegating would leak `ConfigError`'s taxonomy through
the type whose purpose is to have its own. A caller wanting the inner
detail matches on `source()`.

## 5. Scope

**In:** the three `error.rs` files, the two boxed variants and their
construction sites, the `From` impls, six `kind()` accessors and six kind
enums, deleting all 15 suppressions, `#[non_exhaustive]` across the
~43 re-exported types, construction for the two payload types that lack
it, the migration guide.

**Out:** redesigning the taxonomy — the variants are right; their
*shape* and *openness* are what change. `envelope.rs`'s `ErrorKind`.
Boxing any `io::Error` variant. `kind()` on anything that is not one of
the six error enums. Types that are `pub` but not re-exported, and
structs with no public fields — neither is reachable-and-constructible,
so neither benefits.

## 6. Evidence required

- **All 15 suppressions deleted**, and `cargo clippy --workspace
  --all-targets --all-features -- -D warnings` passes. The count is the
  acceptance test.
- `--force-warn clippy::result_large_err` reports **zero** sites.
- **Every error's `Display` output is unchanged.** Assert on rendered
  strings — these appear in user diagnostics and in `validate`'s output,
  which is a published surface.
- `Error::source()` still reaches the underlying `toml::de::Error`
  through the box.
- `kind()` returns the right kind for every variant of all six enums.
- A `compile_fail` doctest proving `#[non_exhaustive]` is load-bearing —
  **one error enum and one payload struct** — and verify it really is, by
  removing the attribute, confirming the doctest turns green when it
  should have stayed red, then restoring it. Report that you did.
- **The derived list of types that gained the attribute**, with the
  method from § 3. Not my number — yours.
- **Every type constructible from outside before is still constructible
  after.** Enumerate them and show a construction path for each. This is
  the acceptance bar for the blanket half.
- Migration guide covering both breaks. For `#[non_exhaustive]`, explain
  the *pattern* — `default()` then assign, add a wildcard arm — not 43
  separate entries.
- Full suite green with the count against `main`'s baseline;
  `cargo fmt --all --check`.

## 7. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: a suppression survives after boxing both variants (the
cause is then not what this handoff says); boxing changes any `Display`
output; a `kind()` mapping is genuinely ambiguous for some variant; or
**a type cannot be made constructible without a design decision** —
for example one whose required fields are not obvious, or where `new()`
would need an argument list long enough to be its own API question.
That last one is mine to answer, not a judgement call to make mid-sweep.
