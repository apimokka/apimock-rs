# apimock-rs RFCs

This directory holds the project's Request-For-Comments documents
governing design decisions, feature additions, and policy. The
lifecycle and folder conventions are defined in
[RFC 000](./done/000-rfc-lifecycle-policy.md) — read it first if
you're contributing a new RFC or moving an existing one.

## Layout (4-folder variant per RFC 000)

```
rfcs/
  README.md           ← this file
  proposed/           ← open for review; design may still change
  done/               ← implemented; preserved as historical record
  archive/            ← withdrawn or superseded; kept for reference
```

`draft/` is unused — authors keep drafts in personal branches until
ready for review.

## Proposed

*(none — all v5.11.0 RFCs implemented)*

## Done (Implemented)

| ID  | Title | Shipped in |
|-----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | (policy in effect) |
| 001 | [URL path operator selection in RulePayload](./done/001-url-path-operator-in-rule-payload.md) | v5.8.0 |
| 002 | [Structured headers and body conditions in RulePayload](./done/002-headers-and-body-conditions-in-rule-payload.md) | v5.8.0 |
| 003 | [TLS and log settings in RootSettingKey](./done/003-tls-and-log-in-root-setting-key.md) | v5.8.0 |
| 004 | [Structured WhenView for headers and body conditions](./done/004-structured-when-view.md) | v5.8.0 |
| 005 | [File tree filtering for `FileTreeView`](./done/005-file-tree-filtering.md) | v5.8.0 |
| 006 | [Live match-trace channel from server to GUI](./done/006-live-match-trace-channel.md) | v5.8.0 (skeleton) + v5.9.0 (RFC 009) |
| 007 | [Rule-evaluation strategy variants](./done/007-rule-evaluation-strategy-variants.md) | v5.8.0 — see addendum re ConditionalFallback |
| 008 | [Body match language extension](./done/008-body-match-language-extension.md) | v5.8.0 |
| 009 | [Trace channel socket transport and integration tests](./done/009-trace-socket-transport.md) | v5.9.0 |
| 010 | [Body match semantics: null/Exists and `equal_integer`](./done/010-body-match-null-and-equal-integer.md) | v5.9.0 |
| 011 | [RoundRobin rule-evaluation strategy](./done/011-round-robin-strategy.md) | v5.9.0 |
| 012 | [Config-driven `FileTreeFilter`](./done/012-file-tree-filter-config.md) | v5.9.0 |
| 013 | [RulePayload url_path / url_path_op validation](./done/013-rule-payload-url-path-validation.md) | v5.9.0 |
| 014 | [Header order preservation via IndexMap](./done/014-header-indexmap-order.md) | v5.10.0 |
| 015 | [`apimock match-test` CLI subcommand](./done/015-match-test-cli.md) | v5.10.0 |
| 016 | [Per-condition NodeId addressability](./done/016-per-condition-node-id.md) | v5.10.0 |
| 017 | [Payload operator routing parity](./done/017-payload-operator-routing-parity.md) | v5.11.0 |
| 019 | [File tree filter: `.gitignore` honouring and glob excludes](./done/019-file-tree-gitignore-and-glob-excludes.md) | v5.11.0 |
| 020 | [TLS hot-reload feasibility audit](./done/020-tls-hot-reload-feasibility-audit.md) | v5.11.0 (Outcome C) |

## Archive

| ID  | Title | Reason |
|-----|-------|--------|
| 018 | [ConditionalFallback strategy: audit and recommendation](./archive/018-conditional-fallback-strategy.md) | Withdrawn — existing multi-rule-set dispatch already provides the intended behaviour |

---

## Adding a new RFC

1. Pick the next free number (021 or higher).
2. Create `rfcs/proposed/NNN-slug.md`.
3. Open it for review.
4. On acceptance and shipping, move it to `done/`, update the
   Status field with the release tag, and update this index.

See [RFC 000](./done/000-rfc-lifecycle-policy.md) for the full policy.
