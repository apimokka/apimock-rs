# apimock-rs RFCs

Lifecycle and folder conventions: [RFC 000](./done/000-rfc-lifecycle-policy.md).

## Layout

```
rfcs/
  README.md       ← this file
  proposed/       ← written, under review; not yet approved
  accepted/       ← owner approved; implementer may start, or has
                    finished but the work has not been released
  done/           ← released; historical record
  archive/        ← withdrawn or superseded
```

## Proposed

Written and under review. **Not approved** — nothing here may be
implemented yet.

See [ROADMAP.md](../ROADMAP.md) for themes, milestones, priority,
depends-on, and the rest of the planned portfolio.

*(None open.)*

## Accepted

Approved by the project owner: the design is settled and an implementer
may start. An RFC sits here from approval until the version carrying it
ships — see [RFC 000](./done/000-rfc-lifecycle-policy.md).

| ID  | Title | State |
|-----|-------|-------|
| 039 | [An additive-only gate for the public API](./accepted/039-public-api-additive-only-gate.md) — enable after 6.0.0 | **not started** — deliberately deferred; enable *after* 6.0.0 |
| 067 | [CORS: stop reflecting any origin with credentials](./accepted/067-cors-credential-reflection.md) — audit S-01, **highest-ranked** | accepted 2026-09-01; handed off (tranche) |
| 068 | [Bound what one request can consume](./accepted/068-bound-per-request-resources.md) — audit S-02, S-03 | accepted 2026-09-01; handed off (tranche) |
| 069 | [Reject configuration we do not understand](./accepted/069-reject-unknown-config-keys.md) — audit F-17 | accepted 2026-09-01; handed off (tranche) |
| 070 | [`round_robin` must rotate per match group](./accepted/070-round-robin-per-match-group.md) — audit F-01 | accepted 2026-09-01; handed off (tranche) |
| 071 | [Stop deep-cloning application state per request](./accepted/071-share-application-state.md) — audit P-01, P-02 | accepted 2026-09-01; handed off (tranche) |
| 072 | [Header matching must fail closed](./accepted/072-header-matching-fails-closed.md) — audit S-04 | accepted 2026-09-01; handed off (tranche) |
| 073 | [Observability: correct events, honest limits, no leaks](./accepted/073-observability-correct-and-safe.md) — audit F-08, S-05, S-06 | accepted 2026-09-01; handed off (tranche) |
| 074 | [TLS: bound the handshake, and fail loudly](./accepted/074-tls-availability.md) — audit S-07, S-08 | accepted 2026-09-01; handed off (tranche) |
| 075 | [URL path fidelity](./accepted/075-url-path-fidelity.md) — audit F-03, F-05, F-02 | accepted 2026-09-01; handed off (tranche) |
| 076 | [Serve JSON as it was written](./accepted/076-serve-responses-as-written.md) — audit F-04, P-04 | accepted 2026-09-01; handed off (tranche) |
| 077 | [Work that should not be per-request](./accepted/077-per-request-work.md) — audit P-05–P-09 | accepted 2026-09-01; handed off (tranche) |
| 078 | [Correct four false statements; add troubleshooting](./accepted/078-documentation-corrections.md) — audit D-01–D-07 | accepted 2026-09-01; handed off (tranche) |
| 079 | [Remove code that says something untrue](./accepted/079-dead-and-misleading-code.md) — audit F-10, M-03, M-04, M-09 | accepted 2026-09-01; handed off (tranche) |

> The other 20 that were here shipped in **6.0.0** (2026-08-28) and have
> moved to `done/`. **039 stays**: it is approved and deliberately
> unimplemented until *after* 6.0.0 — enabling an additive-only gate
> before the major that breaks things would have been backwards.

Handoffs live under `handoffs/NNN-slug/` and inherit their status from
the governing RFC — they are companion execution documents, not
separate lifecycle items (see [RFC 000](./done/000-rfc-lifecycle-policy.md)).

