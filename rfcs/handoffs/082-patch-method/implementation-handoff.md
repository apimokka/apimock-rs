# Handoff — RFC 082: match PATCH, and make the refusal say why

**Governing RFC.** [082](../../accepted/082-match-the-methods-users-actually-send.md),
**accepted 2026-10-06 with Amendment 1**. Read both; Amendment 1 is at
the end of the file and it is the larger half of the work.
**Milestone.** Next minor. Closes the external audit's **F-06**, the
last open user-visible finding from it.
**Baseline.** **`main`'s head — cut from it.** No hash is pinned: every
tranche that pinned one shipped with a stale baseline. At time of
writing `main` is `0050ce8`, CI green.
**Branch.** **Not needed — work on `main`.** RFC 080 makes `main` the
working branch, and its § 3 carve-out applies only to things that can
behave differently on Windows or macOS. This is serde plumbing and
string comparison; nothing here is platform-sensitive.

---

## 0. What you are building

Two things, and the second is not optional garnish:

1. `HttpMethod` gains `Patch` and `#[non_exhaustive]`; PATCH joins
   `DEFAULT_ALLOWED_METHODS`.
2. **The refusal explains itself.** A user writing `method = "OPTIONS"`
   must be told *why* OPTIONS is not matchable, not merely that it
   isn't. Amendment 1 is the whole of this, and it exists because a
   message that is accurate about the valid set and silent about intent
   still leaves the user with a wrong belief — they conclude apimock is
   incomplete and re-file F-06.

## 1. The file list — corrected, and wider than the RFC says

**RFC 082 § 5's list is incomplete. Use this one, and re-run the search
yourself rather than trusting either:**

```
grep -rn "HttpMethod" crates/ --include='*.rs'
```

| site | what it needs |
|---|---|
| `crates/apimock-routing/src/rule_set/rule/when/request/http_method.rs` | the variant, `#[non_exhaustive]`, `as_str` arm, and all of Amendment 1 |
| `crates/apimock-server/src/constant.rs:3` | PATCH into `DEFAULT_ALLOWED_METHODS` |
| `crates/apimock/src/cmd/rule_check.rs:99-102` | **exhaustive match that reimplements `as_str()`** — see § 2 |
| `crates/apimock-config/src/workspace/edit/payload.rs:67-80` | the `set` CLI's parse **and its own hand-written "supported:" list** — see § 3 |
| `crates/apimock-routing/src/view/build.rs:255` | **missed by RFC 082 § 5.** Already `m.as_str().to_owned()` — **no change needed.** Listed so you do not go looking for a fourth edit that isn't there |
| `crates/apimock-config/src/toml_writer.rs:400` | same shape, already delegates to `as_str()` — **no change needed**, but the round-trip test in § 5 does cover it |

RFC 082 § 5 named `payload.rs`, `rule_check.rs` and `toml_writer.rs`
and missed `view/build.rs` entirely. It was found by grepping the whole
tree for the **type** rather than for a function name. This is the sixth
incomplete file list in this audit; the search above is the fix, not
this table.

## 2. `rule_check.rs` — delete the duplication, don't extend it

```rust
let expected = match req.http_method.as_ref() {
    None => return,
    Some(HttpMethod::Get) => "GET",
    …
};
```

This is `as_str()` written out again. The compiler will stop you when
`Patch` lands, and **the fix is not to add a fifth arm** — it is:

```rust
let Some(expected) = req.http_method.as_ref().map(|m| m.as_str()) else { return; };
```

`view/build.rs` and `toml_writer.rs` already do exactly this. Adding an
arm here would leave this project with one method-to-string conversion
that must be maintained and three that cannot drift.

## 3. The single source of truth must cross a crate boundary

Amendment 1 introduces `MATCHABLE` as the one list the refusal message
is generated from. **`payload.rs` has a second, independent copy of that
list** — in a different crate:

```rust
reason: format!("unsupported HTTP method `{}` — supported: GET, POST, PUT, DELETE", other)
```

