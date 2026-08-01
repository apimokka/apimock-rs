# RFC 019 — File tree filter: `.gitignore` honouring and glob excludes

**Status.** Implemented (v5.11.0)
**Tracks.** RFC 005 completion — implementing the two filter
features that RFC 005 specified but the v5.8.0 + v5.9.0 (RFC 012)
implementation did not deliver: `.gitignore` honouring and glob
pattern support for `extra_excludes`.
**Touches.** `apimock-routing` (`view/build.rs` — `FileTreeFilter`,
`build_file_tree`, `list_directory`), `apimock-config`
(`file_tree_config.rs` — `[file_tree_view]` TOML section,
`RootSettingKey` variants), `Cargo.toml` (new dependencies),
documentation, examples.

## Summary

RFC 005 specified a `FileTreeFilter` shape with three filtering
mechanisms — built-in excludes, glob-pattern `extra_excludes`, and
optional `.gitignore` honouring. The v5.8.0 implementation shipped
the first; the v5.9.0 implementation (RFC 012) made the filter
config-driven. Two pieces remain:

1. **`extra_excludes` is exact-name match, not glob.** RFC 005 wrote
   `extra_excludes = ["tmp/"]` and described it as "glob patterns".
   The code does plain `name == pattern` comparison.
2. **`.gitignore` honouring is unimplemented.** No `gitignore` field
   on `FileTreeFilter`, no parsing code, no opt-in TOML key.

This RFC closes both gaps with a small dependency addition
(`globset` + `ignore`) and four small API/config changes.

## Motivation

The two missing features matter in slightly different ways.

**Glob `extra_excludes` is a daily-use feature.** Real exclusion
lists are pattern-shaped: `*.log`, `*.tmp`, `cache-*/`, `*~`.
Without glob support, users have to enumerate every literal name,
which is impractical for editor-generated files (`.swp`, `.swo`,
`.swn`, …) and impossible for templated names. The current
exact-match implementation forces users into a list maintenance
loop the original RFC explicitly aimed to avoid.

**`.gitignore` honouring is occasional but high-value when it
applies.** A mock-server workspace sitting under a Git repository
often inherits a well-curated `.gitignore` — build outputs, cache
dirs, language-specific patterns. Re-deriving the same list in
`[file_tree_view].extra_excludes` is duplication. The opt-in nature
of this feature (off by default) keeps surprise low while letting
power users opt in.

## Guide-level explanation

### Glob excludes — new behaviour

`[file_tree_view].extra_excludes` accepts glob patterns. Existing
entries continue to work because a literal name like `"tmp"` is
also a valid glob that matches exactly the name `tmp`.

```toml
[file_tree_view]
extra_excludes = [
  "tmp/",         # any directory called tmp
  "*.log",        # any .log file
  "cache-*",      # cache-foo, cache-bar, …
  "*~",           # editor backup files
]
```

A trailing `/` makes the pattern directory-only (matches directories,
not files of the same name). Patterns without `/` match either.

### `.gitignore` honouring — new opt-in

```toml
[file_tree_view]
respect_gitignore = true   # default: false
```

When `true`, the filter parses any `.gitignore` files found in the
fallback respond directory and its ancestors, applying the same
ignore rules Git would. `.git/info/exclude` and the global
`~/.gitconfig` excludes file are not honoured (a deliberate
simplification — the in-tree `.gitignore` is the common case).

When `false` (the default), behaviour is unchanged from v5.9.0.

### Interaction with `service.fallback_respond_dir`

A subtle corner: if the configured `service.fallback_respond_dir`
itself sits under a path that built-in excludes or `.gitignore`
would normally hide (e.g. the user puts mock fixtures under
`target/fixtures/`), the filter must not hide the root itself —
otherwise the GUI shows an empty tree. The root directory is
always kept; only its *contents* are filtered.

## Reference-level explanation

### `FileTreeFilter` additions