> **`handoffs/` is for work we commission from our own implementer**,
> and nothing else. A few are named for a release or a task rather than
> an RFC number (`6-0-0-…`, `post-6-0-0-…`); those still commission
> execution here, which is what makes them handoffs.
>
> **A document addressed to another team is not one.** It commissions
> nothing in this repository and has no RFC status to inherit. Split it
> by what it actually is:
>
> - **Explaining our published surface** → `docs/src/`, where it is
>   published, versioned with the code, and covered by the documentation
>   gates. `docs/src/library/` exists for exactly this.
> - **A specific ask to a specific team** → `.git-exclude/`, as
>   correspondence, alongside review-requests and release plans.
>   Name it `<version>-for-<team>` —
>   `.git-exclude/handoff-external/6.0.0-for-gui-app-team/` is the first.
>   **The version is the point:** such a document describes the API
>   surface of one release. Pinning it in the name means it stays an
>   accurate record of what that team was told, instead of being edited
>   until it no longer matches anything.
>
> Added 2026-08-30, after a package for an external app team was filed
> here where it matched neither RFC 000's definition nor its own purpose:
> it identified a missing library guide, then sat somewhere it could not
> serve as one.

> **RFCs 067–079 all come from the independent audit of 6.0.0** and are
> handed off as **six tranches**, not thirteen packages —
> `rfcs/handoffs/audit-t1-…` through `audit-t6-…`. Each tranche is a
> coherent shippable unit with its own release constraint, which is how
> the work is actually done; grouping by RFC number would have split
> them across releases and lost the ordering that matters.

## Done (Released)

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
| 054 | [The v5 deprecation release](./done/054-deprecation-release.md) | v5.19.0 |
| 066 | [Branching and merge policy](./done/066-branching-and-merge-policy.md) — who may merge; the release line is not the dev team's to move | (policy) |
| 040 | [Trace channel: redaction, and non-JSON body capture](./done/040-trace-capture-and-redaction.md) | v6.0.0 |
| 041 | [Error type shape: boxing, `kind()`, and `#[non_exhaustive]`](./done/041-error-type-shape.md) | v6.0.0 |
| 042 | [External change detection: correct the contract](./done/042-external-change-detection.md) | v6.0.0 |
| 043 | [Module split: `workspace/edit.rs`](./done/043-module-split-edit-rs.md) | v6.0.0 |
| 048 | [v6 concept: the CLI as a first-class interface](./done/048-v6-cli-interface-concept.md) — umbrella | v6.0.0 |
| 050 | [Should non-JSON request bodies be captured at all?](./done/050-non-json-body-capture-decision.md) — decision RFC | v6.0.0 |
| 051 | [Redact credential headers in verbose request logging](./done/051-verbose-log-header-redaction.md) | v6.0.0 |
| 052 | [`#[non_exhaustive]` on the public types that keep growing](./done/052-non-exhaustive-public-types.md) | v6.0.0 |
| 053 | [The v6 CLI contract](./done/053-v6-cli-contract.md) | v6.0.0 |
| 055 | [`apimock get`: what will this request return?](./done/055-get-command.md) | v6.0.0 |
| 056 | [Preserve what people wrote: `toml_edit` for the save path](./done/056-toml-edit-migration.md) | v6.0.0 |
| 057 | [`apimock set`: make the server answer X under condition Y](./done/057-set-command.md) | v6.0.0 |
| 058 | [`respond_dir` grows on every save](./done/058-respond-dir-prefix-persistence.md) — **released defect, R-10** | v6.0.0 |
| 059 | [CLI contract conformance across every command](./done/059-cli-contract-conformance.md) | v6.0.0 |
| 060 | [Property-test the config write path](./done/060-write-path-property-testing.md) | v6.0.0 |
| 061 | [Test on the platforms we ship](./done/061-cross-platform-ci.md) | v6.0.0 |
| 062 | [The v6 threat model, refreshed](./done/062-v6-threat-model.md) | v6.0.0 |
| 063 | [Confine the serve path](./done/063-serve-path-confinement.md) | v6.0.0 |
| 064 | [Finish the CLI front door](./done/064-cli-front-door-completion.md) | v6.0.0 |
| 065 | [The response body-source model](./done/065-response-body-source-model.md) | v6.0.0 |

## Archive

| ID  | Title | Reason |
|-----|-------|--------|
| 018 | [ConditionalFallback audit](./archive/018-conditional-fallback-strategy.md) | Withdrawn — existing dispatch covers the case |

---

## Adding a new RFC

1. Pick the next free number — **058+**. Numbers 039–043 are reserved
   by [ROADMAP.md](../ROADMAP.md) for the planned M3 RFCs; 048 (the v6
   umbrella) is still open. Create `rfcs/proposed/NNN-slug.md`.
2. On shipping, move to `done/`, update Status field, update this index.

See [RFC 000](./done/000-rfc-lifecycle-policy.md) for the full policy.
