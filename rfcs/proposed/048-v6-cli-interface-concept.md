# RFC 048 — v6 concept: the CLI as a first-class interface

**Status.** Proposed — concept accepted by the project owner 2026-08-17;
this document records the design. **Umbrella RFC**: it decides shape and
constraints, and spawns the RFCs that implement them. It ships no code
itself, in the manner of RFC 034.
**Tracks.** Product direction. v6 makes configuration and inspection
available *through the command line* to callers who are not sitting at a
terminal — principally AI CLI agents and CI — without weakening the
server or the GUI integration that v4 and v5 built.
**Touches.** A new CLI surface over existing `apimock-config` /
`apimock-routing` machinery; documentation; the compatibility promise.
**No change to the mock-serving hot path is intended.**

## Summary

Two families of command:

- **get** — *"what will this path return?"*, *"what will I get for this
  request?"* — answered from configuration, in plain text or JSON.
- **set** — *"make the server answer X under condition Y"* — modifying
  configuration files more safely than a person editing TOML by hand.

The purpose is to let an AI CLI stand up a useful mock server in a few
commands and verify an application against it, and to give the GUI a
second route to the same operations.

## Context — where this sits in the project's history

| Era | Theme |
|---|---|
| v1–v3 | Server APIs, unrefined |
| v4 | Wholesale rewrite; routing refined — the API shape in use today |
| v5 | APIs for GUI integration, for non-expert users, with server robustness and performance preserved |
| **v6** | **The CLI becomes an interface in its own right, not a launcher** |

Each major has marked an era rather than strictly a semver break. v6
continues that, and this time a break is explicitly permitted (§ 7).

## The reframe that should govern the work

**v6 is largely a second front-end over machinery v5 already built.**

- The **get** half exists in embryo: `apimock match-test`
  (`crates/apimock/src/args.rs` → `cmd/match_test.rs`) already answers
  "what would this request match?". RFCs 004, 016 and 029 built
  structured match views, per-condition addressability, and diff
  granularity — the machinery for *explaining* an answer.
- The **set** half exists as the GUI's write path: `toml_writer` +
  `Workspace::save`, guarded by RFC 024's external-change detection.

So the difficulty is not capability. It is **surface design, guarantees,
and documentation** — which is where the effort should go, and why this
document exists before any implementation RFC.

## 1. Who uses this, what they expect, and how they fail

Design decisions are justified against these users by name throughout.
Each is described by **how it behaves, what it expects, and how it goes
wrong** — the failure column is the one that earns its keep, because
this project's recurring lesson is that things break quietly and stay
broken.

### U1 — A person at a terminal

**Behaves.** Types a command, reads the output, adjusts, tries again.
Explores by trial, often without reading documentation first.

**Expects.** Output meant for eyes. `--help` when stuck. To be told when
they mistype something.

**Fails by.** Mistyping a flag and not noticing. Believing the answer
describes the server that is actually running, when it was answered from
a different config file. Editing TOML by hand and getting the shape
subtly wrong.

**Therefore.** Help and version must work; a wrong flag must say so; the
answer must state which configuration produced it.

### U2 — An AI CLI agent — *the user this release is for*

**Behaves.** Composes commands from documentation it was given, runs
them non-interactively, parses stdout, branches on the result, and
chains many operations without a human reviewing any single step.

**Expects.** A stable, versioned, machine-readable shape. Deterministic
output. Errors it can distinguish programmatically. That a command which
appears to succeed did succeed.

**Fails by — silently, and this is the whole design problem.** It takes
a wrong answer as truth and builds on it. It cannot tell *no rule
matched* from *config file invalid* from *bad flag* if all three are
prose. Given a typo'd flag that starts a server instead of erroring, it
records success and proceeds. It rewrites a config file, destroys the
comments a person wrote, and never sees the warning that says so.

**Therefore.** Versioned schemas; a categorised error taxonomy; strict
stdout/stderr separation; meaningful exit codes; loud failure over quiet
substitution. Where U1 and U2 conflict, U2 decides the machine-readable
surface and U1 decides the default human one.

### U3 — A CI pipeline

