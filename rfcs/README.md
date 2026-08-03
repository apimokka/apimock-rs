# apimock-rs RFCs

Lifecycle and folder conventions: [RFC 000](./done/000-rfc-lifecycle-policy.md).

## Layout

```
rfcs/
  README.md       ← this file
  proposed/       ← open for review
  done/           ← implemented; historical record
  archive/        ← withdrawn or superseded
```

## Proposed

See [ROADMAP.md](../ROADMAP.md) for themes, milestones, release cycle,
and the rest of the planned portfolio.

### M1 — Pipeline trust → v5.15.0

| ID  | Title | Priority | Depends on | Handoff |
|-----|-------|----------|------------|---------|
| 030 | [Warning-clean baseline](./proposed/030-warning-clean-baseline.md) | P0 | — | [yes](./handoffs/030-warning-clean-baseline/implementation-handoff.md) |
| 031 | [CI quality gates](./proposed/031-ci-quality-gates.md) | P0 | 030 | [yes](./handoffs/031-ci-quality-gates/implementation-handoff.md) |
| 032 | [Release and packaging repair](./proposed/032-release-and-packaging-repair.md) | P0 | — | [yes](./handoffs/032-release-and-packaging-repair/implementation-handoff.md) |
| 033 | [Supply-chain gates](./proposed/033-supply-chain-gates.md) | P1 | 031 | pending D-04 |

Execution order: 030 → 031 → 033, with 032 in parallel.

### M2 — Documentation and examples → v5.16.0

| ID  | Title | Priority | Depends on | Handoff |
|-----|-------|----------|------------|---------|
| 034 | [Documentation information architecture](./proposed/034-documentation-information-architecture.md) | P0 | — | pending M1 |
| 035 | User guide and configuration reference rewrite | P0 | 034 | not yet written |
| 036 | [Example configurations](./proposed/036-example-configs.md) | P0 | — | pending M1 |
| 037 | README rethink | P1 | 034 | not yet written |
| 038 | Technical reference refresh and document integrity | P1 | 034 | not yet written |

RFC 034 is design-first and gates 035, 037, and 038; RFC 036 is
independent and runs in parallel. RFCs 035, 037, and 038 are written
once 034's page map is agreed — drafting them earlier would mean
inventing a structure twice.

Handoffs live under `handoffs/NNN-slug/` and inherit their status from
the governing RFC — they are companion execution documents, not
separate lifecycle items (see [RFC 000](./done/000-rfc-lifecycle-policy.md)).

## Done (Implemented)

| ID  | Title | Shipped |
|-----|-------|---------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | (policy) |
| 001 | [URL path operator in RulePayload](./done/001-url-path-operator-in-rule-payload.md) | v5.8.0 |
| 002 | [Headers and body conditions in RulePayload](./done/002-headers-and-body-conditions-in-rule-payload.md) | v5.8.0 |
| 003 | [TLS and log in RootSettingKey](./done/003-tls-and-log-in-root-setting-key.md) | v5.8.0 |
| 004 | [Structured WhenView](./done/004-structured-when-view.md) | v5.8.0 |
| 005 | [File tree filtering](./done/005-file-tree-filtering.md) | v5.8.0 |
| 006 | [Live match-trace channel](./done/006-live-match-trace-channel.md) | v5.8.0+v5.9.0 |
| 007 | [Rule-evaluation strategy variants](./done/007-rule-evaluation-strategy-variants.md) | v5.8.0 |
| 008 | [Body match language extension](./done/008-body-match-language-extension.md) | v5.8.0 |
| 009 | [Trace socket transport](./done/009-trace-socket-transport.md) | v5.9.0 |
| 010 | [Body match null/Exists + equal_integer](./done/010-body-match-null-and-equal-integer.md) | v5.9.0 |
| 011 | [RoundRobin strategy](./done/011-round-robin-strategy.md) | v5.9.0 |
| 012 | [Config-driven FileTreeFilter](./done/012-file-tree-filter-config.md) | v5.9.0 |
| 013 | [RulePayload url_path/op validation](./done/013-rule-payload-url-path-validation.md) | v5.9.0 |
| 014 | [Header IndexMap order](./done/014-header-indexmap-order.md) | v5.10.0 |
| 015 | [`apimock match-test` CLI](./done/015-match-test-cli.md) | v5.10.0 |
| 016 | [Per-condition NodeId addressability](./done/016-per-condition-node-id.md) | v5.10.0 |
| 017 | [Payload operator routing parity](./done/017-payload-operator-routing-parity.md) | v5.11.0 |
| 019 | [File tree gitignore + glob excludes](./done/019-file-tree-gitignore-and-glob-excludes.md) | v5.11.0 |
| 020 | [TLS hot-reload (Outcome C)](./done/020-tls-hot-reload-feasibility-audit.md) | v5.11.0 |
| 021 | [Negated value operators](./done/021-negated-value-operators.md) | v5.12.0 |
| 022 | [MapHasKey / MapDoesNotHaveKey body operators](./done/022-map-has-key-body-operator.md) | v5.12.0 |
| 023 | [Body capture in trace events](./done/023-trace-body-capture.md) | v5.12.0 |
| 024 | [Workspace external-change detection](./done/024-workspace-external-change-detection.md) | v5.13.0 |
| 025 | [Per-rule-set strategy override](./done/025-per-rule-set-strategy.md) | v5.13.0 |
| 026 | [`apimock validate` CLI subcommand](./done/026-validate-cli-subcommand.md) | v5.13.0 |
| 027 | [Rule priority field](./done/027-rule-priority-field.md) | v5.14.0 |
| 028 | [StructuralContains body operator](./done/028-structural-contains.md) | v5.14.0 |
| 029 | [Per-condition diff granularity](./done/029-diff-granularity.md) | v5.14.0 |

## Archive

| ID  | Title | Reason |
|-----|-------|--------|
| 018 | [ConditionalFallback audit](./archive/018-conditional-fallback-strategy.md) | Withdrawn — existing dispatch covers the case |

---

## Adding a new RFC

1. Pick the next free number — **042+**. Numbers 034–041 are reserved
   by [ROADMAP.md](../ROADMAP.md) for the planned M2 and M3 RFCs.
   Create `rfcs/proposed/NNN-slug.md`.
2. On shipping, move to `done/`, update Status field, update this index.

See [RFC 000](./done/000-rfc-lifecycle-policy.md) for the full policy.
