# RFC 037 — README rethink

**Status.** Implemented (v5.16.0). Approved 2026-08-04
(`.git-exclude/reviewed/037-readme-rethink/REVIEW-001.md`).
**Amended 2026-08-04, amendment applied** — see § Amendment: the README
links the docs *root only*.
**Tracks.** M2 (Documentation and examples). The README is the project's
highest-traffic document and the crates.io landing page, and it has
drifted: it cites a version eight releases old, makes an unevidenced
performance claim, and omits the section its own structure rule
specifies.
**Touches.** `README.md` only. Nothing under `docs/src` — that is
RFCs 035 and 038.

## Summary

Rewrite `README.md` against the project's stated README structure and
RFC 034's page map: keep it short, make every claim true, and hand off to
the documentation site rather than competing with it.

**Release-bearing.** `crates/apimock/Cargo.toml` declares
`readme = "../../README.md"`, so this file *is* the crates.io landing
page and ships inside the published crate. Unlike RFCs 035 and 038, this
one changes a release artifact — it and RFC 036 define v5.16.0's content
(see `ROADMAP.md` § M2).

## Motivation

Surveyed 2026-08-03 against the shipped 5.15.0 README.

### 1. It cites a version eight releases old

`README.md:97` — *"skip every prompt and write the same defaults 4.7.0
wrote"*. `README.md:100` — *"existing non-interactive usage of 4.7.0
keeps working unchanged"*.

Both were written as release notes for 4.7.0 and never generalised. To a
5.15.0 reader they are meaningless at best; at worst a version-8-releases-
old reference reads as an abandoned project.

### 2. It makes a performance claim nobody can evidence

`README.md:52` — *"as validated with k6 load testing"*.

No k6 script, log, or result exists anywhere in the repository. This was
recorded as RISK-004 in the v5.14.0 handoff bundle, carried forward
through every subsequent review, and has never been substantiated.

The surrounding claims (no preloading, per-request non-blocking reads)
are true and verifiable from the code. Only the k6 attribution is
unsupported.

### 3. It omits the section its own structure rule specifies

The project's README structure is six sections, of which section 5 is
**Features / Design Notes** — *"Avoid redundancy. Features can be moved
to the full docs, leaving only Design Notes here."*

The current README goes from Vite integration straight to the docs link.
There is no statement of what apimock is like to work with — the
read-on-demand model, the dotted-path body syntax, the file-then-rules
dispatch order. A reader deciding whether to adopt it has to leave to
find out.

### 4. Acknowledgements are eight dependencies out of date

Listed: tokio, hyper, toml, serde, serde_json, json5, console, rhai,
thiserror, anyhow, mdbook.

Missing: `rustls`, `tokio-rustls`, `csv`, `regex`, `globset`, `ignore`,
`uuid`, `indexmap`. `rustls` in particular carries the entire TLS
surface.

### 5. It advertises one install path and badges another

The Quick start covers `npm install -D apimock-rs` / `npx apimock` only.
A crates.io badge and a docs.rs badge sit at the top, but `cargo install
apimock` appears nowhere — despite crates.io being the channel that has
stayed current (all four crates at 5.15.0).

### 6. Two shipped CLI subcommands are absent

`apimock validate` and `apimock match-test` shipped in RFCs 026 and 015.
Neither appears. The `validate` subcommand is the one most likely to
matter to the README's stated audience — CI pipelines.

## Goals

1. Every factual claim in the README is true and checkable.
2. The structure matches the project's own six-section rule.
3. It is *shorter*, not longer — competing with the docs site is the
   failure mode to avoid.
4. It serves the first-time reader deciding whether to adopt apimock,
   and hands off cleanly to the docs for everything else.

## Non-goals

- Anything under `docs/src` — RFCs 035 and 038.
- Examples — RFC 036, already implemented.
- Changing badges, logo, or the licence/acknowledgement policy beyond
  correcting the dependency list.
- Re-running or producing load-test evidence. § Motivation 2 is resolved
  by removing an unsupported claim, not by generating support for it —
  though if evidence is produced independently, the claim can return.

## Guide-level explanation

The README's job, in order: *what is this, why would I want it, how do I
start, what is it like to work with, where do I go next.* Everything else
belongs on the docs site.

Target structure, per the project rule:

| § | Section | Content |
|---|---|---|
| 1 | Hero | Badges, logo, catchphrase — largely as-is |
| 2 | Overview | What it is, in a few lines |
| 3 | Why / When | Use cases — as-is, it works |
| 4 | Quick start | **Both** install paths: `npx apimock` and `cargo install apimock`. Zero-config first response. `--init`. Vite integration. |
| 5 | **Features / Design Notes** | **New.** Read-on-demand model; file-then-rules dispatch; dotted-path body syntax, explicitly not JSONPath; the CLI subcommands. Design notes, not a feature list — the features live in the docs. |
| 6 | More detail | Links into the docs site, using RFC 034's section names |

## Reference-level explanation

### Required corrections

1. Remove both `4.7.0` references. Describe `--init --yes` by what it
   does, not by which release introduced it.
2. Remove *"as validated with k6 load testing"*. Keep the surrounding
   claims — they are true and code-verifiable.
3. Add § 5 Features / Design Notes.
4. Complete the Acknowledgements list.
5. Add `cargo install apimock` to Quick start.
6. Mention `apimock validate` and `apimock match-test` — briefly, in § 5,
   linking to the docs for detail.

