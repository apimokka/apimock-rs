# Implementation Handoff — RFC 043, module split of `workspace/edit.rs`

**Governing RFC.** [RFC 043](../../accepted/043-module-split-edit-rs.md)
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0. **Non-breaking** — no public item moves
**Sequencing.** RFC 057 has **landed**, so this is unblocked. See § 5.
**Self-contained.** Everything you need is here. RFC 043 is the
authority; if the two disagree, report it rather than following this.

---

## 1. What this is, and what it is not

`workspace/edit.rs` is **1005 lines of code** — the largest module in the
workspace by a wide margin, 51% above the next. Split it along the seams
its own section comments already mark.

**This is a pure refactor.** No public item moves, no behaviour changes,
no signature changes. The evidence that it worked is that *nothing
happened*: same tests, same results, same public API.

If a hunk in your diff contains a logic change, it does not belong in
this work. That is the single most important sentence here — a refactor
that smuggles in a fix is a refactor nobody can review.

## 2. The seams are already drawn

`edit.rs` is one `impl Workspace` block: `apply()` plus eighteen `cmd_*`
methods, already grouped by section comment and aligned with the RFCs
that added them.

| Region | Lines | Contents |
|---|---|---|
| `apply()` dispatcher | 58–138 | the match over `EditCommand` |
| Rule-set commands | 139–261 | `cmd_add_rule_set`, `cmd_remove_rule_set` |
| Rule commands | 262–519 | `cmd_add_rule`, `cmd_update_rule`, `cmd_delete_rule`, `cmd_move_rule`, `cmd_update_respond` |
| Root settings | 520–691 | `cmd_update_root_setting` — **171 lines, one function** |
| Conditions (RFC 016) | 692–956 | six header / body condition commands, plus `find_rule_indices` |
| Strategy (RFC 025) | 957–1005 | `cmd_update_rule_set_strategy` |

**You are not inventing a decomposition.** You are promoting the
author's own section comments from comments to module boundaries.

The pattern already exists: `workspace/edit/` holds `payload.rs` (325
lines) and `id_shift.rs`. This continues a split that was started and
stopped.

## 3. Target layout

```
workspace/edit.rs              apply() dispatcher + shared helpers   ~150
workspace/edit/payload.rs      (unchanged)                            325
workspace/edit/id_shift.rs     (unchanged)
workspace/edit/rule_set.rs     add/remove rule set, RFC 025 strategy ~170
workspace/edit/rule.rs         add/update/delete/move, respond       ~260
workspace/edit/root_setting.rs cmd_update_root_setting               ~170
workspace/edit/condition.rs    RFC 016's six condition commands      ~255
```

Each file carries its own `impl Workspace` block with its own methods —
Rust allows multiple `impl` blocks for a type across modules in the same
crate, which is what makes this a move rather than a redesign.

`find_rule_indices` and any other shared helper stay in `edit.rs`, or
move to a visibility the children can reach if more than one needs them.
Keep every `cmd_*` **private**; none of them is public today and none
becomes public.

**`cmd_update_root_setting` stays one function.** At 171 lines it is the
largest body in the crate, and whether it wants breaking up is a
question about that function, not about module layout. Out of scope —
see § 6.

## 4. Target: no module above ~300 lines

That is the goal, not a lint. RFC 043 § Unresolved 1 explicitly declined
to add a line-count lint, because line count is a poor proxy: `trace.rs`
scores 1019 raw lines while being perfectly healthy, since 449 of them
are an inline test module. **Do not add a size lint.**

For the same reason, `server/trace.rs` was **dropped from this RFC's
scope** — the ROADMAP one-liner named it, but it is 570 lines of code,
fifth largest and unremarkable. `view.rs`, `toml_writer.rs` and `get.rs`
form a next tier and are also deliberately left alone. **Split only
`edit.rs`.**

## 5. Sequencing — why this waited

RFC 043 was held until RFC 057 landed, because `set` touches this
neighbourhood and a large file-move in parallel would have produced
conflicts nobody could review cleanly. **057 is now merged**, so you are
clear.

Rebase before you start, and do the move in one focused pass rather than
spread across days — the longer this sits, the more it collides with
anything else touching `edit.rs`.

## 6. Scope

**In:** `crates/apimock-config/src/workspace/edit.rs` and new files under
`crates/apimock-config/src/workspace/edit/`. The `mod` declarations. The
`use` lines each new file needs.

**Out:** `trace.rs`, `view.rs`, `toml_writer.rs`, `get.rs` (§ 4). Any
change to `EditCommand`, `ApplyError`, or dispatcher semantics.
Decomposing `cmd_update_root_setting`. Moving tests — `workspace/tests/`
is organised by RFC and by feature, a different and reasonable axis.
A module-size lint.

## 7. Evidence required

The evidence is that nothing changed:

- **Test count and results identical before and after.** Report both
  numbers explicitly; "all green" is not the same claim.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` clean.
- **No public API change.** No `pub` item changed module path in a way
  that alters its re-export. State how you checked.
- **The diff is moves plus `use` lines.** A reviewer should be able to
  read it as such. Call out explicitly if any hunk is not a pure move,
  and why it was unavoidable.
- No file under `workspace/edit/` above ~300 lines, and `edit.rs` itself
  around 150.
- Every `cmd_*` is still private.

## 8. Escalation

Blocking issues and design questions go in a
`.git-exclude/review-request/` package.

Escalate if: a pure move turns out to be impossible somewhere without a
visibility change that widens a public surface; or if moving a method
reveals a genuine bug. **Do not fix the bug in this change** — report it,
and it gets its own RFC. Mixing a fix into a move is exactly what makes
refactors unreviewable, and finding one would be a good outcome worth
recording separately.
