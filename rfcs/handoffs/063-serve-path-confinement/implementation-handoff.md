# Implementation Handoff — RFC 063, confine the serve path

**Governing RFC.** [RFC 063](../../accepted/063-serve-path-confinement.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0, a 5.19.1 backport, **and a 4.8.1 backport** —
three tracks. Tracks A and B are in § 5; **Track C has its own
document**: [`track-c-v4-backport.md`](./track-c-v4-backport.md).
v4 is a live, supported line, not legacy.

> ## 🔒 EMBARGOED — read before anything else
>
> This describes an **unpublished, remotely reachable vulnerability in
> released software.** It is not public and must not become public
> before the advisory is.
>
> - Work on a **local branch only**. Do not push to `origin`, do not
>   open a PR, do not dispatch CI on a branch whose diff or commit
>   messages describe the flaw.
> - **Commit messages and test names must not describe the
>   vulnerability** until publication. "Confine resolved paths to the
>   respond directory" is fine. "Fix path traversal" is not.
> - Your review-request package goes in `.git-exclude/` as usual, which
>   is outside git — that stays true here and is why it is safe.
>
> The owner has approved: a **5.19.1** backport, a **published
> advisory**, and **6.0.0 waits** for both.

**Self-contained.** Everything you need is here.

---

## 1. The two defects, both one root cause

**Finding 1 — remotely reachable.** Verified against a running server
with `fallback_respond_dir = "serve"` and `outside.txt` one level above:

```
$ curl --path-as-is 'http://127.0.0.1:3199/../outside.txt'
SECRET-OUTSIDE-CONTENT      [HTTP 200]
```

Measured boundaries, so you do not re-derive them:

| Attempt | Today |
|---|---|
| `/../outside.txt` (raw `..`) | **200 — leaked** |
| `/../../outside.txt` | 404 |
| `/%2e%2e/outside.txt` | 404 — not decoded into `..` |
| `/hello` (normal) | 200, correct |

**Finding 2 — config capability.** A rule with
`respond.file_path = "../../outside.txt"` serves that file. Not
request-reachable (the value is operator-authored), but it means a
config file can read anything the process can.

**Root cause, shared:** a resolved path is never checked against the
directory it is supposed to stay inside. `dyn_route` joins a
request-derived path onto the respond dir (`dyn_route.rs:126`) and tests
only `.exists()`. `normalize_url_path`
(`apimock-routing/src/util/http.rs:20`) trims slashes and applies a
prefix — **it does not remove `..`**. No traversal guard exists anywhere
in `apimock-server`.

## 2. The fix

**Canonicalise and compare**, at all three `FileResponse` construction
sites:

| Site | Base directory |
|---|---|
| `dyn_route.rs:139` | the fallback respond dir |
| `respond_response.rs:77` | the rule set's `respond_dir` |
| `middleware/middleware_response.rs:63` | the same, for a Rhai-returned path |

Serve only if the canonical candidate is inside the canonical base.
Otherwise **404**.

**404, not 403.** A distinct status tells a prober that a file exists
outside the root. 404 tells them nothing. Log the refusal at debug level
with the reason; the response stays bare.

**Canonicalise the base once at load**, not per request — only the
candidate needs per-request work. See § 4 on measuring it.

## 3. The three open questions, decided

**Does `respond.file_path` get an opt-out? No — confinement is
absolute.**

RFC 062 gave the *write* path `--allow-outside` because a person typing
`set --rule-set ../x.toml` is asking for it and is the only one exposed.
The *serve* path is reachable by anything that can send a request. An
opt-out there is a config toggle that turns a security control off, and
someone will set it to make a layout work.

If files genuinely live elsewhere, **point `respond_dir` at them** —
explicit, per rule set, and already supported.

**Symlinks: refuse.** Canonicalisation resolves them, so a symlink
escaping the base is refused. That is the right default. If it breaks a
real layout we will hear about it and can revisit with evidence.

**Strip `..` in `normalize_url_path` too: yes.** Defence in depth. Two
independent controls — normalisation removes the ordinary case,
canonicalise-and-compare catches everything else, including symlinks and
any form that decodes into `..` later. Neither alone is the fix.

## 4. Evidence required

- **The § 1 repro returns 404**, asserted by a test that drives a real
  server with `--path-as-is` and a raw `..`.
- The encoded form, `..` mid-path, and a bare `..` are all covered.
- `respond.file_path` outside `respond_dir` is refused.
- A Rhai middleware returning an outside path is refused.
- **Every one of these tests fails without the fix.** Write them first,
  watch them fail, then fix — and **report both halves**. That ritual
  has caught a vacuous test three times this cycle, including one I
  required myself.
- **Normal serving is unaffected**: the W7 script and every existing
  `dyn_route` test still pass.
- **Per-request latency impact measured** and reported as a number.
- Full suite green; `fmt`; `clippy -D warnings`.

## 5. Two tracks

**Track A — `main`, for 6.0.0.** The full fix as above.

**Track B — `release/5.19`, for 5.19.1.** Branch from the `5.19.0` tag
(the `release/5.19` branch exists at `71fca6f`). **Minimal change only:**
the traversal fix and its tests. Nothing else — no refactors, no
unrelated test tidying, no `#[non_exhaustive]`, none of v6's work. A
patch release for a security fix must be trivially reviewable by someone
comparing it against the tag.

Expect Track B to differ: 5.19.0 predates RFC 043's module split, RFC
058's `resolved_respond_dir`, and RFC 062's confinement. **Do not
backport those to make the patch apply cleanly** — adapt the fix to the
old shape instead, and say where the two diverge.

## 6. Escalation

Blocking issues go in a `.git-exclude/review-request/` package as usual.

Escalate if: canonicalisation costs meaningful per-request latency;
confinement breaks a legitimate layout the § 3 answer does not cover; or
the backport cannot be made minimal without pulling in v6 work — that
last one is a decision about the patch release, not a judgement call to
make mid-implementation.

**And escalate immediately if anything about this reaches `origin`
before the advisory is published.** That is not a failure to hide; it is
a fact the disclosure timeline depends on.
