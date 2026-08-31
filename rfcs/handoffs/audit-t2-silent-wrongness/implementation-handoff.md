# Handoff — Tranche 2: silent wrongness

**Governing RFCs.** [069](../../accepted/069-reject-unknown-config-keys.md)
(unknown config keys), [070](../../accepted/070-round-robin-per-match-group.md)
(round-robin), [072](../../accepted/072-header-matching-fails-closed.md)
(header matching). All accepted 2026-09-01.
**Milestone.** Next **minor** — see § 1.
**Baseline.** `main` @ `5d9e5bc`, after tranche 1.

---

## 0. Why these three together

Each produces **the wrong answer with no error**. That is the class this
project has spent v6 removing from the CLI front door; these three sit
one layer below it, in the config loader and the matcher.

They also share a release constraint, which is the more practical reason
to group them.

## 1. This tranche is behaviour-breaking. That decides the release.

All three change what an existing setup does:

| RFC | What breaks |
|---|---|
| **069** | A config that loads today stops loading |
| **070** | A `round_robin` rule set returns a different sequence |
| **072** | A header condition that passes today starts failing |

Every one of those is a **fix**. They are still breaks, and someone's
working setup changes under them.

> **Therefore: a minor release with a migration note, not a patch.**
> The 6.x additive-only promise is about the *API*, and none of these
> touches it — but user-visible behaviour changing under a patch upgrade
> is exactly what erodes trust in a version scheme.
>
> Write the migration entries **as you go**, not at the end. Each RFC
> names what a user will see.

## 2. The traps

**069 — the examples are the test.** RFC 069 § Testing says every config
under `examples/` and in the test corpus must still load. **If one does
not, that is a finding, not a test to update** — it means we have been
shipping an example containing a key that does nothing. Report it before
changing it.

**069 — `deny_unknown_fields` narrows forward compatibility.** An older
binary reading a newer config now hard-errors instead of ignoring the
new key. The RFC accepts that trade and scopes the attribute to the
rule-facing structs only for that reason. **Do not widen it to root
config**, where RFCs 067 and 068 are each adding a setting.

**070 — answer the memory question before implementing.** Keying
counters by match group means a map that grows with distinct groups.
The RFC argues that is bounded by rule-set structure rather than
traffic. **Establish it.** If a construction exists where distinct
groups grow with request volume, the design needs a bound and you should
say so rather than shipping a leak on a long-running server.

**072 — the flip is one line; the agreement test is the deliverable.**
Changing `return true` to `return false` at `headers.rs:92` is trivial.
What prevents recurrence is a test running the same input through both
the server path and `rule_check.rs`'s, asserting the same verdict — the
same shape as `respond_validator_agreement.rs`, which exists because two
validators diverged in precisely this way.

**072 — decide `absent` deliberately.** A header that is present but
unreadable is neither "matching" nor "absent". The RFC flags this as
unresolved. Pick an answer, document it, test it; do not let it fall out
of the implementation.

## 3. Verified for you

So you do not re-derive it:

- `headers.rs:81-92` returns **`true`** on a `to_str()` error — the
  fail-open.
- `rule_check.rs:132` uses `hv.to_str().unwrap_or("")` — which is why
  the two disagree. Both confirmed 2026-09-01.
- Round-robin reproduction: a rule set with 2 rules matching `/a` and 3
  matching `/b`, `strategy = "round_robin"`. Requesting `/a` alone gives
  `a1 a2 a1 a2` (correct). **Alternating `/a` and `/b` gives `a1` on
  every `/a` request.**
- Unknown-key reproduction: `headerz` instead of `headers` on a rule
  with a `url_path`. `validate` prints "Validation passed"; the rule then
  serves to a request carrying **no header at all**.

## 4. Where to look — a floor, not a list

- `crates/apimock-routing/src/rule_set.rs` — the round-robin counter
  (`:77`, used around `:319-338`)
- `crates/apimock-routing/.../when/request/headers.rs`
- `crates/apimock/src/cmd/rule_check.rs`
- The rule-facing `Deserialize` types for 069

**Then grep.** For 069 especially: find *every* struct on the rule path
that derives `Deserialize`, not just the obvious four. A missed one is a
key that still silently vanishes, and the whole point is that the user
cannot tell.

## 5. Acceptance

**069**
- [ ] The § 3 reproduction now fails to load, naming the key
- [ ] A near-match suggestion fires for `headerz` → `headers`
- [ ] **The end-to-end scenario**: the rule no longer serves to an
      unconditioned request — assert the response, not just the loader
- [ ] Every `examples/` config and test-corpus config still loads, or a
      failure is **reported**
- [ ] Migration-guide entry written

**070**
- [ ] The § 3 reproduction rotates on both paths
- [ ] Single group unchanged: `a1 a2 a1 a2`
- [ ] Three-plus groups interleaved
- [ ] The memory question answered, with evidence
- [ ] `vary-the-response-for-one-path.md`'s example, run as written,
      produces what the page says

**072**
- [ ] Non-UTF-8 header value does **not** satisfy a condition
- [ ] `match-test` agrees, pinned by an **agreement test** over valid,
      invalid, empty, absent, and each operator kind
- [ ] `absent` semantics decided, documented, tested
- [ ] `get --why` explains the non-match

**All three**
- [ ] Gates green; **API baseline diff empty or declared**
- [ ] CI green on all 12 jobs before merge

## 6. Report back

`.git-exclude/review-request/audit-t2-silent-wrongness/`, including the
§ 2 memory answer for 070, the `absent` decision for 072, and **any
`examples/` config that stopped loading** under 069.
