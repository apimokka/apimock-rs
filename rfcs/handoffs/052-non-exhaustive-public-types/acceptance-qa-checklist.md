# Acceptance / QA Checklist — RFC 052

**Governing RFC.** [RFC 052](../../done/052-non-exhaustive-public-types.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## Established from source, not inherited

- [ ] Construction sites counted **by crate**, independently of the
      handoff's § 2 table
- [ ] Any disagreement with that table **reported**

## The attribute

- [ ] `#[non_exhaustive]` on all five: `TraceConfig`, `RequestSummary`,
      `ParsedRequest`, `LogConfig`, `VerboseConfig`
- [ ] No field, default or behaviour changed

## Cross-crate construction still works

- [ ] **Every** site from § 2's table accounted for and named
- [ ] `ParsedRequest` — `parsed_request_from`, `match_test.rs`,
      `benches/routing.rs`, and the test sites
- [ ] `VerboseConfig` — the `const` in `apimock-server`; the chosen fix
      **stated with its reasoning**
- [ ] No builder added where a plain constructor serves

## Prove it does what it is for

- [ ] A struct literal for one of the five **fails to compile from
      another crate** — demonstrated, e.g. a `compile_fail` doctest
- [ ] Each new constructor is exercised by a test

## Scope held

- [ ] Error enums untouched — that is RFC 041's question
- [ ] No constructor grew logic beyond building a value; anything
      larger was **escalated**

## G2 handled without guessing

- [ ] Built to our own workspace's needs, not to a guess about the GUI
- [ ] Which types got constructors, and which did not, **reported** so
      the pending answer can be checked against it

## Migration guide

- [ ] `docs/src/guides/migrating-to-6-0.md` still accurate for the final
      shape
- [ ] Names the constructors a downstream caller should use, not just
      that the break exists

## Suite and gates

- [ ] Full suite green; count reported against the **450** baseline
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
