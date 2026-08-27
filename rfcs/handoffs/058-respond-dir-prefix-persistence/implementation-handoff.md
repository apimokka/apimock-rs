# Implementation Handoff — RFC 058, `respond_dir` persistence

**Governing RFC.** [RFC 058](../../done/058-respond-dir-prefix-persistence.md)
**Risk.** **R-10** in `ROADMAP.md` — a released data-integrity defect
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0 — this is **breaking**, see § 2
**Self-contained.** Everything you need is restated here; RFC 058 is the
authority, and if the two disagree, report it rather than following this.

---

## 1. What is wrong, in one paragraph

`Prefix::respond_dir_prefix` holds two different values at different
times. The user writes `respond_dir = "responses"`, relative to the
rule-set file. `RuleSet::new` then joins it with the config-relative
directory to get something the matcher can use relative to the process
CWD — and **assigns the result back into the same field**. `toml_writer`
persists whatever is in that field. So the next load joins the
already-joined value again, and the file grows by one `./` per save,
without bound.

```
respond_dir = "./."  →  "././."  →  "./././."  →  …
```

The two sibling fields are spared, and the contrast is the whole
diagnosis: `url_path_prefix` only `.map()`s a value that already exists,
and `compute_fallback_respond_dir` returns early when the value is still
the default (`crates/apimock-config/src/config.rs:171`).
`respond_dir_prefix` is the only field that defaults **and** resolves
**and** writes back.

**This ships in 5.19.0** — both the join (`rule_set.rs:106-127`) and the
writer (`toml_writer.rs:84-85`). The GUI calls the same
`Workspace::save()` and hits it today. Nothing about it is new on `main`,
and RFC 056 did not cause it.

## 2. The three open questions, decided

### Unresolved 1 — is `Prefix` externally reachable? **Yes. Established.**

I checked rather than leaving it to you:

```
crates/apimock-routing/src/lib.rs:37      pub mod rule_set;
crates/apimock-routing/src/rule_set.rs:7  pub mod prefix;
prefix.rs:7                               pub struct Prefix
```

`apimock_routing::rule_set::prefix::Prefix` is public. **Adding a field
is a breaking change**, and `Prefix` is not `#[non_exhaustive]`.

**So: mark `Prefix` `#[non_exhaustive]` in this same change.** 6.0.0 is
the window and this fix wants to be inside it. That is the RFC 052
treatment, applied to a type RFC 052 did not cover.

**Related, and deliberately not your scope:** `RuleSet`, `Rule` and
`Respond` are also public — re-exported at the crate root
(`lib.rs:44`) — and also not `#[non_exhaustive]`. R-09 is therefore
wider than recorded. I am handling that separately. **Touch only
`Prefix`**; if you find yourself adding the attribute to a second type,
stop and say so.

### Unresolved 2 — repair files that already grew? **Yes, narrowly.**

Collapse a `respond_dir` whose value is **purely `./` segments** (`./.`,
`././.`, …) to `.` during load. Those are provably the same directory,
so nothing can break, and an authored path like `responses` or
`./responses` is never touched.

This rides along with a save the user already asked for — it is not a
standalone rewrite, and RFC 058 § Migration rejects standalone rewrites
for good reason.

**Known imperfection, accept it and document it:** a file damaged in the
era before this fix keeps `respond_dir = "."` even if it originally had
no `[prefix]` at all, because after the fact we cannot distinguish
"manufactured then grown" from "authored". Leaving `.` is honest and
harmless. Do not try to be cleverer.

### Unresolved 3 — persist `respond_dir = "."` when the user wrote it? **Yes.**

They wrote it; RFC 056 says keep it. Goal 2 is about files that never
had a `[prefix]` section, not about a user who typed the default
explicitly.

## 3. The fix

Two changes, and the second is what makes the first stick.

**a. Stop overwriting the authored value.** `respond_dir_prefix` keeps
exactly what was read from the file. The resolved directory the matcher
needs goes in a **separate field**.

The codebase already solves this one type away — copy its shape:

```rust
// crates/apimock-routing/src/rule_set/rule/respond.rs
pub status: Option<u16>,               // what the user wrote
#[serde(skip)]
pub status_code: Option<StatusCode>,   // the resolved runtime form
```

**Mechanism note, because it differs.** `Prefix` derives `Deserialize`
only — there is no `Serialize`, and the write path is `toml_writer`'s
hand-built table (`toml_writer.rs:84-85`), not serde. So `#[serde(skip)]`
is *not* what keeps the resolved value out of the file; the writer
simply must never be handed it. Take the shape of the precedent, not its
implementation.

**b. Stop manufacturing a value that was never written.** `unwrap_or(".")`
plus an unconditional `ret.prefix = Some(prefix)` means a rule set with
**no** `[prefix]` section acquires one on its first save. Confirmed:
`apimock set` bootstraps a rule set as exactly `rules = []`
(`crates/apimock/src/cmd/set.rs:138`), and one save gives it a `[prefix]`
block containing `respond_dir = "./."`.

Absent must stay absent. Once the load stops inventing a value, the
writer needs no special case — it already emits only when the field is
`Some`.

## 4. Scope

**In:** `crates/apimock-routing/src/rule_set.rs`,
`crates/apimock-routing/src/rule_set/prefix.rs`, and whatever minimal
change `crates/apimock-config/src/toml_writer.rs` needs. The
`#[non_exhaustive]` on `Prefix` only. The migration-guide entry.

**Out:** `url_path_prefix` and `fallback_respond_dir` — both correct,
present only as the contrast that isolates the bug. `EditCommand` access
to `respond_dir` (nothing exposes it and this does not add it). The
meaning of `respond_dir`. `#[non_exhaustive]` on any other type.
Rewriting files outside a save the user requested.

## 5. Evidence required

- **A load+save cycle is a fixed point.** Save three times over; assert
  the file is **byte-identical** after each. This is the regression test
  the bug never had, and it is the acceptance bar.
- A rule set with no `[prefix]` section still has none after a save.
- `respond_dir = "responses"` round-trips **unchanged**.
- `respond_dir = "."`, written explicitly, round-trips unchanged.
- `respond_dir = "././."` collapses to `"."` on the next save, and
  `respond_dir = "./responses"` does **not** collapse.
- **`Respond::file_path` still resolves correctly at request time** —
  prove it with a real request that returns file-backed content, not by
  inspecting a field. The field is exactly what was wrong before, so it
  is not evidence.
- A `respond_dir` pointing at a non-existent directory still fails
  `Prefix::validate` as today.
- **Run RFC 057's W7 script three times over** and confirm the config is
  byte-stable after the first run. That is the end-to-end proof.
- A `compile_fail` doctest for `Prefix`'s `#[non_exhaustive]`, in the
  manner RFC 052 established.
- Full suite green with the count against `main`'s baseline; `cargo fmt
  --all --check`; `cargo clippy --workspace --all-targets --all-features
  -- -D warnings`.

## 6. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate specifically if: the resolved value turns out to be read from
`respond_dir_prefix` somewhere outside the matcher (grep before you move
it — that is why the resolved value keeps a name rather than
disappearing); or if collapsing `./` segments interacts badly with
Windows path separators; or if `#[non_exhaustive]` on `Prefix` breaks
something inside the workspace, which would mean an internal caller is
constructing it by literal.