### Link targets

RFC 034 restructures the docs site, and RFC 034 D6 declines to provide
redirects. **Every docs link in the README must point at a page that
exists in the new structure.** If RFC 035 or 038 has not yet landed the
target page, link the section index rather than a page that 404s.

This is the coupling that makes README work order-sensitive: it may be
written any time, but its links must be checked immediately before it
merges.

### Constraints

- The README ships in the crate. Relative links to repository files
  break on crates.io — use absolute URLs for anything outside the
  README itself. `docs/src/assets/logo.png` is already relative and
  already has this problem; fix it in passing.
- Keep it short. If a section wants to grow, that is a signal it belongs
  in the docs.

## Testing and verification

1. Every link resolves — including from a crates.io rendering, i.e. no
   repository-relative links.
2. No version reference to any release other than the current one.
3. No claim that cannot be traced to code or a checked artifact.
4. The six sections are present and in order.
5. Every crate in `[workspace.dependencies]` that ships in the binary
   appears in Acknowledgements.

## Drawbacks

1. **Removing the k6 claim weakens the pitch.** "Fast" without evidence
   is weaker marketing than "validated with k6". It is also honest. If
   the load test is re-run and recorded, the claim earns its place back.
2. **A new § 5 makes the README longer**, against the goal of shortening
   it. Mitigated by moving feature enumeration out to the docs, which
   should net out shorter.

## Rationale and alternatives

**Alternative A (chosen): rewrite in place against the structure rule.**

**Alternative B: minimal fix — correct the four defects, leave the shape
alone.** Cheaper and tempting. Rejected because § 5's absence is the
reason a reader cannot tell what apimock is like to work with, and that
is the README's main job.

**Alternative C: fold the README into the docs site and leave a stub.**
Rejected — it is the crates.io landing page and the GitHub front door;
both need it to stand alone.

## Unresolved questions

1. **Should the k6 claim be re-earned rather than removed?** Re-running a
   load test is out of this RFC's scope, but if the owner wants the claim
   kept, producing the evidence becomes a prerequisite rather than a
   follow-up. Default is removal.
2. **How much of § 5 is design notes versus a feature list?** The project
   rule says features move to the docs and only design notes stay. The
   line is a judgement call the implementer should propose and justify.

---

## Amendment 2026-08-04 — the README links the docs root only

### The problem this closes

`README.md` ships inside the published crate as the crates.io landing
page. **A published crate is immutable**, so whatever URLs are in the
README at release time are frozen forever.

The implemented version links two deep pages:

```
https://apimokka.github.io/apimock-rs/user-guide/
https://apimokka.github.io/apimock-rs/user-guide/configuration-reference.html
```

RFC 034's page map abolishes `user-guide/`. So once RFCs 035 and 038
restructure the site, a published README would point at URLs that no
longer exist — permanently, in that crate version.

### Decision

**Link the documentation root only** —
`https://apimokka.github.io/apimock-rs/` — and let the site's own
navigation carry the reader onward. Drop the deep links.

The root never moves under any restructuring, so the frozen artifact
stops depending on a mutable URL structure altogether. This holds
regardless of release ordering, which is why it is adopted independently
of the sequencing decision below.

Cost: a reader lands at the root rather than directly on the
configuration reference. Accepted — a small navigation step is worth a
link that cannot rot.

### What this does *not* fix

A reader following the root link still arrives at whatever the docs
currently say. Until RFCs 035 and 038 land, that is the pre-034 tree,
which states `service.strategy` is *"the only value supported today"*
and documents 5 operators against 49 in code.

This RFC removed an unverifiable performance claim on the principle that
every claim must be checkable. Pointing a reader at a page asserting four
shipped strategies do not exist is the same failure at one remove.

**That is why v5.16.0 waits for RFCs 035 and 038** (owner decision,
2026-08-04). See `ROADMAP.md` § M2.

### Implementation — exactly three edits

The amendment as first written said "drop the deep links" without saying
what replaces the second one. Resolved 2026-08-04, on escalation:

**1. `README.md:126`** — the inline docs link in § 5:

```diff
- See the [docs](https://apimokka.github.io/apimock-rs/user-guide/).
+ See the [docs](https://apimokka.github.io/apimock-rs/).
```

**2. `README.md:134`** — the Configuration Reference bullet. **Remove the
bullet entirely.** Do not repoint it at
`reference/apimock-toml-root-settings.html`: that is a page which exists
today, but pointing a *frozen* artifact at it reintroduces precisely the
coupling this amendment exists to remove. A deep link that happens to be
valid at publication is still a deep link.

**3. Preserve the signal, not the link.** Dropping the bullet silently
loses something real — the configuration reference is the single
most-consulted page, and a reader benefits from knowing it exists. So the
root link's sentence carries the mention instead:

```diff
- For more details, **🧭 check out our [full documentation](…/apimock-rs/)**.
-
- - Configuration Reference: 📋 [View all settings here](…/user-guide/configuration-reference.html)
+ For more details — including the complete configuration reference —
+ **🧭 check out our [full documentation](https://apimokka.github.io/apimock-rs/)**.
```

Net: **one docs URL in the whole README**, the root, which cannot rot.
The reader still learns a configuration reference exists and reaches it
through the site's own navigation.

### Scope note

`README.md` has been explicit non-change scope in every task this
milestone, deliberately. This amendment is executed under its own named
task and no other — the discipline is the reason the file has stayed
correct.

