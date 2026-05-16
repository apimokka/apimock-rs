# Roadmap

This file records design questions that have been identified during
development but intentionally postponed to a later release. Items
here are *not* bugs — they're follow-on work whose right shape is
easier to decide after some related primary feature has shipped.
Recording the rationale here prevents the original context from being
lost between releases.

## Deferred items

### Hidden / VCS / build-artifact directory filtering in `FileTreeView`

**Identified during:** 5.3.0 design discussion (after `FileTreeView`
was added as part of routing snapshot enrichment, spec §5.5).

**Status:** Deferred. No release scheduled. Pick up when there's
concrete user feedback on what clutters the GUI tree view.

**Description.** `Workspace::snapshot()` produces a `FileTreeView` of
the fallback respond directory. As of 5.3.0, the depth-1 eager
population strategy enumerates the top-level directory verbatim — no
entries are filtered. A `.git`, `node_modules`, `target`, or
`.DS_Store` entry that happens to live at the top level of the
fallback respond dir appears alongside the user's mock data.

**Why this is deferred.**

- *Performance is not affected.* depth-1 eager only lists the top
  level; subdirectory contents are loaded on demand when a GUI
  explicitly calls `Workspace::list_directory(parent_id)`. A `.git`
  entry has the same display cost as any other single directory
  entry — the heavy contents are never enumerated unless the user
  clicks to expand. The "hidden folder + lazy expansion" combination
  doesn't compound into a pathological case.
- *No safety risk.* apimock is a mock-server development tool, so a
  rendered `.git/config` doesn't escalate to anything more serious
  than a noisy GUI. The runtime `dyn_route` fallback that serves
  files matching incoming URL paths has always been
  filter-agnostic, and retroactively filtering it would break
  legitimate uses such as serving `/.well-known/security.txt`.
- *No agreed shape.* Candidate filtering strategies — dotfile prefix,
  hardcoded denylist, `.gitignore` parsing, user-configurable
  patterns — each have trade-offs that are easier to evaluate once
  there's a real GUI built against `FileTreeView` and concrete
  feedback on what users find annoying.

**Suggested approach when picked up.**

A two-step plan that doesn't lock in long-term policy:

1. Apply a minimal default filter (dotfile prefix only — entries
   whose `file_name()` starts with `.`) to `FileTreeView` only.
   Leave `dyn_route` untouched.
2. Make the filter override-able via `Workspace::load_with_options`
   or a builder, so a GUI that wants to show dotfiles can opt out.

This belongs in the routing crate, ideally co-located with the
existing file-tree builder.

### Header / body.json round-trip through `toml_writer`

**Identified during:** 5.2.0 implementation.

**Status:** Deferred. Pending routing-crate changes.

**Description.** Rule-set rules with `headers` or `body.json` match
conditions parse cleanly into the in-memory model but are *not*
re-serialised by `apimock_config::toml_writer`. Saving a rule set
that contains such conditions drops them from the on-disk file.

**Why this is deferred.** The routing crate's `Headers` and `Body`
types don't expose their internal map shape outside the crate, so
the writer can't read them. Adding accessors is a routing-crate API
change with its own design decisions (which structure to expose:
the original TOML form, or the parsed-and-validated form?). Better
to address as a focused change in a routing-crate-only release than
to mix it into a config-crate release.

**Suggested approach.** Add `Headers::iter()` and a public accessor
on `Body` that yields the JSON-path / value pairs. Then extend
`toml_writer::request_table` to read them. Add round-trip tests in
`apimock-config` covering rule sets with both kinds of condition.
