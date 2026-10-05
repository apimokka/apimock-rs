# RFC 082 — Match the methods users actually send (PATCH), and stop advertising ones we cannot

**Status.** Proposed — awaiting owner approval.
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
thing. § 4 states both, as owner questions rather than silent omissions.

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

Both are genuine gaps; neither is a matcher change. **Owner questions in
§ Unresolved.**

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
| Closing PATCH makes HEAD/OPTIONS look finished | § 4 states both as open, and the schema docs will say which methods are matchable and why OPTIONS is not. |
| PATCH added to the matcher but not to `allow-methods` | § 2 is in scope precisely so this cannot happen. |

## Unresolved questions

1. **`Head` — add it, with body suppression?** Matching HEAD is only
   correct if the response body is dropped. Recommend doing both, in a
   separate RFC, rather than shipping a matcher that invites an invalid
   response. The owner may prefer it folded in here.
2. **`Options` — should a rule be able to answer a preflight?** Today
   the built-in handler owns OPTIONS unconditionally. Letting a rule
   take precedence is a CORS-visible behaviour change and wants its own
   decision. Recommend leaving preflight authoritative and documenting
   it.
3. **Does `DEFAULT_ALLOWED_METHODS` belong in config at all?** It is a
   constant today. Out of scope here; noted because this RFC is the
   first thing to need it changed.
