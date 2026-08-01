# RFC 026 — `apimock validate` CLI subcommand

**Status.** Implemented (v5.13.0)
**Tracks.** Developer tooling — a companion to `apimock match-test`
that validates a workspace config without starting the server.
**Touches.** `crates/apimock` (`args.rs`, new `cmd/validate.rs`),
documentation.

## Summary

`apimock validate <config-path>` loads the workspace at
`<config-path>`, runs `Workspace::validate()`, prints all
diagnostics, and exits 0 on success or 1 on validation errors.
Useful in CI pipelines and as a pre-flight check before starting
the mock server.

## Reference-level explanation

### CLI

```
apimock validate --config apimock.toml
```

Flag: `--config` / `-c` (required). Points to `apimock.toml`.

### Output

```
apimock-rule-set.toml: [WARNING] rule #3: respond.file_path not found
apimock.toml: [ERROR] listener.port must be > 0
Validation failed: 1 error, 1 warning.
```

On success: `Validation passed (N rules across M rule sets).`

### Exit codes

- `0` — no errors (warnings allowed).
- `1` — at least one `Severity::Error`.
- `2` — config could not be loaded (parse / file-read error).

### Flags

| Flag | Description |
|------|-------------|
| `--config`, `-c` | Path to `apimock.toml` (required) |
| `--strict` | Treat warnings as errors (exit 1) |
| `--quiet` | Suppress output; use exit code only |
| `--json` | Output diagnostics as JSON array |

### Integration with the existing `match-test` subcommand

Both subcommands live in `crates/apimock/src/cmd/`. The dispatch in
`args.rs` routes `apimock match-test ...` and `apimock validate ...`
to their respective modules. Shared workspace-loading logic is
extracted into `cmd::common::load_workspace(path) -> AppResult<Workspace>`.

## Unresolved questions

None.