**Behaves.** Runs a fixed set of commands, asserts exit codes, never
answers a prompt.

**Expects.** Stable exit codes, no interactivity, no hidden network
dependency.

**Fails by.** Going green while having validated nothing — the failure
this project has already lived through, when npm shipped 4.6.9 binaries
with every job passing.

**Therefore.** A real failure must be non-zero; a command must never
wait for input; and "nothing to check" must not be reported as success.

### U4 — The GUI application — *a constraint, not a target*

**Behaves.** Long-lived session: load a workspace, edit, preview a diff,
save.

**Expects.** The library API it already uses to keep working, and its
in-memory model not to be invalidated behind its back.

**Fails by.** Compile error if we break the API — which is the *good*
case, because it is loud. The bad case is a CLI write landing under a
running GUI session and the two disagreeing about what is on disk.

**Therefore.** The library API must not regress, and reconciliation
(RFCs 024, 042) becomes a shared concern rather than a GUI detail.

### U5 — An MCP host — *later, and derivative*

**Behaves and fails as U2**, through an adapter.

**Therefore.** No separate architecture. If U2's contract is clean, an
MCP server is a thin shim over it; designing MCP as a parallel thing
would be the mistake.

### The thread running through all five

Every one of them fails the same way — **confidently misled** — and only
the consequences differ. U1 investigates, U3 fails a build, U2 builds a
tower on top of it. That is the argument for spending v6's effort on
error taxonomy, provenance and loud failure, rather than on breadth of
commands.

## 2. Workflows

Concrete sequences, written as acceptance targets rather than
illustrations.

### W1 — What does this path return? *(U1, U2)*

One command, no running server, config on disk. Answers with the
response body, its status, and its headers.

### W2 — What do I get for *this request*? *(U2, U3)*

As W1 but with method, headers and body supplied, because rules match on
all of them. This is `match-test`'s existing model, promoted.

### W3 — Why? *(U1 debugging, U2 self-correcting)*

Which rule set, which rule, which condition decided it — and for a
near-miss, which condition failed. RFCs 016 and 029 already produce this
granularity internally.

**W3 is what makes U2 able to fix its own mistakes.** An agent told only
"no match" must guess; an agent told "rule #3 matched the path but its
`x-api-key` header condition failed" can act. This is the highest-value
part of the get family and should not be deferred as polish.

### W4 — Make the server answer X under condition Y *(U2, U1)*

The set family. Must support **preview before commit** — `SaveResult`
already carries `changed_files`, `diff_summary` and `requires_reload`,
which is close to the right response shape; `compute_diff_summary` is
currently `pub(super)` and would need exposing.

### W5 — Validate before running *(U3)*

`apimock validate` exists (RFC 026). It gains meaning here because U2
*generates* configuration, and generated configuration is exactly what
nobody reads before running.

### W6 — Someone else changed the file *(all)*

CLI, GUI and a text editor may all write. RFC 024 detects external
change; v6 must decide what the CLI *does* about it — refuse, merge, or
overwrite — and say so in the contract rather than in behaviour.

### W7 — Bootstrap, end to end *(U2)* — **the acceptance test for v6**

From an empty directory to a running mock that answers a non-trivial
conditional request, in a handful of non-interactive commands, with
every step machine-checkable.

**v6 is finished when W7 is a script that runs in CI, not when a feature
list is complete.** If W7 is awkward, the design is wrong regardless of
how complete the surface looks.

## 3. The contract, in four layers

They fail independently, so they are specified independently.

1. **Invocation** — subcommand and flag names; exit codes; strict
   stdout/stderr discipline. Diagnostics on stdout break every U2/U3
   consumer at once, so that separation is a hard rule, not a
   convention.
2. **Data** — response schemas, **versioned**, so they can evolve
   without silently breaking scripts.
3. **Errors** — machine-readable and categorised. U2 must distinguish
   *no rule matched* from *config invalid* from *bad flag*
   programmatically. Today all three are prose on a terminal.
4. **Workflow** — for `set` only: idempotency, preview, transaction
   boundary across multiple edits, and behaviour under W6. **Safety is a
   property of the sequence, not of any single call**, which is why a
   data model alone is not a sufficient contract for `set`.

