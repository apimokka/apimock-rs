# Acceptance / QA Checklist — RFC 055

**Governing RFC.** [RFC 055](../../done/055-get-command.md)
**Contract.** [RFC 053](../../done/053-v6-cli-contract.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

---

## The answer is true — the check that matters

- [ ] **`get` agrees with a running server** on the same config, across
      requests covering **every** dispatch stage
- [ ] **Zero-config: a file-tree path returns the file**, not "no match"
- [ ] Rule-set path returns the rule's body, status and headers

## Every dispatch stage covered

- [ ] `OPTIONS` handled as the server handles it
- [ ] Rule sets
- [ ] `dyn_route` fallback
- [ ] **Middleware: not executed**, and its presence **reported** when
      configured, with the answer marked incomplete
- [ ] No `--run-middleware` flag added
- [ ] Any stage that could not be covered was **escalated**

## One implementation of matching

- [ ] `rule_set_response` / `dyn_route_content` reused — matching **not**
      reimplemented
- [ ] What `apimock-server` had to expose **established from source** and
      kept minimal; anything larger escalated

## `--why`

- [ ] Names the deciding rule set, rule and condition
- [ ] **Near-miss names the failing condition** — the feature, not a
      nicety
- [ ] **Off by default in text, on by default in JSON**

## Contract

- [ ] `--format json` emits a valid RFC 053 envelope
- [ ] Provenance: absolute paths of the configuration that answered
- [ ] No match → **exit 0** with a result saying so, per RFC 053

## Scope held

- [ ] `match-test` untouched — still exits 1 on no-match, deliberately
- [ ] No `set` work
- [ ] No talking to a running server (except in the comparison test)

## Suite and gates

- [ ] Full suite green; count reported against the **455** baseline
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
