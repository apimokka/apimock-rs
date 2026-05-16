# RFC 012 — Config-driven FileTreeFilter via `[file_tree_view]` TOML section

**Status.** Implemented (v5.9.0)
**Tracks.** RFC 005 completion — wiring the `FileTreeFilter` struct
that RFC 005 introduced into the TOML config model so workspaces
can persist filter preferences across sessions without requiring
callers to pass a `FileTreeFilter` on every API call.
**Touches.** `apimock-config` (`config.rs`, new `file_tree_config.rs`,
`workspace/snapshot.rs`), `apimock-routing` (`view/build.rs` —
minor: `FileTreeFilter` already public), documentation.

## Summary

RFC 005 introduced `FileTreeFilter` and `build_file_tree_with` /
`list_directory_with` but did not add a TOML section to persist
filter settings. The `Workspace` API still calls `build_file_tree`
(default filter) unconditionally; a caller who wants a custom filter
must pass it on every `list_directory_with` call and cannot save
their preferences to `apimock.toml`.

This RFC adds a `[file_tree_view]` section to `apimock.toml` that
controls the filter applied by `Workspace::snapshot()` and
`Workspace::list_directory()`, matching the design sketched in
RFC 005's Guide-level explanation.

## Motivation

A project that sets its fallback respond dir to the repo root will
always see `target/` and `node_modules/` in `FileTreeView` unless
it passes a custom `FileTreeFilter` on every call. There is no way
to make this persistent in config — the user must re-specify it
every time they construct a `Workspace` or a GUI session starts.

Concretely:

- A GUI embedder calls `ws.snapshot()` and receives the filtered
  tree. It does not have a chance to inject a `FileTreeFilter`
  into `snapshot()` because that method takes `&self` with no
  extra parameters.
- A CLI tool that repeatedly calls `ws.list_directory(path)` must
  carry the filter in its own state.

Moving the filter into the config model solves both.

## Guide-level explanation

```toml
# apimock.toml

[file_tree_view]
show_hidden     = false        # hide dotfiles (default)
builtin_excludes = true        # hide target/, node_modules/, etc. (default)
extra_excludes  = ["tmp", "fixtures/generated"]
include         = ["*.json", "*.toml"]  # only show these; empty = show all
```

After this change, `Workspace::snapshot()` automatically applies
the configured filter; no API change is needed for callers that
already use `snapshot()`.

`Workspace::list_directory(path)` also uses the configured filter.
The existing `list_directory_with(path, filter)` override stays
available for one-off GUI toggles (e.g. a "show all" button).

## Reference-level explanation

### New config type (`apimock-config`)

```rust
// crates/apimock-config/src/config/file_tree_config.rs

use serde::Deserialize;

/// Persistent filter preferences for `FileTreeView`.
///
/// Deserialised from the `[file_tree_view]` section of `apimock.toml`.
/// Absent section → `FileTreeViewConfig::default()`, which mirrors
/// `FileTreeFilter::default()` (show_hidden=false, builtin_excludes=true).
#[derive(Clone, Deserialize)]
pub struct FileTreeViewConfig {
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_true")]
    pub builtin_excludes: bool,
    #[serde(default)]
    pub extra_excludes: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

fn default_true() -> bool { true }

impl Default for FileTreeViewConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            builtin_excludes: true,
            extra_excludes: Vec::new(),
            include: Vec::new(),
        }
    }
}

impl FileTreeViewConfig {
    /// Convert to the routing crate's `FileTreeFilter`.
    pub fn to_filter(&self) -> apimock_routing::FileTreeFilter {
        apimock_routing::view::build::FileTreeFilter {
            show_hidden: self.show_hidden,
            builtin_excludes: self.builtin_excludes,
            extra_excludes: self.extra_excludes.clone(),
            include: self.include.clone(),
        }
    }
}
```

### Config integration

`Config` gains:

