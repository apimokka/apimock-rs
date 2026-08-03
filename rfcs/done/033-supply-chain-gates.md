# RFC 033 — Supply-chain and dependency-hygiene gates

**Status.** Implemented (v5.15.0)
**Tracks.** M1 (Pipeline trust). No workflow checks dependencies for
known vulnerabilities, incompatible licences, or lockfile drift. A
vulnerable transitive dependency would ship in a release undetected.
**Touches.** `.github/workflows/ci.yaml` (extends RFC 031's workflow)
and `.github/CONTRIBUTING.md`. No crate source, no public API, no
runtime behaviour.

## Summary

Add two dependency-hygiene checks to CI: a vulnerability audit and a
lockfile-freshness check. Recorded as SEC-006 in the v5.14.0 decision
log and unaddressed since.

**Scope narrowed by owner decision D-04 (2026-08-02): the
dependency-licence policy is dropped.** `cargo-deny` and `deny.toml` are
not part of this RFC. The rationale and the supporting graph survey are
retained below as a decision record, because the question will recur.

Depends on [RFC 031](./031-ci-quality-gates.md) for the workflow this
extends.

## Motivation

apimock ships a binary built with `--locked` from a committed
`Cargo.lock` — good practice, and it is what makes the dependency set
knowable. But nothing ever inspects that set.

The dependency surface is not small. The workspace pins tokio, hyper,
hyper-util, rustls, tokio-rustls, rhai, toml, json5, csv, regex,
globset, ignore, uuid, indexmap, serde, serde_json, and others, each
pulling transitive dependencies. `rustls` and `tokio-rustls` sit
directly on the TLS path — the one part of this codebase where a
dependency vulnerability has security consequences rather than merely
correctness ones.

SEC-005 already records that `--locked` reduces supply-chain risk by
pinning what gets built. That is only half the property: pinning
guarantees you build *the same* dependencies every time, not that those
dependencies are *safe*. Without an audit step, `--locked` faithfully
reproduces a vulnerable build.

There is a second, quieter risk. `Cargo.lock` is committed but nothing
verifies it matches the manifests. A `Cargo.toml` edit that forgets to
regenerate the lockfile leaves `--locked` builds resolving versions
that no longer match what the manifest asks for.

## Guide-level explanation

Two additional CI checks:

| Check | Command | Blocking |
|---|---|---|
| Vulnerability audit | `cargo audit` | Yes |
| Lockfile freshness | `cargo update --workspace --locked` | Yes |

A newly disclosed advisory against a dependency turns CI red on the
next run — including on branches that changed nothing. That is intended:
the advisory is new information about existing code.

## Reference-level explanation

### Vulnerability audit

`cargo audit` against the RustSec advisory database, over the committed
`Cargo.lock`.

**Advisory handling.** When an advisory fires, the response is, in
order of preference: upgrade the dependency; if no fixed version
exists, assess whether the vulnerable path is reachable from apimock's
usage; if unreachable, record an ignore entry **with the advisory ID,
the reachability argument, and a review date**. A bare ignore with no
rationale is not acceptable — it is how an audit gate decays into
decoration.

**Scheduled run.** The audit also runs on a weekly `schedule` trigger,
not only on push. Advisories are published against code that has not
changed; a check that only runs on commits will not see them.

### Dropped by D-04 — `cargo-deny` and `deny.toml`

**Not implemented.** The first draft proposed `cargo-deny` covering
licences, bans/duplicates, and sources. Owner decision D-04 dropped it.

Two consequences worth recording so they are not lost:

- **Bans and duplicates** are not checked. Multiple major versions of
  the same crate bloat a binary whose release profile is explicitly
  tuned for size (`opt-level = "z"`, `lto`, `strip`,
  `codegen-units = 1`). This is a quality nicety, not a risk gate.
- **Sources** are not restricted to crates.io. A git or out-of-workspace
  path dependency would not be flagged automatically — though `--locked`
  builds from a committed `Cargo.lock` make one visible in review.

Both are cheap to add later if wanted. Neither was the reason this RFC
exists.

### Decision record — the licence check, and why it was dropped

**It is not about apimock's own licence.** apimock is Apache-2.0 and
has been since the start; nothing in this RFC touches that. The check
inspects the licences of the **270 crates in apimock's dependency
graph** — code the project does not own but does link into and
distribute inside every released binary.

The concern it guards against is a transitive dependency arriving under
a strong copyleft licence (GPL/AGPL), which would impose obligations on
the distributed binary that conflict with shipping it as Apache-2.0.
Nobody chooses such a dependency deliberately; it arrives as a
dependency-of-a-dependency during a routine version bump.

**Evidence from the actual graph** (surveyed 2026-08-02, all 270
packages). The graph is overwhelmingly permissive — 139 `MIT OR
Apache-2.0`, 42 `MIT`, 18 `Unicode-3.0`, 17 `Apache-2.0 OR MIT`, and so
on. **No GPL or AGPL anywhere.** Exactly three crates are not
obviously permissive:

| Crate | Licence | Reached via | Assessment |
|---|---|---|---|
| `smartstring` 1.0.1 | MPL-2.0+ | `rhai` → `apimock-server` → `apimock` — **in the shipped binary** | Weak, *file-level* copyleft. Obliges disclosure of modifications to MPL-licensed files; apimock does not modify it. Linking into a larger Apache-2.0 work is permitted. Not a conflict. |
| `tiny-keccak` 2.0.2 | CC0-1.0 | transitive | Public-domain dedication. No obligations. |
| `webpki-roots` 1.0.9 | CDLA-Permissive-2.0 | TLS path | Permissive data licence. No obligations. |

Two conclusions follow, and the second is a correction.

1. **The risk this check guards against is currently zero.** There is
   no licence conflict in the graph today. Its value is prospective —
   catching the bad transitive dependency on the day it arrives, not
   fixing something now broken.
2. **The allow-list proposed in this RFC's first draft was wrong.**
   That list (Apache-2.0, MIT, BSD-2/3, ISC, Unicode) would have failed
   CI immediately on all three crates above — including `smartstring`,
   which is in the shipped binary and entirely legitimate. Shipping
   that list would have produced a red gate on day one for no real
   problem. It is corrected below.

**Corrected policy**, derived from the surveyed graph rather than from
convention:

```toml
allow = [
  "MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception",
  "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
  "Unicode-3.0", "Unlicense", "CC0-1.0", "BSL-1.0",
  "CDLA-Permissive-2.0",
  "MPL-2.0",   # weak file-level copyleft; see smartstring above
]
```

Everything else — notably GPL, LGPL, and AGPL — fails. That is the
whole point of the list, and it is the only line in it that does real
work.

**Outcome — D-04, 2026-08-02: dropped.** The licence check was the
weakest of the three originally proposed. `cargo audit` and the lockfile
check address live risks; the licence check addressed a hypothetical one
that currently measures zero across all 270 crates. The owner chose to
ship the two live-risk checks and not carry a policy file for the third.

Revisit if a dependency ever arrives under a licence outside the set
surveyed above — the survey and the corrected allow-list are retained
here precisely so that conversation starts from evidence rather than
from the wrong list the first draft proposed.

### Lockfile freshness

`cargo update --workspace --locked` fails if the lockfile does not
match the manifests. Cheap, and it closes RFC 031's Unresolved
question 2.

### Explicit non-change scope

- No dependency is upgraded, added, or removed *as part of this RFC*
  beyond what is required to clear a firing advisory. A general
  dependency-modernisation pass is separate work.
- No change to `[workspace.dependencies]` version pins for tidiness.
- No crate source.

## Required tests

1. **Both checks pass** on `main` at the merge commit.
2. **Each check has been observed failing.** A deliberately stale
   lockfile, and — where practicable — a crate version with a known
   advisory; confirm each fails independently. As in RFC 031, a gate
   seen only passing has not been tested.
3. **The weekly schedule trigger fires** and reports.

Note the environment constraint RFC 031 hit (Decision 001): observing a
gate fail requires a real Actions run, which requires the disposable
branch procedure. Same guardrails apply — no push to `main`, no tag, no
Release, no pull request.

## Acceptance criteria

1. `cargo audit` runs on push, pull request, and a weekly schedule. The
   lockfile check runs on push and pull request; whether it also runs on
   the schedule was delegated to the implementer (see § Lockfile
   freshness) and resolved as **excluded** — its result cannot change
   without a commit, so a scheduled re-run would be a guaranteed no-op.
2. Both pass on `main`.
3. Each has been observed failing independently.
4. **No `deny.toml` exists and `cargo-deny` is not invoked** — D-04.
5. Any advisory ignore entry carries an advisory ID, a reachability
   argument, and a review date.
6. No dependency was changed except as required to clear a firing
   advisory.

## Drawbacks

1. **CI can turn red without any commit.** A new advisory blocks
   unrelated work until triaged. This is the correct behaviour and also
   a genuine interruption; the reachability-assessment path exists so
   that triage does not always mean an immediate upgrade.
2. **One more tool to keep working.** `cargo audit` is widely used and
   stable.
3. **No licence, duplicate, or source policy.** Consequence of D-04,
   recorded above rather than left implicit.

## Rationale and alternatives

**Alternative A (chosen, as narrowed by D-04): `cargo audit` plus a
lockfile-freshness check, both blocking.** Addresses the two live risks
without carrying a policy file.

**Alternative B: `cargo deny` instead of `cargo audit`** — it subsumes
advisory checking. Not chosen: `cargo audit` is the more direct,
better-understood signal for the vulnerability case, and D-04 removed
the licence policy that was `cargo-deny`'s main additional draw.

**Alternative C: GitHub Dependabot alerts instead.** Complementary, not
equivalent: Dependabot notifies, it does not gate. Worth enabling
alongside; not a substitute.

**Alternative D: advisory-only, non-blocking.** Rejected for the reason
recorded in RFCs 030 and 031.

**Alternative E: fold this into RFC 031.** Defensible — both are CI
workflow changes. Kept separate because the two answer different
questions (is our code clean? / are our dependencies safe?) and have
different failure modes: RFC 031's gates go red when someone changes
something, these go red when the world changes underneath unchanged
code. With D-04 narrowing this RFC to two checks the case for merging is
stronger than it was, but separate RFCs keep that distinction legible in
the record.

## Unresolved questions

1. ~~**Is the licence check wanted at all?**~~ ✅ **Resolved — D-04,
   2026-08-02: dropped.** See the decision record above. `cargo-deny`
   is out of scope entirely, which also forgoes its bans/duplicates and
   sources checks; both are cheap to add later and neither was this
   RFC's purpose.
2. **How should a firing advisory with no fixed version be handled at
   release time?** Blocking a release on an unreachable vulnerability
   in a dev-dependency would be disproportionate; blocking on a
   reachable one in `rustls` would not. Recommend the reachability
   assessment above rather than a blanket rule, and revisit if it
   proves too loose in practice.
3. **Should the weekly schedule open an issue automatically** rather
   than only reporting a red run? Convenient, but it needs
   `issues: write` on a workflow that otherwise needs no write scope.
   Defer.
