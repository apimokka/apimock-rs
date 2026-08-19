# RFC 043 — Module split: `workspace/edit.rs`

**Status.** Proposed — awaiting owner approval.
**Tracks.** Maintainability. Non-breaking; no public surface changes.
**Touches.** `crates/apimock-config/src/workspace/edit.rs` and the
existing `crates/apimock-config/src/workspace/edit/` directory.
**Depends on.** Nothing. **Sequencing matters** — see § Risks.

## Summary

Split `workspace/edit.rs` — 1005 lines, the largest module in the
workspace — along the seams its own section comments already mark.
Purely internal: no public item moves, no behaviour changes.

## Motivation

### One module is an outlier; the rest of the tree is fine

Ranking every non-test module by lines of code (excluding inline
`#[cfg(test)]` blocks):

| Lines | Module |
|---|---|
| **1005** | `apimock-config/src/workspace/edit.rs` |
| 666 | `apimock-config/src/view.rs` |
| 621 | `apimock-config/src/toml_writer.rs` |
| 615 | `apimock/src/cmd/get.rs` |
| 570 | `apimock-server/src/trace.rs` |

`edit.rs` is **51% larger than the next module**, and it is the only one
that looks anomalous rather than merely long.

### Correcting this RFC's own premise

`ROADMAP.md` lists RFC 043 as *"Module split: `workspace/edit.rs`,
`server/trace.rs`"*. **The `trace.rs` half does not survive
measurement.** Its file is 1019 lines, but 449 of those are an inline
test module: 570 lines of code, fifth in the table above and unremarkable.

The raw line count made it look like a peer of `edit.rs`. It is not, and
a module that is 44% tests is showing a virtue rather than a problem.
**This RFC drops `server/trace.rs` from its scope** and proposes no work
there.

`view.rs`, `toml_writer.rs` and `get.rs` form a next tier, but none is an
outlier and splitting on size alone is churn. Left alone deliberately.

### The seams are already drawn

`edit.rs` is one `impl Workspace` block containing `apply()` and
eighteen `cmd_*` methods, already grouped by section comment:

| Region | Lines | Contents |
|---|---|---|
| `apply()` dispatcher | 58–138 | the match over `EditCommand` |
| Rule-set commands | 139–261 | `cmd_add_rule_set`, `cmd_remove_rule_set` |
| Rule commands | 262–519 | add / update / delete / move, `cmd_update_respond` |
| Root settings | 520–691 | `cmd_update_root_setting` — **171 lines, one function** |
| RFC 016 conditions | 692–946 | six header / body condition commands |
| RFC 025 strategy | 957–1005 | `cmd_update_rule_set_strategy` |

Six regions, marked by the author, aligned with the RFCs that added
them. **The split is not a new decomposition — it is the one already
written in the comments**, promoted from a comment to a module boundary.

And the pattern exists: `workspace/edit/payload.rs` (325 lines) is
already a submodule of exactly this module. This RFC continues a split
that was started and stopped.

## Goals

1. No module in `workspace/edit/` above ~300 lines.
2. Zero public API change — `Workspace::apply` keeps its signature; every
   `cmd_*` stays private.
3. Zero behaviour change, demonstrated by an unchanged test suite.

## Non-goals

- Splitting `trace.rs`, `view.rs`, `toml_writer.rs` or `get.rs`.
- Changing `EditCommand`, `ApplyError`, or the dispatcher's semantics.
- Moving tests. `workspace/tests/` is organised by RFC and by feature,
  which is a different and reasonable axis; leave it.
- Introducing a line-count lint. See § Unresolved 1.

## Design

```
workspace/edit.rs            apply() dispatcher + shared helpers   ~150
workspace/edit/payload.rs    (unchanged)                            325
workspace/edit/rule_set.rs   add/remove rule set, RFC 025 strategy ~170
workspace/edit/rule.rs       add/update/delete/move, respond       ~260
workspace/edit/root_setting.rs  cmd_update_root_setting            ~170
workspace/edit/condition.rs  RFC 016's six condition commands      ~255
```

Each file carries one `impl Workspace` block with its own methods.
`find_rule_indices` and any other shared helper stay in `edit.rs`, or
move to `edit/mod`-level visibility if more than one child needs them.

`cmd_update_root_setting` at 171 lines is a single function and stays
one; whether it wants breaking up is a question about that function, not
about module layout, and this RFC does not answer it.

## Testing and verification

This is a refactor, so the evidence is that **nothing changed**:

- Test count and results identical before and after. Report both numbers.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` clean.
- **No diff in the public API.** If RFC 039 has landed, its check proves
  this outright; until then, confirm no `pub` item changed module path in
  a way that alters its re-export.
- The diff is moves plus `use` lines. A reviewer should be able to read
  it as such — if a hunk contains a logic change, it does not belong in
  this RFC's commits.

## Risks

| Risk | Mitigation |
|---|---|
| **Conflicts with RFC 057's work in the same file** | Real and the main risk: RFC 057 adds `set` and may touch `edit.rs`'s neighbours. **Sequence this after RFC 057 lands**, not in parallel |
| A "pure move" quietly carries a logic change | Reviewed as moves-only; any logic hunk is out of scope by construction |
| Churn in `git blame` | `git blame` follows moves; and a 1005-line module already costs more to read than the blame history costs to trace |
| Splitting by size rather than meaning | The boundaries are the author's own section comments, not arbitrary line counts |

## Unresolved questions

1. **Should a module-size limit be enforced?** A lint would prevent
   recurrence, but line count is a poor proxy — `trace.rs` proves it,
   scoring high on raw lines while being perfectly healthy. Recommend
   no lint: fix the outlier, and revisit only if a second one appears.
2. **Does `cmd_update_root_setting` want decomposing?** 171 lines in one
   function is the largest single body in the crate. Out of scope here;
   worth looking at once it sits alone in its own file, where it is
   easier to judge.
