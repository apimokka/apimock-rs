# RFC 062 — The v6 threat model, refreshed for a CLI that writes

**Status.** **Accepted** — approved by the project owner 2026-08-20.
**Not yet implemented.**
[Handed off](../handoffs/062-v6-threat-model/implementation-handoff.md) 2026-08-20,
with its open questions decided. Blocking for 6.0.0 as a decision.
**Tracks.** Security. Requested by the owner when the v6 concept was
settled; never done. **Blocking for 6.0.0** as a *decision*, not because
a live hole is known.
**Touches.** Documentation, and — depending on § Design's decision —
`crates/apimock/src/cmd/set.rs`.
**Depends on.** [RFC 048](../accepted/048-v6-cli-interface-concept.md)
§ 9 (T1/T2), [RFC 057](../accepted/057-set-command.md) (the write
surface).

## Summary

RFC 048 § 9's threat model was written before `set` existed. It covers
middleware (T2) and says nothing about a command that creates and
rewrites files at caller-supplied paths. Write the missing half, and
decide one question it raises.

## Motivation

### The model is older than the surface it should cover

RFC 048 § 9 settled **T2** — whether `set` may attach Rhai middleware —
and the owner's answer was no: *"Few profit with big maintenance cost."*
That was the right question for the design as it stood then.

Since then `set` shipped, and with it:

- creating `apimock.toml` where none exists,
- creating rule-set files,
- rewriting existing rule-set files in place,
- resolving `--rule-set` against `service.rule_sets`, canonicalising and
  falling back when the target does not exist yet.

**None of that is in any threat model.** The owner asked for a refresh
*"for v6 development"* when the concept was accepted; it never happened,
and the surface grew in the meantime.

### What I found probing it

Measured against the built binary:

| Probe | Result |
|---|---|
| `set --rule-set ../escaped.toml` | **Writes outside the config directory** |
| `set --rule-set /abs/path.toml` | **Writes at an absolute path** |
| `set --rule-set` at an existing **non**-rule-set TOML | **Refused, exit 2, file unchanged** ✅ |

The third result matters as much as the first two: **`set` cannot
destroy arbitrary file content.** A target that does not parse as a rule
set is rejected before anything is written.

So the capability is *"create a file, containing apimock rule-set TOML,
at a path the caller names"* — bounded by the user's own privileges.

### Is that a vulnerability? Not as such — and that is why it needs writing down

For a person at a shell, `--rule-set ../foo.toml` writing to `../foo.toml`
is not a flaw, it is what the flag means. `cp` behaves the same way.

**But v6's defining user is U2, the AI CLI agent**, and agents compose
commands from material that is not always trusted — a spec being
mocked, a filename in a task description, a path echoed back by a tool.
In that setting the flag is a file-creation primitive, and the argument
"the user asked for it" no longer straightforwardly holds, because the
user did not type it.

That is a **product decision about defaults**, not a bug, and it should
be made deliberately and written down rather than left as an emergent
property nobody has stated.

## Goals

1. A written threat model covering v6's actual surface: the CLI write
   path, config bootstrapping, path resolution, and what `set` may and
   may not reach.
2. **Decide the confinement question** (§ Design).
3. Restate the settled decisions (T2) so the document is complete
   rather than a diff against RFC 048.

## Non-goals

- Reopening T2. Middleware stays out of `set`; that is decided.
- A formal methodology. A page that says what the surface is, who the
  actors are, and what is deliberately allowed beats a framework nobody
  reads.
- Sandboxing, capability systems, or a permissions model. Out of
  proportion to a local development tool.

## Design

### The question to decide

**Should `set` confine writes to the config's directory tree by
default?**

**Option A — confine, with an explicit opt-out.** `set` refuses a
`--rule-set` that resolves outside the root config's directory unless a
flag (say `--allow-outside`) is given. A typo like `../../rules.toml`
fails loudly instead of silently creating a file somewhere unexpected.

**Option B — leave it open, and document it.** The flag means what it
says; confinement would surprise the legitimate case of a shared
rule-set directory beside the project.

**Recommendation: A.** The cost is one flag for an uncommon layout. The
benefit is that the common failure — an agent or a person naming a path
that is not what they meant — becomes visible instead of silent. This
project's own precedent points the same way: `--dry-run` was made to
*refuse* rather than half-write (RFC 057 REVIEW-001 § 4) on exactly this
reasoning, that a safety affordance which sometimes acts is worse than
one that declines.

Option B is defensible and cheaper, and the owner may prefer it — but it
should be chosen, not inherited.

### The document

A page under `docs/src/` — not a private note — covering:

- **Actors**: person at a shell, AI CLI agent (U2), CI, GUI, MCP host.
- **Surface**: what the server reads, what the CLI writes, what
  middleware can do, what TLS touches.
- **Deliberate allowances**, with reasons: file creation at named paths
  (subject to the above decision), Rhai's capabilities, verbose logging
  and what RFC 051 redacts.
- **Non-goals**, stated plainly: apimock is a development tool, not
  hardened for hostile input or multi-tenant use, and should not be
  exposed to untrusted networks.

That last section is the one that protects users best, because it tells
them what apimock is not.

## Testing and verification

- The three probes above become tests, whichever option is chosen —
  their *expected* results differ by option, but all three must be
  asserted rather than left to manual probing.
- If Option A: a `--rule-set` outside the tree is refused as `usage`,
  exit `2`, **and nothing is written**; the opt-out flag permits it; the
  existing-non-rule-set refusal keeps working.
- The document is reviewed against the actual code, not against RFC
  048's description of it. Where they disagree, the code wins and the
  discrepancy is itself a finding.

## Risks

| Risk | Mitigation |
|---|---|
| A threat-model document that is prose nobody acts on | The deliverable includes a decision and tests, not only a page |
| Option A breaks a legitimate layout | The opt-out exists; and the migration guide names the change |
| Writing it surfaces more issues than we want before 6.0.0 | The same trade as RFC 060: better found now. Triage with findings in hand |
| It becomes stale the way RFC 048 § 9 did | It lives in `docs/src/`, in the tree, reviewed with the code that changes the surface |

## Unresolved questions

1. **Option A or B** (§ Design). The only question that changes code.
2. **Does the GUI inherit whatever `set` does?** It calls the same
   `apimock-config` API, but confinement as designed here is a CLI-layer
   check. If the property should hold for every caller it belongs
   lower down, and that is a larger change worth knowing about now.
3. **Does `--file` (respond file paths) need the same treatment?**
   It is a read path, not a write path, so the exposure differs — but it
   takes a caller-supplied path and deserves a sentence either way.