## 4. Static first — decided

`get` and `set` operate on **configuration files, with no running
server** (Option 1). Rationale: always available, no process lifecycle,
and it matches `match-test`'s existing model.

**Two consequences to design for.**

*Drift.* A static answer can disagree with a running server started with
a different `--config`, or whose files changed since startup. An agent
told X while the server returns Y has been actively misled — so `get`
must report **which configuration it answered from**, not only the
answer.

*The other options.* A server-hosted config API (Option 2) is
**deliberately deferred** — it would put a configuration *write* surface
on a network port belonging to a process people run in CI and Docker,
sometimes bound to `0.0.0.0`. If it is ever built: separate port,
loopback only, disabled by default, never the listener serving mock
traffic. MCP (Option 3) is an adapter over § 3, not a third
architecture.

## 5. Documentation is a deliverable, not a by-product

Stated by the owner and recorded here: **specs and documentation rank
with the public API design.** For U2 this is literal — the documented
contract is the only thing an agent's author can build against.

So each implementation RFC ships its documentation with it, and the
Diátaxis structure from RFC 034 gains a reference section for the CLI
contract, versioned alongside the schemas of § 3.2.

## 6. Prerequisites carried over from v5

Ordinary v5 items that v6 promotes to blocking.

| Item | Why it blocks |
|---|---|
| **Unknown flags are silently ignored** (`args.rs`, `args_option_value`) — a typo starts a server | Disqualifying for U2: silent wrong behaviour that looks like success |
| **No `--version` / `--help`** | The first thing any agent or human runs |
| **Bare relative `--config apimock.toml` fails** | An agent will write exactly that |
| **`toml_writer` canonicalises — sorted keys, no comments** | See § 8 |
| **`guard` is a published, zero-field stub** | A `set` surface would expose a field that does nothing |
| **RFC 045 Goal 4 — `validate` passes on inert config** | Answered "no general mechanism" when humans wrote every line; U2 generating config changes that calculus |
| **Prerelease versions are unreleasable** (caret pins) | Only if a `6.0.0-rc` is wanted |

## 7. Compatibility — decided by the owner

**v6 may break 5.x invocations.** Owner decision, 2026-08-17: the major
version changes, so breakage is permitted; a **migration guide** covers
the transition.

One consequence must be designed for rather than accepted. U1 reads
release notes; **U2 and U3 do not**, and a migration guide never reaches
them. Therefore:

> **A removed or changed invocation must fail loudly, with a
> machine-readable error naming its replacement.** It must never
> silently do something different.

That is compatible with breaking — it constrains *how* we break, not
*whether*. It also means § 6's "unknown flags are ignored" defect is a
prerequisite for the migration story, not merely for hygiene.

RFC 039's additive-only gate therefore needs an **explicit exception
path for major versions**, which it would not have needed under a
no-break promise.

### 7.1 A deprecation window ships in 5.x first

Owner decision, 2026-08-17. A clean break at 6.0.0 is preceded by a
**5.x release that warns about what will change**, so that a user or a
pipeline meets the change once while the old behaviour still works.

**This inverts the obvious order of work, and that is the point to
notice.** A deprecation warning has to name its replacement, so v6's
breaking surface must be *decided* — not implemented, but decided and
written down — **before the final 5.x ships**. We cannot finish v5 and
then design v6; the last 5.x release is itself a v6 deliverable.

Consequences:

- **The list of breaking changes is a v5 blocker.** Not the design of
  each command, but the enumeration: which invocations change, and to
  what.
- **Warnings go to stderr, never stdout.** U2 and U3 parse stdout; a
  deprecation notice there breaks the very consumers it is meant to
  help. This is § 3.1's rule, and the deprecation window is its first
  real test.
- **A warning is not a failure.** During the window the old form keeps
  working and keeps exit code 0, or the window achieves nothing.
- **The window needs an announced end.** "Deprecated since 5.x, removed
  in 6.0.0" — stated in the message itself, not only in release notes,
  because U2 reads the message and not the notes.

What closes out v5 is therefore no longer just M3's remainder. It is
M3's remainder **plus** the deprecation release, and the § 6
prerequisites that make a deprecation legible at all — a CLI that
silently ignores unknown flags cannot deliver a warning that anyone
acts on.

