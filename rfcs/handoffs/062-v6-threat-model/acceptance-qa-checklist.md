# Acceptance / QA Checklist — RFC 062, the v6 threat model

**Governing RFC.** [RFC 062](../../done/062-v6-threat-model.md)
**Handoff.** [`implementation-handoff.md`](./implementation-handoff.md)

## A. Before-state confirmed

- [ ] All three probes in the handoff § 2 reproduced **before** any
      change, and their results match what is written there.

## B. Confinement (Option A)

- [ ] `set --rule-set ../outside.toml` → `usage`, **exit 2**, and
      **nothing written** — verified by listing the directory, not by
      exit code alone.
- [ ] The same for an absolute path outside the tree.
- [ ] The opt-out flag permits both, and is named in the error message.
- [ ] The opt-out appears in RFC 059's conformance table.
- [ ] An existing **non**-rule-set TOML is still refused and unchanged.

## C. The regression most likely to bite

- [ ] **`set` in an empty directory still bootstraps.** Creating a file
      that does not exist yet is a write to a non-existent path; a naive
      confinement check breaks it.
- [ ] `--dry-run` in an empty directory still refuses and writes nothing
      (RFC 057's behaviour, unchanged).
- [ ] The W7 acceptance script still passes end to end.

## D. The document

- [ ] Lives under `docs/src/`, linked from the nav, `mdbook build docs`
      clean.
- [ ] Covers actors, surface, deliberate allowances with reasons, and
      non-goals.
- [ ] **T2 is restated in full**, not referenced — the page must stand
      alone.
- [ ] **The GUI asymmetry is recorded**: confinement is CLI-layer, so
      `apimock-config` callers do not inherit it.
- [ ] `--file` gets its sentence explaining why a read path is out of
      scope.
- [ ] The non-goals section says plainly that apimock is a development
      tool, not hardened for hostile input, and should not face an
      untrusted network.

## E. Gates

- [ ] Full suite green; count against `main`'s baseline.
- [ ] `cargo fmt --all --check` and `clippy … -D warnings` clean.
- [ ] Any surface discovered while writing the document is **reported as
      a finding**, not quietly fixed.
