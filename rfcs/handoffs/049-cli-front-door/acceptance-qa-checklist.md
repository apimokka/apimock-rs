# Acceptance / QA Checklist — RFC 049

**Governing RFC.** [RFC 049](../../done/049-cli-front-door.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Decided questions honoured

- [ ] Bare `apimock` with no arguments **still starts a server**
- [ ] Parser choice (hand-rolled vs crate) **reported with both pieces of
      evidence**: every valid invocation byte-identical, and the
      dependency cost under RFC 033's `cargo-deny` policy
- [ ] `--dir`'s resolution behaviour **established from source**, not
      inferred from `--config` — finding stated either way

## Rejecting the unrecognised

- [ ] Unknown option → **exit 2**
- [ ] Message on **stderr**, not stdout
- [ ] **No server started** — shown explicitly, not implied
- [ ] Near-match suggestion for a plausible typo

## Version and help

- [ ] `--version` → exit 0, stdout, no server
- [ ] `--help` → exit 0, stdout, no server
- [ ] Both work in a **normal** workspace
- [ ] Both work with **no config file present**
- [ ] Both work with a **deliberately invalid config**

## Path resolution

- [ ] `-c apimock.toml` resolves identically to `-c ./apimock.toml`
- [ ] `--dir` handled per the § 2 finding

## Conventions v6 inherits

- [ ] Exit codes: 0 success · 2 usage · 1 everything else
- [ ] stdout carries only `--version` / `--help`; every diagnostic on
      stderr
- [ ] Any convention that looked arbitrary was **raised**, not chosen
      quietly

## No regression — the one that matters most

- [ ] Every currently-valid invocation **enumerated from
      `args/constant.rs`** and each shown to behave as before
- [ ] `match-test` and `validate` subcommands unaffected
- [ ] `--init`, `--init --yes`, `--init --middleware` unaffected,
      including the non-TTY fallback
- [ ] Full suite green; count reported against the **415** baseline

## Documentation, in the same change

- [ ] `docs/src/reference/cli-reference.md:14`'s `-c` quirk note
      **updated** — it becomes false with goal 4
- [ ] `--version`, `--help` and exit codes documented
- [ ] Example sets and `README.md` checked; **files checked are named**

## Scope held

- [ ] No subcommand restructuring
- [ ] No `get` / `set`
- [ ] No change to config loading or the server
- [ ] Anything reaching beyond `args.rs` / `args/constant.rs` / docs was
      escalated

## Gates

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
