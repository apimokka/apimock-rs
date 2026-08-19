# RFC 058 — `respond_dir` is resolved at load and written back, growing on every save

**Status.** **Accepted** — approved by the project owner 2026-08-19.
**Not yet implemented.**
**Tracks.** Correctness; released-version data integrity. Risk **R-10**.
**Touches.** `crates/apimock-routing/src/rule_set.rs`,
`crates/apimock-routing/src/rule_set/prefix.rs`,
`crates/apimock-config/src/toml_writer.rs`.
**Depends on.** Nothing. Interacts with
[RFC 041](../proposed/041-error-type-shape.md) — see § Public surface.

## Summary

`RuleSet::new` resolves `prefix.respond_dir` against the config
directory and writes the resolved value back into the same field the
user authored. `toml_writer` then persists it. Every load+save cycle
resolves the already-resolved value once more, so the field grows by one
`./` segment per save, without bound. Separate the authored value from
the resolved one, and persist only the authored one.

## Motivation

### Reproduction

In an empty directory, with a released binary's behaviour:

```
$ apimock set rule --path /a --status 200      # x1
respond_dir = "./."
$ apimock set rule --path /b --status 200      # x2
respond_dir = "././."
$ apimock set rule --path /c --status 200      # x3
respond_dir = "./././."
```

One segment per invocation. The config still loads — the paths
normalise — which is exactly why this has gone unnoticed.

### It is shipped, and the GUI hits it

Both halves of the mechanism are present at the **5.19.0 tag**:

- `git show 5.19.0:crates/apimock-routing/src/rule_set.rs` — the
  unconditional resolve-and-write-back, lines 106-127.
- `git show 5.19.0:crates/apimock-config/src/toml_writer.rs` — the
  writer emitting `respond_dir` whenever it is `Some`, lines 84-85.

This is **not** a regression from RFC 056's `toml_edit` work and not
new on `main`. Any caller that loads a workspace and saves it degrades
the file, and the GUI calls the same `Workspace::save()`. Since G2 has
the GUI offering to edit configuration files, it is a live data-integrity
defect in a released version.

`apimock set` did not cause this. It is the first caller that saves
often and non-interactively enough for anyone to notice — an argument
for having built the CLI, not against it.

### The actual defect: one field holding two different values

```rust
// crates/apimock-routing/src/rule_set.rs
let respond_dir_prefix = prefix.respond_dir_prefix.as_deref().unwrap_or(".");
let respond_dir_prefix =
    Path::new(current_dir_to_config_dir_relative_path).join(respond_dir_prefix);
...
prefix.respond_dir_prefix = Some(respond_dir_prefix.to_owned());
ret.prefix = Some(prefix);
```

`Prefix::respond_dir_prefix` is asked to be two things at once:

