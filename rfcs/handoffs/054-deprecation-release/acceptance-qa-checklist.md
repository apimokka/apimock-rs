# Acceptance / QA Checklist — RFC 054

**Governing RFC.** [RFC 054](../../done/054-deprecation-release.md)
**Contract.** [RFC 053](../../proposed/053-v6-cli-contract.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Branch discipline

- [ ] Branched from the **`5.18.0` tag**, not `main`
- [ ] No commit from `main` present — RFCs 040/050/051 absent
- [ ] Baseline is **425**, not 437
- [ ] Anything needed from `main` was **escalated**, not cherry-picked

## `--json` is untouched behaviour

- [ ] Emits a **byte-identical** array to 5.18.0's
- [ ] stdout and stderr captured **separately** to prove it
- [ ] Warning on **stderr**, never stdout
- [ ] Warning printed **once**, not per diagnostic
- [ ] Warning names **6.0.0** and the replacement flag
- [ ] Exit code **unchanged** — clean, with errors, `--strict`, `--quiet`

## `--format`

- [ ] `--format json` emits a valid RFC 053 envelope
- [ ] Object, never a bare array; collection inside `result`
- [ ] `schema`, `apimock` present; `apimock` reads **5.19.0**
- [ ] Exactly one of `result` / `error`
- [ ] Asserted on **parsed JSON**, not a string match
- [ ] `--format text` matches today's default output
- [ ] `--json --format json` → **usage error, exit 2**, not a precedence rule

## The envelope is reusable

- [ ] Implemented as a shared helper, **not inline in `validate`**
- [ ] Next command could use it without copying
- [ ] Any awkwardness in producing or parsing it **reported** — this
      release exists partly to find that on `validate` rather than `get`

## Scope held

- [ ] `match-test` untouched
- [ ] No `get` / `set` work
- [ ] `validate`'s diagnostics, severities and exit codes unchanged
- [ ] Nothing from RFCs 040/050/051

## Migration guide

- [ ] Ships in this release
- [ ] Covers the `#[non_exhaustive]` change (RFC 052) — struct literals
      and exhaustive destructuring stop compiling
- [ ] Covers field additions (RFCs 040, 050) and the error enums (041)
- [ ] Covers `--json`'s removal and its replacement
- [ ] Written for someone migrating, not as a changelog
- [ ] Gaps stated honestly rather than guessed

## Suite and gates

- [ ] Full suite green; count reported against **425**
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
