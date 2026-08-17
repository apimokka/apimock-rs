# Implementation Handoff — RFC 049, the CLI front door

**Governing RFC.** [RFC 049](../../done/049-cli-front-door.md)
**Umbrella.** [RFC 048](../../proposed/048-v6-cli-interface-concept.md) § 6
**Milestone.** Closing v5 — **blocks the deprecation release**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

Make the CLI refuse what it does not understand, and answer the two
questions every user asks first. No new capability.

**Why it blocks something.** RFC 048 § 7.1 commits to a deprecation
window in a 5.x release, where superseded invocations warn and name
their replacement. A CLI that silently discards what it does not
recognise cannot deliver a warning anyone acts on — so the deprecation
release waits on this.

## 2. Two of the three unresolved questions are decided

**Question 3 — does bare `apimock` still start a server? → YES,
unchanged.** Much of the documentation and every example depends on it.
Goal 1 concerns *unrecognised arguments*, not the absence of arguments.

**Question 1 — hand-rolled parser or a crate? → YOURS TO DECIDE, on
stated evidence.** I am not deciding it from here because the deciding
facts are in the code and the dependency graph, not in this document.

Judge it on two things and report both:

1. **Does every currently-valid invocation survive byte-identically?**
   This is the hard constraint. A crate that changes how `-p 4000` or
   `--init --yes` behaves fails the test regardless of what else it
   gives.
2. **What does it cost under the existing supply-chain policy?**
   ~~RFC 033's `cargo-deny` gates apply. Report the added dependency
   count and whether the licence allow-list needs touching.~~

   **Corrected 2026-08-17 — this was wrong.** There is no `deny.toml`
   and `cargo-deny` is not invoked anywhere: owner decision **D-04**
   dropped it on 2026-08-02, and `rfcs/done/033-supply-chain-gates.md`
   § 204 records that plainly. The real gate is `cargo audit`
   (`.github/workflows/ci.yaml`). The dev team checked rather than
   complied and reported the contradiction — see
   `.git-exclude/reviewed/049-cli-front-door/REVIEW-001.md` § 2. Left
   struck through rather than rewritten, so the record shows what was
   asked and that it was wrong.

My leaning, offered as input and not as instruction: v6 will restructure
this surface into subcommands, and a crate is the likely answer *then*.
Migrating once rather than twice is worth something. But it is worth
nothing if it perturbs a single existing invocation, and "we can always
adopt it in v6" is a perfectly good outcome to report.

**Question 2 — does `--dir` share the `--config` resolution fault? →
ESTABLISH FROM SOURCE.** Do not infer it from `--config`'s behaviour.
Test it, say what you found, and fix it there too if it is the same
fault. If it behaves differently, that difference is itself worth
reporting.

## 3. What to build

**Reject unknown arguments.** After known names are consumed, any
remaining argument beginning with `-` is an error naming it. Add a
near-match suggestion where one exists —
`unknown option '--prot'; did you mean '--port'?` That line is the
difference between a dead end and a self-correction, for a person and
for an agent alike, so treat it as part of the feature rather than
polish.

**`--version` and `--help` short-circuit early** — before configuration
is read and before any listener is bound. They must work in a directory
with no config file, and in one with a deliberately broken config.
"What version am I running" is asked precisely when things are wrong.

**Exit codes**, applied across the whole CLI:

| Code | Meaning |
|---|---|
| 0 | Success, including `--version` / `--help` |
| 2 | Usage error — unknown option, missing or invalid value |
| 1 | Everything else |

**Stream discipline.** `--version` / `--help` on stdout; every
diagnostic on stderr.

**Path resolution.** A bare `--config apimock.toml` resolves like
`./apimock.toml`.

## 4. This sets conventions v6 inherits — get them right

The exit codes and the stdout/stderr split are not local decisions.
RFC 048 § 3 makes them part of v6's machine-readable contract, and the
deprecation warnings land on stderr **because of the rule established
here**. If a choice in this RFC looks arbitrary, it is probably the one
v6 will be stuck with — raise it rather than picking quietly.

## 5. Scope boundaries

- **Out:** restructuring into subcommands (v6's decision — doing it here
  breaks invocations before the window exists to announce them); `get` /
  `set`; changing what any *valid* invocation does.
- **In:** `crates/apimock/src/args.rs`, `args/constant.rs`, and the
  documentation this falsifies.
- If the work starts reaching into config loading or the server, stop
  and escalate.

## 6. Documentation — changes in the same commit, not after

`docs/src/reference/cli-reference.md:14` currently documents the `-c`
quirk as current behaviour:

> **Prefix relative paths with `./`** — a bare filename … currently
> fails to resolve …

That sentence becomes false the moment goal 4 lands. It must be updated
in the same change, and the new `--version` / `--help` / exit-code
behaviour documented alongside. Check the example sets and `README.md`
for anything else this falsifies, and name the files you checked.

RFC 048 § 5 records that documentation ranks with the API design here,
so this is a deliverable rather than a follow-up.

## 7. Evidence required

- Unknown option → **exit 2**, message on **stderr**, and **no server
  started**. Show the last part explicitly; it is the actual defect.
- Near-match suggestion appears for a plausible typo.
- `--version` and `--help` → exit 0, stdout, no server started — proven
  **three times**: normal workspace, no config file, and a deliberately
  invalid config.
- `-c apimock.toml` and `-c ./apimock.toml` resolve identically.
- **Every currently-valid invocation behaves exactly as before.**
  Enumerate them from `args/constant.rs` and show each one. This is the
  regression that matters most: the point is to reject only what was
  already meaningless.
- Full suite green; report the count against the 415 baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 8. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including a § 2 finding that
contradicts the RFC, and the § 4 concern if a convention looks wrong.