- **What the user wrote** — `respond_dir = "responses"`, relative to the
  rule-set file. A documented option
  (`docs/src/reference/rule-set-schema.md:44`, *"Prepended to every
  `respond.file_path` in this rule set"*).
- **What the matcher needs** — the same directory relative to the
  process's CWD, so `Respond::file_path` resolves at request time.

The load overwrites the first with the second. Nothing then remembers
what the user wrote, so the save has nothing correct to write back, and
the next load treats the resolved value as if it were authored.

**The two sibling fields are spared, and the contrast is the proof:**

| Field | Behaviour | Grows? |
|---|---|---|
| `url_path_prefix` | `.map(|p| normalize_url_path(p, None))` — transforms only what already exists | No |
| `service.fallback_respond_dir` | `compute_fallback_respond_dir` returns early when the value is still the default (`config.rs:171`) | No |
| `respond_dir_prefix` | defaults **and** resolves **and** writes back | **Yes** |

It is the only field with all three.

### A second defect, from the same line

`unwrap_or(".")` manufactures a value where the user wrote none, and
`ret.prefix = Some(prefix)` is unconditional. So a rule-set file with no
`[prefix]` section **gains one on its first save**. Confirmed: `set`
bootstraps a rule set as exactly `rules = []`
(`crates/apimock/src/cmd/set.rs:138`), and after one save the file
contains a `[prefix]` block with `respond_dir = "./."`.

RFC 056's guarantee is *preserve what people wrote*. Inventing a section
nobody wrote is the same class of violation as mangling one they did.

## Goals

1. `respond_dir` stops growing. A load+save cycle is a fixed point.
2. A rule set with no `[prefix]` section still has none after a save.
3. A user-authored `respond_dir = "responses"` is still exactly that
   after a save.
4. Request-time resolution of `Respond::file_path` is unchanged.

## Non-goals

- Changing what `respond_dir` means, or its documented semantics.
- Making `respond_dir` editable through `EditCommand`. Nothing exposes
  it today and this RFC does not add it.
- Repairing `url_path_prefix` or `fallback_respond_dir`. Both are
  correct; they appear here only as the contrast that isolates the bug.

## Design

### Separate the authored value from the resolved one

The codebase already solves this exact problem one type away:

```rust
// crates/apimock-routing/src/rule_set/rule/respond.rs
pub status: Option<u16>,        // what the user wrote
#[serde(skip)]
pub status_code: Option<StatusCode>,   // the resolved runtime form
```

`Respond` keeps the authored `status` untouched and puts the parsed form
in a separate field that never serialises. `Prefix` should do the same:
`respond_dir_prefix` stays exactly as authored, and the resolved
directory lives in a new field used only by the matcher.

**One implementation note that changes the mechanism.** `Prefix` derives
`Deserialize` only — there is no `Serialize`, and the write path is
`toml_writer`'s hand-built table (`toml_writer.rs:84-85`). So
`#[serde(skip)]` is not what keeps the resolved value out of the file;
the writer must simply not be given it. The precedent is the shape of
the fix, not its mechanism.

### The writer

`toml_writer::rule_set_table` emits `respond_dir` when
`prefix.respond_dir_prefix` is `Some`. Once that field holds only what
the user wrote, this becomes correct as it stands: absent stays absent,
authored stays authored. Goal 2 then follows from the load side no
longer manufacturing a value, not from a special case in the writer.

### Public surface

`Prefix` is `pub` and is **not** `#[non_exhaustive]`, so adding a field
is a breaking change — the R-09 class that RFC 052 closed for the trace
and request types and that RFC 041 proposes closing for the error enums.

Two consequences to settle during implementation: confirm whether
`Prefix` is externally reachable (it is `pub` within `rule_set`, but is
not re-exported from `apimock-routing/src/lib.rs`), and if it is, mark it
`#[non_exhaustive]` in the same change. 6.0.0 is the window for that, and
this fix is wanted inside it.

## Migration — files already grown

Files in the wild already carry `respond_dir = "./././."`. The fix stops
the growth but does not shorten what is there.

`./././.` and `.` are the same directory, so collapsing leading `./`
segments is semantically safe. **Recommendation: do not auto-rewrite.**
Silently editing a user's file to repair damage we caused is another
unrequested write, and the value is harmless where it stands. Note it in
the 6.0.0 migration guide, where a person can decide.

See § Unresolved 2 if we judge that too passive.

## Testing and verification

- **A load+save cycle is a fixed point.** Save three times; assert the
  file is byte-identical after each. This is the regression test the bug
  never had.
- A rule set with no `[prefix]` section still has none after a save.
- `respond_dir = "responses"` round-trips unchanged.
- `Respond::file_path` still resolves against the rule-set directory at
  request time — the behaviour the resolution exists for. Test through a
  real request, not by inspecting the field.
- A rule set whose `respond_dir` points somewhere that does not exist
  still fails `Prefix::validate` as it does today.
- Run the RFC 057 W7 script three times over and confirm the config is
  byte-stable after the first.

## Risks

| Risk | Mitigation |
|---|---|
| The matcher silently starts resolving against the wrong directory | The request-level test in § Testing, not a field-level assertion — the field is exactly what was wrong before |
| Adding a field to `Prefix` breaks a downstream caller | Establish reachability first; `#[non_exhaustive]` in the same change if it is public. 6.0.0 is the window |
| Some caller depends on reading the resolved value from `respond_dir_prefix` | Grep before moving it; this is why the resolved value keeps a name of its own rather than disappearing |
| Fix looks cosmetic and gets deprioritised | It is a released defect that degrades user files on every save, and the GUI triggers it. R-10 |

## Unresolved questions

1. **Is `Prefix` externally reachable?** It is `pub` but not re-exported
   from the crate root. If nothing outside can name it, adding a field
   is not breaking and the `#[non_exhaustive]` question is moot.
   Establish from source before designing around it.
2. **Should the fix collapse already-grown values?** Recommended no
   (§ Migration). The counter-argument is that we wrote the damage and a
   user who never asked for `[prefix]` should not have to clean it up.
   A middle option: collapse only when the value is purely `./` segments
   — provably equivalent to `.`, and never touching an authored path.
3. **Should `respond_dir` be persisted at all when it equals the
   default?** Goal 2 handles the never-authored case. This asks the
   narrower question of a user who wrote `respond_dir = "."` explicitly.
   Recommend preserving it — they wrote it, RFC 056 says keep it.
