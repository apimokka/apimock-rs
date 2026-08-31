# 6. Known gaps, open questions, and what we want from you

Written so you meet these in a document rather than in your own
debugger. Nothing here is hidden in a "limitations" footnote elsewhere.

## 1. The public module surface is wider than anyone designed

`apimock-server` has **16 of 16** top-level modules `pub mod`.
`apimock-routing`, **6 of 6**. So `apimock_server::http_util`,
`::dyn_route`, `::response_handler`, `apimock_routing::util::glob` and
many others are public API.

**None of it reads as a designed external surface.** It reads as
nothing ever having narrowed it — and it was only noticed when RFC 039's
baseline put the whole public API in one file for the first time.

**What this means for you, concretely:**

- **You may find something useful in there.** It is genuinely public
  and genuinely gated, so depending on it will not break within 6.x.
- **But narrowing it is a live 7.0 candidate.** If you build on
  `apimock_server::http_util`, you may be building on something we
  intend to make private.
- **So tell us what you use.** If a module you depend on is on the
  narrowing list, we would rather know before deciding than after. And
  if something in there is genuinely valuable to you, that is an
  argument for keeping it public *deliberately* — which is a better
  outcome than either of us guessing.

## 2. Three surfaces have never had a real consumer

Restating the README's table because it is the most actionable thing
here:

- `Workspace::has_external_changes()` / `sync_from_disk()`
- `apimock_server::control::{ServerControl, ServerHandle, ServerState}`
- `apimock_server::trace::TraceEmitter` (the subscription side)

Unit-tested inside their crates; never driven by an application.
**Spike these early.** Their shape is a design in draft and you are the
first review.

Specific unknowns we would like answered:

| Question | Why it matters |
|---|---|
| Does `sync_from_disk()` preserve unsaved in-memory edits or discard them? | Decides what you can offer a user on external change |
| Is `has_external_changes()` cheap enough to poll? | Decides whether you need a filesystem watcher |
| How does a consumer *receive* what `TraceEmitter::emit` sends? | The live feed is useless without a clear subscription side |
| Does `ReloadHint::Reload` genuinely avoid a restart in every case it claims? | Decides whether your GUI can apply edits without dropping the listener |

## 3. `[guard]` is a stub

`apimock_routing`'s `Guard` is an empty struct with a `// todo:`. The
rule-set schema documents it honestly as not yet doing anything. **Do
not build UI for it**; there is nothing behind it.

## 4. `--strict` and validate's exit `1` are unreachable

Documented in `docs/src/reference/cli-reference.md`: `Workspace::load`
rejects every condition that could become a `Severity::Error`, so a
config either loads clean or fails to load. **Nothing constructs a
`Severity::Warning` anywhere.**

**The consequence for your UI:** `ValidationReport`'s severity axis is
real in the type and currently single-valued in practice. If you build
a warnings-vs-errors distinction, you will be building the first thing
that needs warnings to exist. That is a reasonable thing to want — tell
us and it becomes a real design question rather than a latent one.

## 5. Two validators, one of which is yours

Covered in [`02-editing-configuration.md`](./02-editing-configuration.md),
repeated because it is the kind of thing that produces a confusing bug
report: `apimock-config`'s node validation is a second implementation
of `apimock-routing`'s `Respond::validate`. They are now covered by an
agreement test. If they ever disagree in your hands, it is our bug.

## 6. Documentation aimed at a library consumer barely exists

`docs/src/` is written for someone running the CLI. There is
`how-it-works/workspace.md` on the crate split, and rustdoc on the
crates themselves — but there is **no library-consumer guide**, and
this package is the closest thing to one.

**If you find yourself writing internal notes to explain our API to
your own team, those notes are the guide we are missing.** We would
take them gratefully, in whatever state they are in.

## What we would like back

In rough order of value to us:

1. **Which of § 1's public modules you actually depend on.** Directly
   shapes a 7.0 decision.
2. **Answers to § 2's four questions**, from real use.
3. **Anywhere a `#[non_exhaustive]` type needs a constructor we did not
   provide.** Cheap for us, additive, unblocks you.
4. **Any `EditCommand` your GUI needs and cannot express.** Also
   additive; the alternative is you writing TOML behind the workspace,
   which we would rather you never do.
5. **Anywhere this package is wrong.** It was written against the
   6.0.0 baselines and checked as it went, but it is long and we have
   been wrong before.

## How to reach us

- **Design questions and gaps** — a GitHub issue is fine.
- **Security** — not an issue; see
  [`05-security.md`](./05-security.md).
- **Anything that should change a documented contract** — say so
  explicitly. Changes to the public API go through the RFC process
  (`rfcs/README.md`), which exists partly so a consumer's request lands
  as a design decision with reasoning rather than an undeclared change.
