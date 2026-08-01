# RFC 024 — Workspace external-change detection

**Status.** Implemented (v5.13.0)
**Tracks.** GUI reload flow — letting a GUI detect when the on-disk
config files have been modified outside the GUI session.
**Touches.** `apimock-config` (`workspace.rs`, `workspace/save.rs`
and sibling modules), documentation.

## Summary

`Workspace` currently has no way to tell the GUI that on-disk files
have changed since the last `load()` or `save()`. A user who edits
`apimock.toml` or a rule-set file in their text editor while the GUI
is open gets no notification; the GUI continues showing stale data.
This RFC adds two methods: `has_external_changes() -> bool` for
polling and `sync_from_disk() -> ConfigResult<()>` for re-loading.

## Reference-level explanation

### File-modification tracking

At `load()` time (and after each `save()`), `Workspace` records the
modification time and size of every config file it loaded:

```rust
struct FileMeta { modified: std::time::SystemTime, len: u64 }
workspace.file_metas: HashMap<PathBuf, FileMeta>  // new field
```

### `has_external_changes() -> bool`

Iterates `file_metas`, stats each path, and returns `true` if any
file's mtime or size differs from the recorded snapshot. Returns
`false` on stat errors (e.g. file deleted — reported as "no change"
to avoid spurious reloads on temp-file churn).

### `sync_from_disk() -> ConfigResult<()>`

Re-runs `Workspace::load` logic in-place:

1. Re-reads and re-parses every tracked config file.
2. Rebuilds the node ID table, preserving IDs for nodes whose
   identity (rule-set path, rule position) is unchanged.
3. Clears and refills `baseline_files`.
4. Updates `file_metas`.

Returns `Err` if any file fails to parse; in that case the workspace
is left unchanged (old data intact, error surfaced to the GUI).

### GUI usage pattern

```rust
// Periodic poll (e.g. every 2 seconds):
if ws.has_external_changes() {
    ws.sync_from_disk()?;
    let snap = ws.snapshot();
    // re-render GUI
}
```

## Unresolved questions

None — file-mtime polling is the simplest correct approach; a
file-system watcher is a future optimisation.