If you add `Patch` to the enum and `"PATCH"` to this match but leave
that string, `apimock set rule --method PATCH` works while the error it
prints for anything else still advertises four methods. **That is the
divergence Amendment 1 exists to prevent, so it must consume the same
source.**

This means `MATCHABLE` cannot be a private `const` — `apimock-config`
needs it. **Expose it deliberately**, as a documented public associated
function on `HttpMethod` (a name like `matchable_names()`, or a
`parse_config_token()` that both the serde path and `payload.rs` call).
Decide which and say why.

> **This moves the API baseline further than RFC 082 § 5 predicted.**
> § 5 says *"the public-api baseline diff is exactly the two lines in
> § 5"* — `#[non_exhaustive]` plus `Patch`. Once Amendment 1 is
> implemented across crates it will be **more than two lines**. That is
> expected, not a mistake to hide: regenerate the baseline, declare the
> real diff, and **say in your package that § 5's prediction was wrong**
> rather than letting a reviewer find the mismatch.

## 4. Case sensitivity — a three-way inconsistency, flagged not fixed

After this RFC, the same method token is treated three different ways:

| surface | `"patch"` lowercase |
|---|---|
| TOML config (`method = "patch"`) | **refused** — `rename_all = "UPPERCASE"`, and Amendment 1 keeps this deliberately |
| `apimock set rule --method patch` | **accepted** — `payload.rs` matches `Some("PATCH") | Some("patch")` |
| the wire (a real request) | **matched** — `is_match` is `eq_ignore_ascii_case` |

Amendment 1 calls the config/wire asymmetry pre-existing and leaves it
alone. **It did not know about the CLI surface**, which makes two of the
three accept lowercase while config refuses it.

**Do not fix this.** Keep each surface exactly as it behaves today,
including `payload.rs`'s two-spelling match (add `Some("PATCH") |
Some("patch")`, matching the existing style). **Report it in your
package as an open question** — it is a design decision for the
architect and owner, and the honest answer may be to make config
case-insensitive, which is out of scope here.

## 5. Acceptance

- [ ] `method = "PATCH"` loads **and a real PATCH request matches the
      rule** — asserted on the HTTP response against a running server,
      not on a unit test of `is_match`
- [ ] PATCH does not match a `GET` rule, and vice versa
- [ ] Lowercase `patch` **on the wire** still matches
- [ ] `method = "patch"` in TOML is **still refused** — § 4's
      non-change, pinned so it cannot drift silently
- [ ] `method = "OPTIONS"` and `method = "HEAD"` are each refused with
      a message containing **both** the enumerated set and that
      method's reason — **quote both verbatim**
- [ ] `method = "GTE"` is refused with the enumerated set and **no**
      reason clause, unchanged from today
- [ ] A test proves the refusal's method list **equals `MATCHABLE`'s
      keys**, so the message cannot drift from the variant set
- [ ] `apimock set rule --method PATCH` writes a rule set that loads,
      and the value round-trips through `toml_writer` unchanged
- [ ] `payload.rs`'s unsupported-method error lists PATCH, **generated
      from the same source** as the serde refusal — not hand-edited
- [ ] `access-control-allow-methods` includes PATCH, observed on a real
      response
- [ ] `rule_check.rs` no longer contains a method-to-string `match`
- [ ] Baseline regenerated, the **real** diff declared, and § 3's note
      about § 5's wrong prediction stated in your package
- [ ] Docs: `docs/src/reference/rule-set-schema.md` (which methods are
      matchable, and why OPTIONS is not),
      `docs/src/reference/response-headers.md` (the header's value), and
      the next migration guide (the `_`-arm fix, same shape as
      `Outcome` in 6.2.0)

## 6. Report back

`.git-exclude/review-request/082-patch-method/`, including:

- the § 3 decision (what you exposed, and why that name)
- the real baseline diff, with § 5's wrong prediction stated
- § 4's three-way case inconsistency as an open question
- the two refusal messages, quoted
