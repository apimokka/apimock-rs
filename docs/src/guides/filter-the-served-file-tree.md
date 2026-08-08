# Filter the served file tree

**This does not filter what the running server serves over HTTP.**
`[file_tree_view]` governs the file-tree view returned by the
`Workspace` config-editing API — the surface GUI tooling uses to browse
a project's fallback-respond directory — not the
`service.fallback_respond_dir` request path itself. A file inside
`node_modules`, `.git`, or any pattern excluded here is still served
if a client requests its exact URL path. If you're trying to keep
specific files out of what `apimock` actually serves, this setting is
not the tool for that — there currently isn't one.

```toml
[file_tree_view]
show_hidden = false
builtin_excludes = true
extra_excludes = ["*.bak", "fixtures/"]
respect_gitignore = true
```

What it's actually for: a GUI editing a workspace's config needs to
show a file-tree view of the fallback directory without drowning the
user in `target/`, `node_modules/`, build artifacts, or files their own
`.gitignore` already excludes from the project. `[file_tree_view]`
tunes that view.

| Field | Effect on the editor's file-tree view |
|---|---|
| `show_hidden` | Include dotfiles/dot-directories |
| `builtin_excludes` | Apply the built-in list — `target`, `node_modules`, `dist`, `build`, `out`, `__pycache__`, `.venv`, `vendor`, `.cargo`, `.gradle`, `.idea`, `.vscode` |
| `extra_excludes` | Additional glob patterns, matched against each entry's bare name |
| `include` | An allow-list glob, files only |
| `respect_gitignore` | Also exclude anything `.gitignore` would, walking up from the listed directory |

Full field reference:
[`apimock.toml` root settings § `[file_tree_view]`](../reference/apimock-toml-root-settings.md#file_tree_view).
