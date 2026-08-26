# Track C — the 4.8.1 backport

**Governing RFC.** [RFC 063](../../accepted/063-serve-path-confinement.md)
**Entry point.** [`implementation-handoff.md`](./implementation-handoff.md) — Tracks A and B, both delivered
**Checklist.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md) — § E applies here too, adapted
**Milestone.** **4.8.1**, published alongside 5.19.1

> ## 🔒 EMBARGOED — and this track is the most sensitive of the three
>
> Same rules as Tracks A and B: **local branch only**, no push, no PR,
> no CI dispatch, and **commit messages and test names must not describe
> the flaw**. "Confine resolved paths to the directory they were
> resolved against" is fine.
>
> This one matters more because **v4 is a live, supported line** — not a
> legacy branch. An accidental push here discloses an unfixed
> vulnerability in software people are running today.

---

## 1. Why this track exists

**v4 is not superseded by v5.** v5 exists for GUI-app integration; v4
remains live and supported in its own right. Both lines get the fix, and
a v4 user must not be told to change major version to be safe.

I recorded the opposite earlier — I assumed v4 was legacy and drafted an
end-of-life line for the advisory. That was wrong, corrected by the
owner, and this track is the correction.

**Confirmed vulnerable by exploit, not by reading:** 4.8.0 was built
from its tag and

```
curl --path-as-is 'http://127.0.0.1:3210/../outside.txt'
V4-SECRET-OUTSIDE      [HTTP 200]
```

## 2. The shape of v4, and what transfers

v4 is a **single crate** — `[package] apimock`, `src/` layout — not the
4-crate workspace. So this is a re-implementation, **not a
cherry-pick**. But it maps site-for-site, and the pattern is now
implemented twice, so it is transcription rather than design.

**The four sites** (paths are exact, at tag `4.8.0`):

| Site | File |
|---|---|
| dyn-route fallback — **the request-derived one, the reachable defect** | `src/core/server/routing/dyn_route.rs` |
| a rule's `respond.file_path` | `src/core/server/routing/rule_set/rule/respond.rs` |
| a Rhai middleware's returned path | `src/core/server/middleware/middleware_response.rs` |
| the shared reader | `src/core/server/response/file_response.rs` |

**`normalize_url_path` is byte-identical to the current one** —
`src/core/util/http.rs:46`, same body, same comments. The `..`-stripping
change from Track A transfers **directly**, with no adaptation.

The dyn-route resolution to confine is at `dyn_route.rs`:

```rust
let request_path =
    Path::new(fallback_respond_dir).join(url_path.strip_prefix("/").unwrap_or_default());
```

— joined from a request-derived path, then only `.exists()` is checked.
There is no confinement anywhere in v4's serve path; I checked every
apparent guard and each is a doc comment, a canonical path stored for an
error message, or a `starts_with` on a URL *prefix* string.

## 3. Scope — minimal, and stricter than Track B

**In:** the confinement check at the four sites, `..` stripping in
`normalize_url_path`, and their tests.

**Out — and this is stricter than Track B was:** no refactors, no
shared-helper extraction beyond what the fix itself needs, no test
tidying, nothing backported from v5 or v6 to make anything apply.

Track B came out at 31 files because I asked for a load-time
canonicalisation optimisation and defence in depth in a patch release —
instructions that were in tension with "minimal", and that tension was
mine. **Do not repeat it here.** If canonicalising per request is
simpler on v4's shape, do that and note the cost; a security patch on a
live line should be reviewable against its tag in one sitting.

## 4. Branch and baseline

- Branch **`release/4.8`** from the **`4.8.0`** tag. No such branch
  exists yet.
- **Record v4's own test baseline before changing anything** — build and
  run the suite at the tag, and report that number. Do not compare
  against `main`'s; the two are unrelated.

## 5. Evidence required

- **The exploit reproduced against unfixed 4.8.0 first**, then returning
  404 after the fix. Both halves, both pasted — the before-state is what
  makes the after-state mean anything.
- `..` mid-path, a bare `..`, and the `%2e%2e` form covered.
- `respond.file_path` outside its respond dir → refused.
- A Rhai middleware returning an outside path → refused.
- Normal serving unaffected: files legitimately inside the respond dir
  still serve, including extension inference (`/foo` → `foo.json`).
- Full suite green against **v4's own baseline**, with the count.
- `cargo fmt --all --check` and `cargo clippy -- -D warnings` clean **on
  v4's toolchain** — if the old code cannot satisfy today's clippy, say
  so and report what fails rather than fixing unrelated lints into a
  security patch.

## 6. Publishing is not your step

For the record, so nobody backports machinery that is not needed:

v4-era tags carry **no crates.io or npm automation**.
`release-executable.yaml` at 4.8.0 builds binaries and attaches them to
the GitHub Release; there is no `cargo publish` or `npm publish` in it.
v4 was published by hand, and 4.8.1 will be too, by the owner, via the
procedure in `RELEASING.md`.

**Do not backport `release-publish.yaml`.** It publishes four crates in
dependency order and v4 has one; adapting it is more machinery than a
one-off patch warrants.

## 7. Escalation

Blocking issues go in a `.git-exclude/review-request/` package.

Escalate if: v4's toolchain or dependencies will not build or test
cleanly; the fix needs something from v5 that cannot be reimplemented in
a few lines; or clippy on v4 demands changes unrelated to this fix.

**And escalate immediately if anything reaches `origin`.** On this track
that would disclose an unfixed vulnerability in a live, supported line.
