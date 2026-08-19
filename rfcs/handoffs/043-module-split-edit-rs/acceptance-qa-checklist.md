# Acceptance / QA Checklist — RFC 043, module split of `workspace/edit.rs`

**Governing RFC.** [RFC 043](../../accepted/043-module-split-edit-rs.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md) —
the regions, the target layout, and what is out of scope.

This is a refactor, so every box is a form of "nothing changed".

## A. Nothing changed

- [ ] **Test count identical before and after.** Report both numbers,
      not "all green".
- [ ] All tests pass, and the same tests exist — none renamed, moved or
      dropped.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- [ ] `apimock get`, `set`, `validate` and `match-test` behave as before.

## B. The diff reads as moves

- [ ] The diff is moves plus `use` lines and `mod` declarations.
- [ ] **Any hunk that is not a pure move is called out explicitly**, with
      why it was unavoidable. If there are none, say so.
- [ ] No logic change, no signature change, no renamed method.

## C. Public surface untouched

- [ ] No `pub` item changed module path in a way that alters its
      re-export. State how this was checked.
- [ ] Every `cmd_*` method is still private.
- [ ] `Workspace::apply` keeps its exact signature.

## D. The layout goal

- [ ] `edit.rs` is roughly 150 lines — dispatcher plus shared helpers.
- [ ] No file under `workspace/edit/` exceeds ~300 lines.
- [ ] `payload.rs` and `id_shift.rs` are untouched.
- [ ] `cmd_update_root_setting` is still **one function**, now in its own
      file.

## E. Scope held

- [ ] `trace.rs`, `view.rs`, `toml_writer.rs` and `get.rs` are untouched.
- [ ] **No module-size lint was added.** RFC 043 declined one on purpose.
- [ ] `workspace/tests/` is unchanged — tests were not reorganised.
- [ ] `EditCommand`, `ApplyError` and dispatcher semantics unchanged.
