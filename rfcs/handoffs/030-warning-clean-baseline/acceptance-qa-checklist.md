# Acceptance / QA Checklist — RFC 030

**Governing RFC.** [RFC 030](../../done/030-warning-clean-baseline.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

Run per slice, then once for the whole. Paste actual output into the
review-request package — not a summary of it.

---

## Per-slice gate

```bash
cargo fmt --check
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo test --workspace
```

- [ ] `cargo fmt --check` exits 0
- [ ] Slice crate's clippy exits 0
- [ ] `cargo test --workspace` reports **371 passed; 0 failed**
- [ ] Per-crate test counts unchanged: 22 / 3 / 140 / 15 / 60 / 116 / 14 / 1
- [ ] Every hunk in the diff traces to a specific diagnostic
- [ ] No test assertion modified
- [ ] No `pub` signature changed
- [ ] Every `#[allow]` added has a `// clippy: <reason>` comment

---

## Whole-RFC gate

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets
cargo test --workspace
```

- [ ] Workspace clippy under `-D warnings` exits 0
- [ ] `cargo build --workspace --all-targets` output is **empty** — no
      rustc warnings. Specifically confirm these three are gone:
      - `unused import: http_method::HttpMethod`
      - `unused import: std::io::Write`
      - `unused variable: config`
- [ ] `cargo test --workspace` = 371 passed, 0 failed
- [ ] `cargo fmt --check` exits 0
- [ ] No new dependency in `Cargo.toml` or `Cargo.lock`
- [ ] No `unsafe` introduced: `grep -rn "unsafe" crates/*/src/` returns
      nothing new
- [ ] No file split; `workspace/edit.rs` and `trace.rs` still single files
- [ ] No `#![allow]` at any crate root; no `[lints]` table added

---

## Behaviour-preservation spot checks

The test suite is the primary guard, but three things it does not
directly assert are worth a manual look, since they were flagged as
concentration points for risky fixes:

- [ ] **Exhaustive matches still exhaustive.** No `match` on `RuleOp`,
      `HeaderOperator`, `BodyOperator`, `Strategy`, or `DiffKind`
      gained a `_ =>` catch-all arm.
- [ ] **`Option<Vec<_>>` semantics intact** (DEC-017) in
      `workspace/edit/payload.rs`: `None` preserves, `Some([])` clears,
      `Some([…])` replaces.
- [ ] **TLS and trace edits itemised.** Any change under
      `apimock-server/src/tls.rs` or `trace.rs` is listed individually
      in the review request with its diagnostic.

---

## Escalations to report

- [ ] Any `clippy::result_large_err` finding on a **public** error type
      (`WorkspaceError`, `ApplyError`, `SaveError`, `ConfigError`,
      `RoutingError`, `ServerError`) — **not fixed**, raised as a
      design request
- [ ] Any finding whose fix would require a public API change
- [ ] Any actual finding count differing from the 130 recorded on
      2026-08-02 — report the real number

---

## Review-request package

- [ ] Created at `.git-exclude/review-request/030-warning-clean-baseline/`
- [ ] Entry-point document orients a reviewer with no prior context
- [ ] Contains all 10 items from § 9.2 of the workflow document
- [ ] **Lists every `#[allow]` added, with justification**
- [ ] Flags which hunks are *not* purely mechanical — this is the
      reviewer's primary focus
