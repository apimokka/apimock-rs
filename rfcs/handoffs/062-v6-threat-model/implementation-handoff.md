# Implementation Handoff — RFC 062, the v6 threat model

**Governing RFC.** [RFC 062](../../accepted/062-v6-threat-model.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0 — **blocking**
**Self-contained.** Everything you need is here. RFC 062 is the
authority; if the two disagree, report it rather than following this.

---

## 1. What this is

Two deliverables: a **document** describing v6's actual security
surface, and one **behaviour change** that follows from it.

RFC 048 § 9's threat model was written before `apimock set` existed. It
settled T2 — `set` may not attach Rhai middleware — and says nothing
about a command that creates and rewrites files at caller-supplied
paths, which is what shipped.

**There is no known live hole.** This is about stating deliberately what
is currently true by accident.

## 2. What I measured, so you do not have to rediscover it

Against the built binary:

| Probe | Result |
|---|---|
| `set --rule-set ../escaped.toml` | **writes outside the config directory** |
| `set --rule-set /abs/path.toml` | **writes at an absolute path** |
| `set --rule-set` at an existing **non**-rule-set TOML | **refused, exit 2, file unchanged** ✅ |

The third bounds the exposure and matters as much as the first two:
**`set` cannot destroy arbitrary file content.** A target that does not
parse as a rule set is rejected before anything is written. Verify all
three yourself before changing anything — they are the before-state.

So the capability is *"create a file containing apimock rule-set TOML at
a path the caller names"*, bounded by the user's own privileges.

## 3. The decision, made: confine by default (Option A)

For a person at a shell, `--rule-set ../foo.toml` writing to
`../foo.toml` is not a flaw — it is what the flag means, and `cp`
behaves the same.

**But v6's defining user is U2, the AI CLI agent**, which composes
commands from material it did not author: a spec being mocked, a
filename in a task description, a path echoed back by another tool. In
that setting the flag is a file-creation primitive, and *"the user asked
for it"* stops holding, because the user did not type it.

**Implement Option A:**

- `set` **refuses** a `--rule-set` (and any other caller-supplied write
  path) that resolves outside the root config's directory tree.
- Refusal is `usage`, **exit 2**, message naming the path and the
  opt-out — and **nothing is written**, including bootstrap files.
- An explicit opt-out flag permits it. `--allow-outside` is the working
  name; pick a better one if you see it and say why.
- Resolution is by canonicalised path where the target exists, and by
  canonicalised parent where it does not — because `set` legitimately
  creates files that do not exist yet, and a naive check would break
  bootstrapping.

**Why refusing rather than warning:** this project already made the same
call for `--dry-run`, which was changed to refuse rather than half-write
(RFC 057 REVIEW-001 § 4). A safety affordance that sometimes acts is
worse than one that declines, because the exception is invisible at the
call site.

### Two scope decisions that come with it

**The check is CLI-layer, not library-layer.** `apimock-config` keeps
today's behaviour, so **the GUI does not inherit confinement**. That is
a deliberate limit of this RFC, not an oversight — pushing it into
`Workspace` would change a published API and affect a caller that is not
U2. **Record it in the document as a known asymmetry.** If it should
hold for every caller, that is a follow-up RFC.

**`--file` is out of scope.** It is a read path, not a write path, so
the exposure differs. Give it a sentence in the document saying so; do
not confine it here.

## 4. The document

A page under `docs/src/` — public, in the tree, reviewed alongside the
code that changes the surface. RFC 048 § 9 went stale precisely because
it lived inside an RFC nobody revisits.

Cover:

- **Actors**: person at a shell, AI CLI agent (U2), CI, GUI, MCP host.
- **Surface**: what the server reads, what the CLI writes, what
  middleware can do, what TLS touches.
- **Deliberate allowances, with reasons** — file creation at named paths
  and its new confinement, Rhai's capabilities, verbose logging and what
  RFC 051 redacts.
- **Settled decisions restated**, not diffed: T2 (`set` may not attach
  middleware) belongs here in full, so the page stands alone.
- **Non-goals, stated plainly**: apimock is a development tool, not
  hardened for hostile input or multi-tenant use, and should not be
  exposed to an untrusted network.

**That last section protects users most**, because it tells them what
apimock is not. Write it first if it helps.

## 5. Evidence required

- The three probes in § 2, re-run after the change, with their **new**
  expected results asserted as tests — not left to manual probing.
- `--rule-set` outside the tree: `usage`, exit 2, **nothing written**
  (check the directory afterwards, not just the exit code).
- The opt-out flag permits it, and is itself covered by RFC 059's
  conformance table.
- **Bootstrapping still works** — `set` in an empty directory creates
  its config, since that is a write to a path that does not exist yet
  and a naive confinement check would break it. This is the regression
  most likely to bite.
- The existing-non-rule-set refusal still behaves as before.
- The document exists, is linked from the docs nav, and `mdbook build
  docs` is clean.
- Full suite green with the count against `main`'s baseline; `fmt`;
  `clippy -D warnings`.

## 6. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: confinement cannot be expressed without duplicating path
logic that belongs in `apimock-config` (that would reopen § 3's
CLI-layer decision); a legitimate layout is broken by it that the
opt-out does not cover; or writing the document surfaces a surface I
have not listed — that last one is the most valuable thing this RFC
could produce, and it should arrive as a finding rather than a quiet
edit.
