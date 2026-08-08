# The quality gates

Six checks run on every push to `main` and every pull request; all six
are required to merge. `.github/CONTRIBUTING.md` carries the
copy-pasteable command list — this page explains what each one is for
and when it runs; it doesn't restate the commands.

| Gate | Catches |
|---|---|
| `fmt` | Formatting drift |
| `clippy` | Lint findings, workspace-wide, across every target and feature combination |
| `test` | Behavioural regressions — the full [`cargo test --workspace`](./build-and-test-locally.md) suite, 409 tests |
| `msrv` | Code that compiles on your toolchain but not on the pinned minimum one |
| `audit` | Known-vulnerable dependencies, via the RustSec advisory database |
| `lockfile` | A `Cargo.toml` edit whose `Cargo.lock` update was forgotten |

`audit` also runs on a weekly schedule, independent of any push or pull
request — a scheduled run turning red with no new commit is correct
behaviour, not a broken gate. It means an advisory was published against
a dependency this project already uses; that's new information about
existing code, not about your change.

Reproduce all six locally before opening a pull request — see
`.github/CONTRIBUTING.md` for the exact commands.

## Version bumps

`./version.sh --update <version>` updates the workspace manifest and
every npm package (including the `optionalDependencies` platform-binary
pins) together, and verifies the result. Individual version fields
aren't hand-edited.
