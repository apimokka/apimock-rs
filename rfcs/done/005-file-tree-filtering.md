# RFC 005 — File tree filtering for `FileTreeView`

**Status.** Implemented (v5.8.0)
**Tracks.** Quality-of-life — keeping `FileTreeView` usable in real
project trees by filtering hidden / VCS / build-artifact directories.
This entry is the long-deferred ROADMAP item.
**Touches.** `apimock-routing` (`FileTreeView` builder,
`Workspace::list_directory`), `apimock-config` (filter configuration
shape if config-driven), documentation.

## Summary

`FileTreeView` currently lists every directory entry without filtering.
In real project trees this includes `.git/`, `node_modules/`, `target/`,
`.DS_Store`, OS-level swap files, and so on — useless clutter for a
respond-file picker. The original 5.3.0 design noted this as deferred
work. This RFC proposes a filter pipeline with sensible defaults, an
opt-out toggle, and (optionally) `.gitignore` honouring.

## Motivation

The respond-file fallback directory often sits inside a larger project
tree, where the tree picker needs to surface useful files without
forcing the user to scroll past hundreds of build artefacts. Concrete
pain cases from the deferred ROADMAP discussion:

- A user mounts the apimock workspace at the root of a Rust crate.
  The `target/` directory contains tens of thousands of files.
  Depth-1 enumeration is fine, but as soon as the user expands
  `target/`, the GUI becomes useless.
- A user has a Node-based mock data generator in the same tree. The
  `node_modules/` directory has the same effect.
- macOS users see `.DS_Store` files everywhere; Linux users see
  editor swap files (`.swp`, `~`).

There's no security or correctness issue — `Workspace::list_directory`
returns everything and the routing layer doesn't care — but the GUI
becomes unusable as a tree picker without filtering.

The deferral reasoning in the original ROADMAP was: the design space
is large (hardcoded list vs config vs `.gitignore` vs hybrid) and
user feedback would help. With stage-2 work resuming, this is the
right time to pick a shape.

## Guide-level explanation

Out of the box, `FileTreeView` hides the following on listing:

- Dotfile and dot-directory entries (`.git/`, `.env`, `.DS_Store`,
  etc.) — anything whose name begins with `.`.
- Known build-output directories: `target/` (Rust),
  `node_modules/` (Node), `dist/` (frontend), `build/`,
  `__pycache__/` (Python), `.venv/`, `vendor/` (Go).

A workspace-level config knob controls behaviour:

```toml
[file_tree_view]
show_hidden = false          # show dotfiles
respect_gitignore = false    # parse .gitignore at tree roots
extra_excludes = ["tmp/"]    # project-specific additions
include = ["*.json", "*.toml"]   # if set, only files matching these patterns
```

The GUI can also pass a per-call override to `list_directory`,
allowing a "show all" toggle without modifying config.

## Reference-level explanation

### Filter pipeline

```rust
pub struct FileTreeFilter {
    pub show_hidden: bool,
    pub builtin_excludes: bool,
    pub gitignore: bool,
    pub extra_excludes: Vec<String>,    // glob patterns
    pub include: Vec<String>,            // glob patterns; empty = include all
}
```

The filter applies in order:

1. If `show_hidden` is `false`, drop entries starting with `.`.
2. If `builtin_excludes` is `true`, drop entries matching the built-in
   list (loaded from a constant in the routing crate).
3. If `gitignore` is `true`, parse `.gitignore` files at each tree
   level on the fly and apply.
4. Drop entries matching any `extra_excludes` glob.
5. If `include` is non-empty, keep only entries matching at least one
   include glob. Directories are always kept (so the user can drill
   into them) — the include filter applies to files.

### API change

```rust
impl Workspace {
    pub fn list_directory(&self, path: &Path) -> Vec<FileNodeView>;

    // NEW: per-call override
    pub fn list_directory_with(
        &self,
        path: &Path,
        filter: &FileTreeFilter,
    ) -> Vec<FileNodeView>;
}
```

Both `FileTreeView` (depth-1 eager) and `list_directory` (on-demand)
honour the filter.

### Configuration source

