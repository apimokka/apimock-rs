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

*(none — all medium-priority RFCs implemented in v5.10.0)*

## Done (Implemented)

RFCs whose work has shipped. Preserved as historical record of
design rationale and alternatives considered.

| ID  | Title | Shipped in |
|-----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | (policy in effect for this directory) |
| 001 | [URL path operator selection in RulePayload](./done/001-url-path-operator-in-rule-payload.md) | v5.8.0 |
| 002 | [Structured headers and body conditions in RulePayload](./done/002-headers-and-body-conditions-in-rule-payload.md) | v5.8.0 |
| 003 | [TLS and log settings in RootSettingKey](./done/003-tls-and-log-in-root-setting-key.md) | v5.8.0 |
| 004 | [Structured WhenView for headers and body conditions](./done/004-structured-when-view.md) | v5.8.0 |
| 005 | [File tree filtering for `FileTreeView`](./done/005-file-tree-filtering.md) | v5.8.0 |
| 006 | [Live match-trace channel from server to GUI](./done/006-live-match-trace-channel.md) | v5.8.0 |
| 007 | [Rule-evaluation strategy variants](./done/007-rule-evaluation-strategy-variants.md) | v5.8.0 |
| 008 | [Body match language extension](./done/008-body-match-language-extension.md) | v5.8.0 |

## Archive

Withdrawn or superseded RFCs.

| ID  | Title | Reason |
|-----|-------|--------|
| _(none yet)_ | | |

---

## Numbering and naming

- Sequential `NNN-slug.md`, three digits, starting at `001`.
  RFC 000 is reserved for the lifecycle policy itself.
- Numbers are stable forever (never reused, never renumbered).
- The slug is human-readable, lowercase, hyphen-separated.

## Status field

Each RFC's first metadata field carries its current state, mirroring
the folder it lives in. The folder is authoritative.

```markdown
**Status.** Implemented (v5.8.0)
```

## Adding a new RFC

1. Pick the next free number (009 or higher).
2. Create `rfcs/proposed/NNN-slug.md`.
3. Open it for review.
4. On acceptance and shipping, move it to `done/`, update the
   Status field with the release tag, and update this index.

See [RFC 000](./done/000-rfc-lifecycle-policy.md) for the full
policy, anti-patterns to avoid, and adoption guidance.