### 7.2 Amendment — 2026-08-17 — the deprecation release ships from a branch

**Owner decision.** `main` now carries breaking work — RFC 040's
`TraceConfig` fields and RFC 050's additions to `ParsedRequest` and
`RequestSummary` — landed *before* the enumeration § 7.1 says must
precede it. So `main` can no longer produce a non-breaking deprecation
release, and a deprecation release that itself breaks is close to
useless.

**The deprecation release is therefore cut from `5.18.0`** on a
short-lived branch carrying only deprecation warnings, not any of
040/050/051. The security fix stays landed on `main`; the branch lives
only as long as the release takes.

This is a sequencing error we walked into rather than a discovery: § 7.1
already said the enumeration blocks the final 5.x, and breaking work
landed anyway.

### 7.3 What a deprecation window can and cannot warn about

Established while planning the branch, and it narrows the release
considerably.

**CLI invocation changes can be warned about.** A superseded invocation
prints to stderr, keeps exit code 0, and names its replacement. That is
§ 7.1's mechanism and it works.

**Most of v6's library breaks cannot be.** There is no mechanism to warn
that a struct is about to become `#[non_exhaustive]`, or that a field is
about to be added — both change what downstream *may write*, and no lint
exists to say "you are using a struct literal on a type that will stop
allowing them". `#[deprecated]` covers removal, not this.

**Consequence for the plan.** The deprecation release is a **CLI**
deprecation release. RFC 052's `#[non_exhaustive]` change, RFC 041's
error boxing, and RFC 050's field additions reach users through the
**migration guide** instead, and are announced rather than warned.

**And that reorders the critical path.** The deprecation release is
gated not on the library enumeration — which is largely known already —
but on v6's **CLI surface** being designed far enough to say which
invocations change and to what. That is § 11's items 3, 4 and 5, and it
is now the work standing between here and the close of v5.

## 8. `toml_writer` — the honesty problem in "safer than by hand"