```rust
pub struct Config {
    // … existing fields …
    pub file_tree_view: Option<FileTreeViewConfig>,
}
```

`Option` so the section is truly optional; `None` → default.

### Workspace snapshot update

`workspace/snapshot.rs` currently calls:

```rust
build_file_tree(fallback_dir)
```

After this RFC:

```rust
let filter = self.config
    .file_tree_view
    .as_ref()
    .map(|c| c.to_filter())
    .unwrap_or_default();
build_file_tree_with(fallback_dir, &filter)
```

`Workspace::list_directory` similarly:

```rust
pub fn list_directory(&self, path: &Path) -> Vec<FileNodeView> {
    let filter = self.config
        .file_tree_view
        .as_ref()
        .map(|c| c.to_filter())
        .unwrap_or_default();
    apimock_routing::view::build::list_directory_with(path, &filter)
}
```

### RootSettingKey additions (RFC 003 pattern)

Four new variants for GUI editing:

```rust
pub enum RootSettingKey {
    // … existing …
    FileTreeShowHidden,
    FileTreeBuiltinExcludes,
    FileTreeExtraExcludes,   // EditValue::StringList
    FileTreeInclude,          // EditValue::StringList
}
```

All return `ReloadHint::reload()` (no process restart needed —
next `snapshot()` call picks up the new filter).

### TOML writer update

`toml_writer` gains a `file_tree_view_table` helper that serialises
`FileTreeViewConfig` into the `[file_tree_view]` section, consistent
with how `[listener]`, `[log]`, `[service]` are already handled.

### Tests

1. **`snapshot_applies_configured_filter`** — Create a fallback dir
   with `users.json`, `.env`, and `node_modules/`. Config sets
   `show_hidden = false`, `builtin_excludes = true`. Assert snapshot
   tree contains only `users.json`.
2. **`snapshot_show_hidden_exposes_dotfiles`** — Same setup but
   config sets `show_hidden = true`. Assert `.env` appears.
3. **`snapshot_extra_excludes`** — Config sets
   `extra_excludes = ["generated"]`. Assert `generated/` is
   absent even though it is not in `BUILTIN_EXCLUDES`.
4. **`snapshot_include_filter`** — Config sets
   `include = ["*.json"]`. Assert only `*.json` files appear;
   `.toml` files are hidden.
5. **`list_directory_with_override`** — Call `list_directory_with`
   with `FileTreeFilter { show_hidden: true, .. }` on a workspace
   that has `show_hidden = false` in config. Assert override wins.
6. **`round_trip_file_tree_config`** — Apply
   `UpdateRootSetting { key: FileTreeShowHidden, value: Boolean(true) }`,
   save, reload. Assert `show_hidden = true` survives the round trip.

## Drawbacks

1. **One more config section.** `apimock.toml` grows a `[file_tree_view]`
   section. Users who don't use the GUI never need it — the section is
   optional and has sensible defaults.
2. **`Config` struct dependency on `FileTreeFilter` shape.** If
   `FileTreeFilter` gains new fields (e.g. `.gitignore` support), both
   `FileTreeViewConfig` and the TOML section must be updated. The
   `to_filter()` conversion makes this mechanical.

## Rationale and alternatives

**Alternative A: `FileTreeFilter` parameter on `Workspace::snapshot()`.**
Forces every caller to construct and pass a filter even if they just
want the configured default. Rejected — breaks the "snapshot is
zero-argument" contract that the GUI spec assumes.

**Alternative B (this RFC): config section + default from config.**
One source of truth for the filter; consistent with how listener,
log, and service are configured.

## Unresolved questions

1. **`.gitignore` honouring.** RFC 005 deferred this. If added later,
   it would be a new boolean field in `FileTreeViewConfig`. No action
   needed here.
2. **Per-directory overrides.** A user might want `show_hidden = true`
   only inside `fixtures/`. Out of scope; the per-call `list_directory_with`
   override is the escape hatch.
