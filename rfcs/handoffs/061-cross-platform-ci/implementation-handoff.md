# Implementation Handoff — RFC 061, test on the platforms we ship

**Governing RFC.** [RFC 061](../../done/061-cross-platform-ci.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0 — **blocking**
**Self-contained.** Everything you need is here. RFC 061 is the
authority; if the two disagree, report it rather than following this.

---

## 1. Expect this to go red, and treat that as the deliverable

You are not adding a green checkmark. **You are finding out what is
broken on two platforms nobody has ever tested.**

Every job in `ci.yaml` is `runs-on: ubuntu-latest`. The release workflow
ships `x86_64-pc-windows-msvc` and `aarch64-apple-darwin`, and its step
is `cargo build --release --target … --locked` — build only, **no
`cargo test`**. macOS and Windows users install binaries on which not
one assertion has ever executed.

If the first run passes everywhere, be suspicious and check the jobs
actually ran the suite rather than skipping it.

## 2. The change

Add a matrix to the **`test` job only**:

```yaml
test:
  strategy:
    fail-fast: false
    matrix:
      os: [ubuntu-latest, windows-latest, macos-latest]
  runs-on: ${{ matrix.os }}
```

`fail-fast: false` is required, not stylistic. RFC 047 needed it for the
same reason: one platform failing must not cancel the others and hide
whether they pass.

**Leave `fmt`, `clippy`, `msrv`, `audit` and `lockfile` on Ubuntu.**
They are platform-independent; tripling them buys nothing.

**Decision — trigger:** run all three on **every push**, same as today's
`test` job. If wall-clock proves painful, report the number and we will
narrow it to `main` plus pull requests. **Never narrow it to "release
only"** — that reproduces today's gap with extra steps.

## 3. Where failures are most likely

Not a prediction to code against — a list of places to look first when
something goes red:

- **Path separators** in fixtures and assertions. Anything comparing a
  literal `"./foo/bar"` against a produced path.
- **Line endings.** RFC 056 and RFC 060 both assert **byte-identical**
  file content. Git's autocrlf on Windows runners can make a file differ
  from what the test wrote.
- **Temp directories** — `tempfile` semantics and cleanup differ.
- **File locking on Windows.** A save rewrites a file; if any handle is
  still open, Windows refuses where Unix allows. RFC 056's atomic write
  and RFC 058's repeated saves are the likely sites.
- **`is_purely_current_dir`** (RFC 058) — written with
  `Path::components()` *specifically* to be Windows-correct, and never
  executed there. This is the single most interesting thing in the run.
- **The W7 acceptance script**, which defines v6 as complete and has
  never run on Windows.

## 4. What to do with a failure

**Triage, do not paper over.**

- A genuine cross-platform defect → report it. **Do not fix it in this
  change**, unless it is a one-line test-fixture issue. A CI change that
  also carries production fixes is unreviewable in the way RFC 043's
  handoff described.
- A test that is Ubuntu-specific by nature → make that explicit
  (`#[cfg]` or a documented skip) and **say which and why** in the
  submission. A silent skip is worse than a red test.
- **Never disable a platform to get green.** If Windows is too flaky to
  keep, that is a finding for me, not a decision to make in the YAML.

## 5. Evidence required

- The `test` job runs on all three platforms; paste the per-platform
  results.
- **The W7 acceptance script passes on all three**, or its failures are
  listed.
- **`fail-fast: false` demonstrated** — show a run where one platform
  failed and the others still reported. If nothing failed naturally,
  break something temporarily to show it, then restore.
- **CI wall-clock impact reported as a number** — before and after, for
  the `test` job.
- Every failure found is listed with: platform, test, and whether it is
  a product defect or a test-environment artefact.
- Ubuntu results are unchanged from today's baseline.

## 6. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: a failure is in the **write path** (`set`, `save`,
`toml_writer`, path resolution) — that is the class RFC 061 exists to
find and the owner has said it may gate the release; Windows proves
flaky in a way quarantining one test cannot settle; or the wall-clock
cost looks unacceptable to you, in which case propose the narrower
trigger rather than dropping a platform.
