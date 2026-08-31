# 2. Editing a configuration — the `Workspace` model

This is your core loop, and the part of the library most deliberately
shaped for you.

## The model

A `Workspace` is a **loaded configuration you edit by domain commands,
not by TOML text.** You never write TOML; you issue an `EditCommand`
and the workspace applies it, preserving the file's comments and
formatting (RFC 056 — `toml_edit`, not re-serialisation).

```rust
use apimock_config::{Workspace, view::EditCommand};

let mut ws = Workspace::load(PathBuf::from("./apimock.toml"))?;
let snap   = ws.snapshot();          // render this
let report = ws.validate();          // show these as diagnostics
let result = ws.apply(command)?;     // mutate
let diff   = ws.preview_changes();   // "you are about to write this"
let saved  = ws.save()?;             // commit to disk
```

## The full surface

Quoted from `crates/apimock-config/public-api.txt`:

| Method | Returns | Use |
|---|---|---|
| `load(PathBuf)` | `Result<Self, WorkspaceError>` | Open a config and everything it references |
| `snapshot()` | `WorkspaceSnapshot` | The whole tree, for rendering |
| `validate()` | `ValidationReport` | Structured diagnostics — see below |
| `apply(EditCommand)` | `Result<ApplyResult, ApplyError>` | One domain-level edit |
| `preview_changes()` | `Vec<DiffItem>` | What `save()` would write |
| `save()` | `Result<SaveResult, SaveError>` | Write to disk |
| `has_unsaved_changes()` | `bool` | Enable/disable your save button |
| `has_external_changes()` | `bool` | ⚠️ see § External changes |
| `sync_from_disk()` | `Result<(), WorkspaceError>` | ⚠️ see § External changes |
| `config()` | `&Config` | The resolved config, read-only |
| `root_path()` | `&Path` | Where it was loaded from |
| `describe(NodeId)` | `Option<String>` | Human label for a node |
| `list_directory(&Path)` | `Vec<FileNodeView>` | For a file-picker over the respond dir |
| `rule_set_id_at(usize)` | `Option<NodeId>` | Positional → id |
| `rule_id_at(usize, usize)` | `Option<NodeId>` | Positional → id |
| `respond_id_at(usize, usize)` | `Option<NodeId>` | Positional → id |

## `EditCommand` — the complete set (15)

```
AddRuleSet          RemoveRuleSet         UpdateRuleSetStrategy
AddRule             UpdateRule            DeleteRule            MoveRule
UpdateRespond
AddHeaderCondition  UpdateHeaderCondition RemoveHeaderCondition
AddBodyCondition    UpdateBodyCondition   RemoveBodyCondition
UpdateRootSetting
```

**This is a closed set.** If a GUI gesture does not map onto one of
these, there is no supported way to express it — tell us rather than
editing TOML behind the workspace's back. Adding a variant is additive
and cheap; two writers to the same file is not.

## `NodeId` — and the thing that will bite you

Every node has a `NodeId`. **They are minted fresh on each
`Workspace::load()`** — the CLI's own `set.rs` says so directly:

> *"a fresh UUID minted per `Workspace::load()`. That is fine for a
> GUI [session] …"*

So a `NodeId` is **valid for the lifetime of one loaded `Workspace`
and no longer.** Do not persist one, do not put one in a URL, do not
send one to a process that might reload.

The `*_id_at(...)` methods exist precisely to convert a **positional**
address — "rule set 0, rule 2", which *is* stable across loads — into
a current `NodeId`. Persist positional addresses; resolve to ids after
each load.

## `validate()` — why it returns structures, not strings

`ValidationReport` carries `Diagnostic` values with `Severity`,
message, and the `NodeId` they attach to. That shape exists **for you**:
`apimock-config`'s validation module says so, contrasting itself with
the routing crate's `log::error!`-and-return-`bool` approach —

> *"a GUI needs structured `(severity, message, target_id)` triples it
> can render inline."*

So you can put a red underline on the exact node. That is the intent;
use it.

> ⚠️ **Known duplication, and it has bitten once.**
> `apimock-config`'s validation is a **second implementation** of the
> rules in `apimock-routing`'s `Respond::validate`. During 6.0.0's
> development the config-side copy did not learn about a new field, and
> `validate` reported a **false error on every affected rule**. There is
> now a test asserting the two agree
> (`respond_validator_agreement.rs`), added after that incident.
> If you ever see `validate()` disagree with whether a config actually
> loads, that is a bug in us — report it, do not work around it.

## Saving, and conflict detection

`save()` is **not** a blind write. Before writing, it compares each
file's current on-disk content against the text captured at load. A
mismatch returns `SaveError::Conflict` rather than overwriting
(RFC 056).

**For a long-lived GUI session this is the important one.** A user
edits in your GUI, edits the same file in their editor, then clicks
save — they get a conflict, not silent data loss. Surface it as a
conflict, offer reload-or-overwrite, and do not retry blindly.

## External changes — the ⚠️ part

`has_external_changes()` and `sync_from_disk()` exist for the
"file changed underneath a long-lived session" case, which is a GUI
problem and essentially only a GUI problem.

**Nothing in this repository calls them.** They are unit-tested inside
`apimock-config` and have never been driven by a real application. The
CLI does not need them — it loads, edits and exits within one
invocation.

You are the first consumer. Specifically unclear, and worth
establishing early:

- What granularity `has_external_changes()` reports at — whole
  workspace, or per file.
- Whether `sync_from_disk()` preserves unsaved in-memory edits, or
  discards them. **Establish this before building a UI around it**; the
  answer changes what you can offer the user.
- Whether it is cheap enough to poll on a timer, or wants a filesystem
  watcher in front of it.

Please tell us what you find. This is exactly the surface where a first
real consumer produces better design than more speculation would.
