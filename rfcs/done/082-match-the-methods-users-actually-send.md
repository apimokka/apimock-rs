# RFC 082 — Match the methods users actually send (PATCH), and stop advertising ones we cannot

**Status.** **Implemented — shipped in 6.3.0** (2026-10-07). Owner
approved 2026-10-06 with Amendment 1; implemented at `2184372`; the
casing hint that extends Amendment 1 followed at `9136df4` (task 018).
**Tracks.** Functionality. External audit 2026-09-01, **F-06** — the last
open user-visible finding from that audit.
**Touches.**
`crates/apimock-routing/src/rule_set/rule/when/request/http_method.rs`,
`crates/apimock-server/src/constant.rs`,
`crates/apimock-config/src/workspace/edit/payload.rs`,
`crates/apimock-config/src/toml_writer.rs`,
`crates/apimock/src/cmd/rule_check.rs`,
`crates/apimock-routing/public-api.txt`,
`docs/src/reference/rule-set-schema.md`,
`docs/src/reference/response-headers.md`, and the next migration guide.
**Origin.** Owner approved costing option (b) on 2026-10-06. **Having
costed it, this RFC recommends against (b)** — see § 3.

## Summary

`when.request.method` accepts exactly four methods. **PATCH cannot be
matched at all**, and PATCH is in universal REST use.

Add `Patch` to `HttpMethod`, mark the enum `#[non_exhaustive]` so the
next addition is free, and add PATCH to the `access-control-allow-methods`
we advertise — because without that last part a browser would still
refuse the PATCH a user just configured.

**Deliberately not adding `Head` or `Options`**, which F-06 also names.
Each needs a behaviour decision beyond matching, and shipping either as
a matcher alone would create a rule that looks right and does the wrong
thing. § 4 states both, and both were decided on approval — see
§ Unresolved questions. Amendment 1 makes the refusal explain each
exclusion to the user rather than leaving it in the docs.

## Motivation

### The gap, reproduced

```
$ cat rs/rules.toml
[[rules]]
when.request.method = "PATCH"
…
$ apimock validate -c ./cfg.toml
apimock validate: failed to load config: … TOML parse error at line 2
  |
2 | when.request.method = "PATCH"
  |                       ^^^^^^^
unknown variant `PATCH`, expected one of `GET`, `POST`, `PUT`, `DELETE`
```

### What is already right, and must survive the fix

That refusal is **loud and enumerated**. A typo behaves the same way:

```
unknown variant `GTE`, expected one of `GET`, `POST`, `PUT`, `DELETE`
```

So today's behaviour is not unsafe or confusing — it is merely
*incomplete*. Any fix has to keep that property. It is the same property
RFC 069 spent a tranche installing for rule keys: a mistyped value is
refused at load, with the valid set named, rather than becoming a rule
that quietly never matches.

### Why this was parked, and why that reason is gone

ROADMAP recorded F-06 as unfixable within 6.x: `HttpMethod` is a public
enum and not `#[non_exhaustive]`, so adding a variant breaks downstream
exhaustive matching. That rested on the **additive-only promise**, which
was retracted on 2026-09-07 — RFC 039 is a *declaration* gate, and
`docs/src/library/api-stability.md` now says so. A declared, documented
break inside 6.x is permitted.

There is a direct precedent six weeks old: **6.2.0 added
`Outcome::Middleware` and marked `Outcome` `#[non_exhaustive]` in one
change**, declared in the baseline and written up in the migration
guide. This RFC is the same move on a different enum.

## Design

### 1. `HttpMethod` gains `Patch`, and becomes `#[non_exhaustive]`

```rust
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum HttpMethod { Get, Post, Put, Delete, Patch }
```

- `#[serde(rename_all = "UPPERCASE")]` already maps TOML `"PATCH"`, so
  the config surface needs no new syntax.
- **`is_match` does not change.** It is already a string comparison —
  `self.as_str().eq_ignore_ascii_case(http_method.as_str())` (RFC 077
  P-07) — so nothing about matching depends on the variant set. Only
  `as_str` gains an arm.
- `Display` keeps its shape (`` method`PATCH` ``, RFC 079 M-09).
- The serde refusal **improves**: the enumerated list a typo gets back
  grows to include PATCH, for free.

### 2. Advertise it

`DEFAULT_ALLOWED_METHODS` (`crates/apimock-server/src/constant.rs:3`) is
`"GET, POST, PUT, DELETE, OPTIONS"`, sent as
`access-control-allow-methods` on every response.

**Without changing this, the fix is half a fix**: a user configures a
PATCH rule, the server would match it, and a browser's preflight refuses
to send the request because we told it PATCH is not allowed. The user
sees a CORS error and no reason to connect it to the method list.

Add PATCH. `docs/src/reference/response-headers.md` documents this
header's value and must change with it.

### 3. Why not option (b), the arbitrary-method matcher

Costed as approved. The shape was: leave `HttpMethod` alone, introduce a
matcher type accepting any method token, and point
`Request::http_method` at it. It works, and it would support TRACE,
CONNECT, WebDAV and custom verbs.