The default `FileTreeFilter` comes from the workspace config (the new
`[file_tree_view]` TOML section). A `FileTreeFilter::default()` mirrors
the documented defaults so a config without the section behaves
sensibly out of the box. The GUI may override per call via
`list_directory_with`.

### Performance

`FileTreeView` enumerates depth-1 eagerly — small directory cost.
On-demand subdirectory listing is bounded by the user's clicks.
Filtering at depth-1 is O(n) per directory; well within the
existing budget.

`.gitignore` parsing is the only potentially expensive step: at each
directory level the workspace must locate and parse the `.gitignore`
file. The recommended path uses an established crate like `ignore`
which already caches parsed rules. If feasible, parse only at the
workspace root and inherit downwards; otherwise parse per level
following Git semantics.

### Built-in exclude list

Compiled into the routing crate as a `const &[&str]`:

```rust
pub const BUILTIN_EXCLUDES: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "out",
    "__pycache__",
    ".venv",
    "vendor",
    ".cargo",
    ".gradle",
    ".idea",
    ".vscode",
];
```

The list is intentionally conservative — entries that are
*overwhelmingly* build outputs across many ecosystems. Anything
project-specific goes through `extra_excludes`.

## Drawbacks

1. **Default behaviour changes silently.** Existing workspaces will
   suddenly hide directories they were previously listing. For most
   users this is the desired outcome; for users who depend on seeing
   `.git/` (unlikely but possible) it's a surprise. Mitigation: ship
   with a changelog entry and a `show_hidden = true` knob.
2. **Built-in list is opinionated.** Someone has a project named
   `target/` for legitimate reasons. `extra_excludes = []` and
   `builtin_excludes = false` lets them bypass; not a hard block.
3. **`.gitignore` parsing pulls in a non-trivial dependency.** The
   `ignore` crate (or similar) adds bytes to the binary. Acceptable
   when behind a feature flag if size matters; on-by-default if not.

## Rationale and alternatives

**Alternative A: hardcoded list only, no config.** Smallest
implementation. Loses flexibility for projects with unusual layouts.

**Alternative B: config-only, no built-in list.** Maximum flexibility;
the burden of curating the exclude list moves to every user.

**Alternative C (this RFC): built-in list as default + config overrides
+ optional gitignore.** Best of both: works out of the box for the
common case, configurable for the edge cases.

**Alternative D: pure `.gitignore` honouring, no list.** Elegant but
requires the workspace to live under a Git repository, which isn't
guaranteed.

We pick C. A is too rigid; B punts the problem; D is too narrow.

## Prior art

- `ripgrep` / `fd` use the `ignore` crate to combine `.gitignore`,
  hidden-file detection, and a built-in list. The `ignore` crate is a
  natural dependency choice if `respect_gitignore` ships.
- VS Code's file explorer uses a configurable `files.exclude` map
  plus `.gitignore`. Close to what this RFC proposes.
- The Tower Web file server has a simpler "no dotfiles" toggle. Too
  minimal for our needs.

## Unresolved questions

1. **Should `.gitignore` honouring be on or off by default?** Off is
   safer (no surprise hides), on is friendlier (matches user
   expectations from other tools). Probably off for v1, with a
   doc note encouraging users to enable it.
2. **Are the built-in excludes a `const` or are they loadable?**
   `const` is fine for now; making them loadable adds machinery for
   little gain. Future: a community-maintained list in a separate
   file if the surface area grows.
3. **Glob library choice.** `globset` is the obvious pick (same
   author as `ignore`) but `glob` is simpler and lighter. Pick at
   implementation time; benchmark before committing.
4. **Interaction with the existing `service.fallback_respond_dir`**.
   The respond fallback directory may itself live under a filtered
   path (e.g. `target/fixtures/`). The filter should *not* hide the
   configured fallback dir itself even if its parent matches the
   builtin list. Edge case to document.

## Future possibilities

- A `FileTreeFilter` builder API for the GUI to compose filters
  programmatically.
- "Recently used" sorting in `FileNodeView` so frequently-picked
  respond files float to the top — separate concern from filtering.
- Per-directory custom filter overrides (e.g. show everything under
  `fixtures/` even if a parent has filtering on).
- Filter performance telemetry — counts of files skipped vs shown,
  surfaced in diagnostics.