```rust
#[derive(Clone, Debug)]
pub struct FileTreeFilter {
    pub show_hidden: bool,
    pub builtin_excludes: bool,
    pub extra_excludes: Vec<String>,    // now glob patterns (was exact-name)
    pub include: Vec<String>,
    pub respect_gitignore: bool,         // NEW
}

impl Default for FileTreeFilter {
    fn default() -> Self {
        Self {
            show_hidden: false,
            builtin_excludes: true,
            extra_excludes: Vec::new(),
            include: Vec::new(),
            respect_gitignore: false,    // off by default
        }
    }
}
```

### Glob matching

Add `globset = "0.4"` as a workspace dependency. `FileTreeFilter`
gains an internal compiled-pattern cache:

```rust
struct CompiledFilter {
    extra_excludes: globset::GlobSet,
    include: globset::GlobSet,
}

impl FileTreeFilter {
    fn compile(&self) -> Result<CompiledFilter, FilterCompileError> {
        let mut excl = globset::GlobSetBuilder::new();
        for pat in &self.extra_excludes {
            excl.add(globset::Glob::new(pat)?);
        }
        // … same for include
        Ok(CompiledFilter {
            extra_excludes: excl.build()?,
            include: /* … */,
        })
    }

    fn keep(&self, compiled: &CompiledFilter, name: &str, is_dir: bool)
        -> bool { /* uses compiled.extra_excludes.is_match(name) */ }
}
```

The compile step runs once per `build_file_tree` / `list_directory`
call. For depth-1 enumeration this is negligible; for repeated
calls a future optimisation might cache compiled filters across
calls, but it's not needed at current scale.

A glob compile failure surfaces as a `Diagnostic` at validation
time (so the GUI shows the user the error before save) and is
treated as "skip this pattern" at runtime (graceful degradation).

### `.gitignore` parsing

Add `ignore = "0.4"` as a workspace dependency. The `ignore` crate
is used by `ripgrep` / `fd` and handles `.gitignore` semantics
correctly (precedence, negations, ancestor traversal).

When `respect_gitignore == true`, the filter constructs an
`ignore::gitignore::Gitignore` rooted at the directory being
listed, walking up to find ancestor `.gitignore` files:

```rust
fn build_gitignore(root: &Path) -> Option<ignore::gitignore::Gitignore> {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
    // Walk up from `root` to filesystem root or to a `.git/` directory,
    // collecting any `.gitignore` files found along the way.
    for ancestor in root.ancestors() {
        let candidate = ancestor.join(".gitignore");
        if candidate.is_file() {
            builder.add(&candidate);
        }
        if ancestor.join(".git").is_dir() {
            break;
        }
    }
    builder.build().ok()
}
```

The compiled gitignore is consulted in `FileTreeFilter::keep`
after the built-in exclude check, before `extra_excludes`. An
entry filtered by gitignore is not surfaced; users who want to
override gitignore for a specific name can add it to `include`.

Per-directory `.gitignore` files (one inside a subdirectory rather
than at the workspace root) are loaded on demand when
`list_directory` descends into that subdirectory.

### `[file_tree_view]` TOML extensions

```toml
[file_tree_view]
show_hidden = false
builtin_excludes = true
extra_excludes = ["*.log", "tmp/"]
include = []
respect_gitignore = false      # NEW
```

`RootSettingKey` gains:

```rust
RootSettingKey::FileTreeRespectGitignore,    // EditValue::Boolean
```

`ReloadHint::for_key` returns `reload()` for the new key (no
listener rebind needed).

### Migration

- Existing `extra_excludes` literal-name entries continue to work
  unchanged. `"tmp"` as a glob matches exactly the name `tmp`,
  same as exact-match.