**Rejected, for two reasons.**

**It destroys the property § Motivation says must survive.** Accepting
any token means `method = "GTE"` loads clean and produces a rule that
silently never matches — reintroducing exactly the RFC 069 / audit F-17
failure class. Recovering the refusal would mean hand-writing the
validation and the "expected one of" message that serde gives free.

**It costs a second concept for a feature nobody asked for.** Two ways
to express a method — an enum of known ones and a string matcher —
is the kind of surface a reader has to learn twice. No issue requests
custom methods; CONNECT is a proxy mechanism with no meaning for a mock
server, and TRACE is commonly disabled as a security measure.

If a custom-method need appears, it is a separate RFC with a real use
case behind it.

### 4. Deliberately out: `Head` and `Options` — a narrowing, flagged

F-06 names PATCH, HEAD and OPTIONS. This RFC closes PATCH only.
**Recorded here rather than left for a reviewer to notice.**

**`Options` would be a rule that can never fire.** Dispatch is
`OPTIONS → middleware → rule sets → dyn_route`
(`crates/apimock-server/src/server.rs:460`), and `server.rs:484`
early-returns for every OPTIONS request as CORS preflight, before rule
sets are consulted. Adding `Options` to the matcher would let a user
write a rule that validates, reads correctly, and is unreachable. That
is worse than not offering it.

**`Head` would let a rule return a body HTTP forbids.** There is no
HEAD-specific handling anywhere in `apimock-server`. Adding `Head`
without also suppressing the response body would let
`respond.text = "..."` answer a HEAD request with content, which clients
and proxies are entitled to mishandle.

Both are genuine gaps; neither is a matcher change. **Both decided on
approval 2026-10-06** — see § Unresolved questions. Amendment 1 carries
these two reasons into the refusal message itself.

### 5. API impact — declared, not avoided

| baseline line | change |
|---|---|
| `pub enum …::HttpMethod` | gains `#[non_exhaustive]` |
| `…::HttpMethod::Patch` | added |

A consumer matching `HttpMethod` exhaustively must add a `_` arm — once,
after which further methods cost them nothing. Identical in shape and
justification to `Outcome` in 6.2.0, and it gets the same migration-guide
entry.

**Our own crates need the same arm**, since `#[non_exhaustive]` binds
outside the defining crate:
`apimock-config/src/workspace/edit/payload.rs:68-71` (the `set` CLI's
method parsing — must also accept `PATCH`/`patch`),
`apimock/src/cmd/rule_check.rs:99-102`, and `toml_writer.rs` (which
notes it depends on the `Deserialize` derive — the write path must
round-trip `--method PATCH`).

## Testing and verification

- `method = "PATCH"` loads, and a PATCH request matches the rule —
  asserted on the **HTTP response**, against a running server.
- A PATCH request does **not** match a `GET` rule, and vice versa.
- Mixed-case `"patch"` on the wire still matches (`hyper::Method` does
  not normalise case — the existing `is_match` tests establish this).
- A typo is still refused at load, and the message now lists PATCH:
  **quote the actual output**, since the enumerated refusal is the
  property this RFC promises to preserve.
- `apimock set rule --method PATCH` writes a rule set that loads, and
  the value round-trips through `toml_writer` unchanged.
- `access-control-allow-methods` includes PATCH, observed on a real
  response.
- The public-api baseline diff is exactly the two lines in § 5.

## Risks

| Risk | Mitigation |
|---|---|
| A consumer's exhaustive `match` on `HttpMethod` stops compiling | Declared in the baseline and in the migration guide, with the `_`-arm fix stated. One-time, by design — that is what `#[non_exhaustive]` buys. |
| Closing PATCH makes HEAD/OPTIONS look finished | **Amendment 1 is the mitigation**: the refusal for `HEAD`/`OPTIONS` names the reason, so a user meets it in the error rather than only in the docs. The schema docs state which methods are matchable and why OPTIONS is not. |
| PATCH added to the matcher but not to `allow-methods` | § 2 is in scope precisely so this cannot happen. |

## Unresolved questions

**1 and 2 were decided on approval, 2026-10-06; 3 remains open.**

1. ~~**`Head` — add it, with body suppression?**~~ **Decided
   2026-10-06: a separate RFC, and it must carry body suppression with
   it.** A HEAD matcher alone would let `respond.text` answer a HEAD
   request with a body HTTP forbids — a rule that looks right and
   responds invalidly. Not in this RFC's scope; Amendment 1 instead makes
   the *refusal* say why HEAD is absent.
2. ~~**`Options` — should a rule be able to answer a preflight?**~~
   **Decided 2026-10-06: preflight stays authoritative**, and the schema
   docs say so. Letting a rule outrank the built-in handler is a
   CORS-visible behaviour change; nothing has asked for it. Amendment 1
   makes the refusal explain this rather than leaving the user to guess.
3. **Does `DEFAULT_ALLOWED_METHODS` belong in config at all?** It is a
   constant today. Out of scope here; noted because this RFC is the
   first thing to need it changed.

