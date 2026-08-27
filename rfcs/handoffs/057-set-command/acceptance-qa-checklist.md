# Acceptance / QA Checklist — RFC 057, `apimock set`

**Governing RFC.** [RFC 057](../../done/057-set-command.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)
**Contract.** Restated in the handoff's § 3 — you do not need RFC 053
in hand to work through section F.

Every box needs evidence in the review-request package — captured
output, not an assertion that it passes.

## A. The acceptance test (RFC 048's W7)

- [ ] The script from RFC 057 § "The acceptance test" runs end to end in
      a clean directory, non-interactively.
- [ ] Every step's exit code asserted: `0` success, `2` usage error,
      `1` everything else (RFC 049).
- [ ] It runs in CI on every commit, not just locally.
- [ ] **Report whether it was awkward to write.** RFC 057 makes this the
      design's falsification test. "It was fine" is a valid answer; so is
      "step 2 needed three flags I had to look up", and the second is
      more useful.

## B. Addressing — the reason this RFC exists

- [ ] **No UUID appears in any `set` output**, on any path: success,
      `--dry-run`, and every error kind. Automated grep for a UUID
      pattern over the JSON, asserting zero matches.
- [ ] An address printed by `get --why` is accepted by `set` **verbatim**,
      with no editing or translation.
- [ ] `get`'s `matched` JSON block now carries `rule_set_file`.
- [ ] `set --rule` is documented as 0-based, and `--help` says so.
- [ ] Addressing a rule set by a path not in `service.rule_sets` fails
      with a clear error, not a panic or a silent no-op.
- [ ] An out-of-range rule index fails the same way.

## C. Preview

- [ ] `--dry-run` leaves every file byte-identical. Checksum before and
      after.
- [ ] The changes `--dry-run` reports match what the same command then
      actually produces.
- [ ] `--dry-run` output identifies targets **by address**, never by
      `NodeId` (§ 1.1 of the handoff).

## D. The write path — RFC 056's guarantees, re-proved at this surface

- [ ] A rule set with comments and non-canonical formatting survives a
      `set`. Show the file before and after.
- [ ] Conflict: modify a file between load and save, run `set`, receive
      kind `conflict` — **and no file modified**.
- [ ] An unreadable file yields kind `io`, distinct from `conflict`.
- [ ] A failed `set` changes nothing at all, not "mostly nothing".

## E. Scope boundaries

- [ ] `service.middlewares` is untouched by every command, **including
      when the config already has entries**. Diff the section.
- [ ] `DeleteRule`, `MoveRule`, `RemoveRuleSet` are not reachable from
      the CLI in this cut.
- [ ] No command in this cut renumbers an existing rule or rule set —
      the property the addressing contract depends on.

## F. Contract conformance (handoff § 3)

- [ ] `--format json` emits the envelope: an object, with `schema`,
      `apimock`, and **exactly one** of `result` / `error`. Asserted on
      parsed JSON, not by string match.
- [ ] `apimock` is the running binary's version.
- [ ] `--format text` is readable by a person and is the default for a
      TTY.
- [ ] Errors carry a `kind` from the closed set in the handoff's § 3,
      with `conflict` and `io` distinguished.
- [ ] Diagnostics go to stderr; stdout carries only the result.

## G. Regression and gates

- [ ] Full suite green; count reported against `main`'s baseline.
- [ ] `cargo fmt --all --check` clean.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- [ ] `get`, `validate` and `match-test` behave exactly as before —
      the `rule_set_file` addition to `get` is the only intended change
      to an existing command.