- The single corner case: an entry that happens to contain a glob
  meta-character (`*`, `?`, `[`, `]`, `\`) under the old behaviour
  would have matched only the literal string with that character;
  under the new behaviour, the character is interpreted as glob
  meta. This is exceedingly unlikely in practice (filenames with
  literal `*` are rare).
- `respect_gitignore` defaults to `false`, so workspaces with no
  config change see no behaviour change.

### Fallback-respond-dir protection

In `build_file_tree(root, &filter)`, the root directory is added
to the resulting `FileTreeView` unconditionally (its name is not
checked against any exclude rule). Only entries *under* the root
are filtered. This protects against the user picking
`target/fixtures` as their `fallback_respond_dir` and finding the
GUI tree empty because `target` is in `BUILTIN_EXCLUDES`.

## Drawbacks

1. **Two new dependencies (`globset`, `ignore`).** Both are
   well-maintained Rust ecosystem standards. Binary size impact
   estimated at ~200 KB after release-mode link-time optimisation
   (TBD; benchmark at implementation).
2. **Subtle behaviour shift on `extra_excludes`.** Edge cases with
   meta-characters in patterns behave differently. Mitigated by
   the rarity of the case; CHANGELOG note is essential.
3. **`.gitignore` parsing has non-obvious semantics.** Precedence
   between `.gitignore` files at different levels, negation rules
   (`!foo`), and `.gitignore` inside subdirectories are handled
   correctly by the `ignore` crate but may surprise users who don't
   know Git's rules. Documentation should link to Git's gitignore
   spec.
4. **Per-directory `.gitignore` reload cost.** If a user has
   `.gitignore` files at many subdirectories, each `list_directory`
   call re-parses them. Acceptable at current scale; a cache is a
   future optimisation if profiling shows it matters.

## Rationale and alternatives

**Alternative A (this RFC): use `globset` + `ignore`.** Standard
Rust libraries with the right semantics. Some dependency cost.

**Alternative B: hand-rolled glob matcher.** A 100-line "supports
`*` and `?` and nothing else" implementation. Cheaper dependency,
narrower correctness. The original RFC 005 considered this in its
"Unresolved questions" §3 with `glob` as the simpler option.
Acceptable if `globset` is overkill, but `globset` is used in
exactly this kind of "filter file names" workflow throughout the
Rust ecosystem, so the choice is mildly conservative either way.

**Alternative C: hand-rolled `.gitignore` parser.** Reinventing a
well-tested wheel. Rejected.

**Alternative D: drop `.gitignore` honouring entirely; keep glob.**
Halves the dependency cost; loses what RFC 005 deliberately
proposed. Acceptable as a smaller v5.11 scope if dependency
sensitivity is high; this RFC recommends both since they're
naturally coupled in the user's mind.

We pick A. B is a viable smaller variant if dependency cost surfaces
as a real concern at implementation time.

## Prior art

- `ripgrep` and `fd` use `globset` + `ignore` for the same job; the
  combination is essentially the Rust ecosystem standard for "filter
  file enumeration like Git would".
- VS Code's file explorer uses `files.exclude` (glob-keyed) plus
  `files.useGitignore`; this RFC's split between `extra_excludes`
  and `respect_gitignore` mirrors that.
- Mountebank, WireMock, etc. don't have a file-tree picker, so no
  direct comparison.

## Unresolved questions

1. **`globset` vs `glob` crate choice.** ✅ **Resolved.** `globset` —
   set-based matching is exactly our use case (filter a single
   filename against many patterns per directory entry), and it's
   the ecosystem standard (ripgrep, fd).
2. **Should the glob support brace expansion (`{*.log,*.tmp}`)?**
   `globset::Glob` does support some bash-like extensions; whether
   to advertise them in user-facing docs is a UX call. Recommend:
   only document `*` / `?` / `[...]` / trailing `/` for stage-2;
   leave braces undocumented (they'll work but aren't promised).
3. **`.gitignore` semantics for the fallback respond dir.** ✅
   **Resolved.** Root is always kept; only contents are filtered.
   Document the override behaviour in advanced topics.
4. **Per-directory `.gitignore` cache.** Worth a small benchmark at
   implementation; if `list_directory` profiles cleanly without a
   cache, defer the optimisation.

## Future possibilities

- A `FileTreeFilter::compile_cache` that memoises compiled globs
  and gitignore parses across calls (deferred optimisation).
- `.git/info/exclude` and global Git excludes honouring.
- A `--show-skipped` diagnostic mode that surfaces filter counts
  (echoing RFC 005's "filter performance telemetry" future).
- A glob-pattern syntax for `include` symmetrical to
  `extra_excludes` (currently `include` is suffix-match per RFC 005;
  could be unified).