---

## Amendment 1 — adopted 2026-10-06: the refusal must say *why*, not just *what*

**Adopted — owner approved 2026-10-06.** Raised by the architect during
a self-review against the owner's stated design philosophy, before the
owner's review. **This widens the RFC's scope**; it is recorded as an
amendment rather than folded silently into § Design.

### The gap

The body above closes PATCH and leaves HEAD and OPTIONS out with
reasons stated in § 4 — but those reasons reach the user **only if they
read the schema docs**. What they actually hit is serde's generic
refusal:

```
unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
```

That names the valid set and says nothing about **intent**. The user
cannot tell whether OPTIONS is an oversight or a decision, so the
reasonable inference is that apimock is incomplete — and the likeliest
next action is to file F-06 again.

By the project owner's second clause — *APIs and UI/UX for users not to
be confused or misunderstand* — this is a defect even though the code
behaves correctly and the refusal is already loud and enumerated. A
message that is accurate about the valid set and silent about intent
still leaves the user with a wrong belief.

### The design

Refuse with the enumerated set **plus the reason**, for every method we
deliberately do not match:

```
unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
  — OPTIONS is answered by the built-in CORS preflight handler before rule
    sets are consulted, so it cannot be matched by a rule
```

Implement by replacing the `Deserialize` derive with
`#[serde(try_from = "String")]` and a `TryFrom<String>` impl, driven by
two `const` tables:

```rust
/// The single source of truth for what config accepts. The refusal
/// message is generated from this, so the two cannot drift.
const MATCHABLE: [(&str, HttpMethod); 5] = [
    ("GET", HttpMethod::Get),
    ("POST", HttpMethod::Post),
    ("PUT", HttpMethod::Put),
    ("DELETE", HttpMethod::Delete),
    ("PATCH", HttpMethod::Patch),
];

/// Methods a user will plausibly try that we deliberately do not match,
/// each with the reason the refusal quotes.
const NOT_MATCHABLE: [(&str, &str); 4] = [
    ("OPTIONS", "answered by the built-in CORS preflight handler before \
                 rule sets are consulted, so it cannot be matched by a rule"),
    ("HEAD",    "not matchable yet: a rule could answer it with a response \
                 body, which HTTP forbids for HEAD"),
    ("TRACE",   "not supported; TRACE is commonly disabled as a security \
                 measure and has no meaning for a mock server"),
    ("CONNECT", "not supported; CONNECT is a proxy mechanism with no \
                 meaning for a mock server"),
];
```

- A token in `MATCHABLE` deserializes, case-sensitively, exactly as
  today.
- A token in `NOT_MATCHABLE` is refused with the enumerated set **and**
  its reason.
- Anything else — a typo — is refused with the enumerated set alone,
  which is what `"GTE"` gets today.

### Why this does not weaken § 3's rejection of option (b)

§ 3 praised serde for giving the enumerated refusal *free*, and this
amendment hand-writes it. **That argument weakens; the rejection does
not**, because it never rested on the free message. It rested on
behaviour: an arbitrary-token matcher lets `method = "GTE"` load clean
and produce a rule that silently never matches, which is the RFC 069 /
audit F-17 failure class. A hand-written message is a maintenance cost;
a silently-never-matching rule is a wrong answer to a user. Those are
not the same kind of thing.

### Drift, which is this repo's own repeated defect

A hand-maintained list that must track a variant set is exactly the
mechanism behind the duplicated-validator findings. Three things hold it
in place:

1. `MATCHABLE` is the **only** list; the message is generated from it.
2. `as_str` stays an exhaustive `match`, so **the compiler** rejects a
   new variant that does not handle it.
3. A test asserts every `MATCHABLE` entry round-trips —
   `parse(s).as_str() == s` — and that the refusal text lists exactly
   `MATCHABLE`'s keys.

The one residual gap is adding a variant and forgetting `MATCHABLE`:
that variant would simply be unconfigurable, and § Testing already
requires a loading test per matchable method, which would fail.

### Deliberate non-change, flagged

**Config stays case-sensitive.** Today `rename_all = "UPPERCASE"`
accepts `"PATCH"` and refuses `"patch"`, while `is_match` is
case-*insensitive* on the wire. Hand-rolling the parse makes accepting
`"patch"` nearly free, and it is arguably friendlier — but it widens
accepted config syntax beyond this RFC's subject, and the inconsistency
is pre-existing rather than introduced here. **Left exactly as it is**,
and recorded as a candidate question rather than quietly changed.

### Acceptance, added to § Testing and verification

- `method = "OPTIONS"` and `method = "HEAD"` are each refused with a
  message containing both the enumerated set and that method's reason —
  **quote both messages verbatim** in the review package.
- `method = "GTE"` is refused with the enumerated set and **no**
  reason clause, unchanged from today.
- `method = "patch"` is still refused — the non-change above, pinned so
  it cannot drift silently.
- The refusal text's method list is generated, not literal: a test
  proves it equals `MATCHABLE`'s keys.
