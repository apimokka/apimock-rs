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

See [ROADMAP.md](../ROADMAP.md) for themes, milestones, priority,
depends-on, and the rest of the planned portfolio.

| ID  | Title |
|-----|-------|
| 040 | [Trace channel: redaction, and non-JSON body capture](./proposed/040-trace-capture-and-redaction.md) |
| 048 | [v6 concept: the CLI as a first-class interface](./proposed/048-v6-cli-interface-concept.md) — umbrella |
| 050 | [Should non-JSON request bodies be captured at all?](./proposed/050-non-json-body-capture-decision.md) — decision RFC |
| 051 | [Redact credential headers in verbose request logging](./proposed/051-verbose-log-header-redaction.md) |
| 052 | [`#[non_exhaustive]` on the public types that keep growing](./proposed/052-non-exhaustive-public-types.md) |
| 053 | [The v6 CLI contract](./proposed/053-v6-cli-contract.md) |
| 054 | [The v5 deprecation release](./proposed/054-deprecation-release.md) |

[`gui-integration-questions.md`](./gui-integration-questions.md) — the
open questions for the GUI team, which several RFCs above are blocked
on. Not an RFC; a coordination record.

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
| 030 | [Warning-clean baseline](./done/030-warning-clean-baseline.md) | v5.15.0 |
| 031 | [CI quality gates](./done/031-ci-quality-gates.md) | v5.15.0 |
| 032 | [Release and packaging repair](./done/032-release-and-packaging-repair.md) | v5.15.0 |
| 033 | [Supply-chain gates](./done/033-supply-chain-gates.md) | v5.15.0 |
| 034 | [Documentation information architecture](./done/034-documentation-information-architecture.md) | v5.16.0 |
| 035 | [User guide and reference rewrite](./done/035-user-guide-and-reference-rewrite.md) | v5.16.0 |
| 036 | [Example configurations for new users](./done/036-example-configs.md) | v5.16.0 |
| 037 | [README rethink](./done/037-readme-rethink.md) | v5.16.0 |
| 038 | [Technical reference refresh and document integrity](./done/038-technical-reference-and-document-integrity.md) | v5.16.0 |
| 044 | [Release process: documentation and automation](./done/044-release-process-documentation-and-automation.md) | v5.16.0 |
| 046 | [Test harness: port race and readiness](./done/046-test-harness-port-race-and-readiness.md) | v5.17.0 |
| 047 | [Verify what was actually published](./done/047-post-publish-artifact-verification.md) | v5.17.0 |
| 045 | [Configuration accepted but ignored](./done/045-configuration-accepted-but-ignored.md) | v5.18.0 |
| 049 | [The CLI front door](./done/049-cli-front-door.md) | v5.18.0 |

## Archive

| ID  | Title | Reason |
|-----|-------|--------|
| 018 | [ConditionalFallback audit](./archive/018-conditional-fallback-strategy.md) | Withdrawn — existing dispatch covers the case |

---

## Adding a new RFC

1. Pick the next free number — **055+**. Numbers 039–043 are reserved
   by [ROADMAP.md](../ROADMAP.md) for the planned M3 RFCs; 048 (the v6
   umbrella) is still open. Create `rfcs/proposed/NNN-slug.md`.
2. On shipping, move to `done/`, update Status field, update this index.

See [RFC 000](./done/000-rfc-lifecycle-policy.md) for the full policy.