`toml_writer` builds `toml::Value` trees and renders with
`to_string_pretty`: canonicalised output — sorted keys, no comments
(`workspace.rs`, and the module's own docs). That was negotiated for the
GUI, whose spec calls full comment preservation an explicit non-goal,
and which can warn the user once through `SaveResult`'s `Info`.

**A CLI advertised as safer than hand-editing cannot silently delete the
user's comments**, and U2 will never read an `Info` diagnostic. The
module already names the remedy — *"Future work could swap this module
for `toml_edit` to preserve formatting; the public `Workspace::save` API
would not change."*

Treat this as v6's **first implementation RFC**. It is what makes the
claim true rather than promotional.

## 9. Threat model

To be refined during v6 development, per the owner. This records the
starting position and the questions it forces.

### 9.1 What actually changes

v5 is read-mostly: it loads configuration and serves responses. **v6
writes configuration**, and its principal user (U2) routinely acts on
input it did not author. Those two facts together are the entire shift —
neither is true of v5.

### 9.2 Trust boundaries

- **Trusted:** the person invoking the CLI, and the filesystem they own.
  It is their tool on their machine; v6 does not defend a user against
  their own commands.
- **Not trusted, and new in v6:** the content an agent read *before*
  composing the command — a ticket, a web page, a README in a repository
  it was pointed at. v5 had no equivalent boundary.
- **Not trusted:** configuration originating from someone else's project.

### 9.3 Threats

| ID | Threat | Posture |
|---|---|---|
| **T1** | **Path traversal through `set`** — writing outside the workspace | Must be prevented. The workspace root becomes a boundary that is *enforced*, not assumed |
| **T2** | **A configuration write becomes code execution.** `service.middlewares` lists Rhai scripts, and `service_table` already emits that key (`toml_writer.rs:190`) — so a `set` built on `Workspace::save` can attach a script the server will run | **Decide explicitly, do not inherit.** Recommendation: middleware attachment is outside `set`'s v6 scope, or reachable only through a distinct explicit command — never through a generic "set this field" verb |
| **T3** | Indirect prompt injection reaching `set` through U2 | Not solvable inside the CLI, but it must not be *amplified*: no shell evaluation of arguments, no implicit writes, destructive operations explicit rather than inferred |
| **T4** | **Secret leakage through `get` / W3 output.** Fixtures carry credential-shaped headers; explain output and JSON responses land in CI logs | Ties directly to RFC 040's redaction work, already flagged security-relevant. v6 must not open a second leak path while that one is being closed |
| **T5** | Symlink / TOCTOU on configuration write | Atomic write (temp + rename); do not follow links out of the workspace; interacts with RFC 024's external-change detection |
| **T6** | Server-hosted configuration API — unauthenticated remote *write* on a process frequently bound to `0.0.0.0` in CI and Docker | Deferred (§ 4). If ever built: separate port, loopback only, disabled by default |
| **T7** | Supply chain of the new `toml_edit` dependency | RFC 033's existing `cargo-deny` gates apply; no new mechanism needed |

**T2 is the one to settle first**, because it is the only threat where
the safe answer is a *scope decision* rather than an implementation
detail — and because the machinery to do the unsafe thing already exists
and would be inherited by default.

### 9.4 Explicit non-threats

Stated so that nobody later assumes protection that was never offered:

- v6 does not defend the invoking user against commands they chose to
  run.
- v6 introduces **no change to how Rhai middleware executes**; whatever
  isolation exists today is unchanged. T2 concerns *reachability* — who
  can cause a script to be attached — not Rhai's own properties.
- The mock server remains a development tool. It is not hardened for
  hostile exposure on an untrusted network, and v6 does not change that.

### 9.5 What this demands of the design

1. The workspace root is a security boundary, enforced on every write.
2. `set` distinguishes **data** from **code**, and treats them
   differently.
3. When input is ambiguous, refuse — do not guess. This is the same rule
   § 1 derives from U2's failure mode, arrived at from a second
   direction.

## 10. Non-goals

- Changing the mock-serving hot path, or its performance characteristics.
- Replacing the GUI, or moving the GUI onto the CLI.
- A server-hosted configuration API (§ 4).
- An MCP server as a distinct architecture (§ 1, U5).
- Hardening the mock server for hostile network exposure (§ 9.4).
- Rewriting v5's config model. v6 exposes it; it does not redesign it.

## 11. RFC portfolio this spawns

Indicative, to be numbered when written.

| # | Subject | Note |
|---|---|---|
| 1 | `toml_edit` migration — preserve comments and ordering | § 8, first |
| 2 | CLI hygiene: `--version`, `--help`, reject unknown flags, fix relative `--config` | § 6, prerequisite |
| 3 | [**RFC 053**](./053-v6-cli-contract.md) — the contract: invocation, schemas, error taxonomy, exit codes | § 3. Drafted 2026-08-17 |
| 3b | **Threat model refinement** — settle T2 (data vs code) before `set` is designed | § 9, blocks RFC 5 |
| 4 | `get` — W1/W2/W3, including the "answered from" provenance | § 2, § 4 |
| 5 | `set` — W4/W6, preview, transaction boundary | § 2 |
| 6 | **Enumerate v6's breaking changes** — the list, not the designs | § 7.1, **blocks the final 5.x** |
| 7 | Deprecation warnings in 5.x, on stderr, with a named removal version | § 7.1, ships in v5 |
| 8 | Migration guide and the loud-failure requirement | § 7 |
| 9 | W7 bootstrap scenario as a CI-run acceptance test | § 2 |

## 12. Open questions

1. **Does `set` write through the existing `Workspace` model, or a
   narrower path?** `Workspace` was shaped for a long-lived GUI session;
   a CLI invocation is load-modify-save-exit. Reusing it avoids two
   writers; not reusing it may be simpler. Establish from the code.
2. **What is the transaction boundary?** One command, one file, one
   invocation, or an explicit batch?
3. **How does `get` express "no match"** — an error, or a successful
   answer whose content is "nothing matched"? Affects U3's exit-code
   handling directly.
4. **Does the GUI eventually consume the same contract**, or keep its
   library API? Not blocking, but it decides whether § 3 is one
   interface or two.
