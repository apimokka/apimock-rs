# Implementation Handoff — RFC 045, Configuration accepted but ignored

**Governing RFC.** [RFC 045](../../proposed/045-configuration-accepted-but-ignored.md)
**Milestone.** M3 — P1, targeting **v5.18.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

Two shipped configuration fields parse, pass `apimock validate`, print at
startup, and do nothing. Fix both, and answer the question underneath
them: should validation have caught this?

This is the first work in a while that changes **runtime behaviour a user
can observe**, rather than pipeline or documentation. Treat existing
behaviour as a constraint, not an obstacle.

## 2. Two of the RFC's three unresolved questions are now decided

**Unresolved 1 — implement or remove `[default].delay_response_milliseconds`?
→ IMPLEMENT.** Option A in the RFC. The field's name and position promise
a rule-set-wide default and that is a reasonable feature; removing it
would break the TOML schema for anyone who set it. Per-rule
`respond.delay_response_milliseconds` overrides it.

This is a **user-visible behaviour change**: a config that today delays
nothing will start delaying. Anyone depending on the current behaviour is
depending on a bug, but it must appear in the CHANGELOG — flag it in your
review request so it isn't missed when the release notes are written.

**Unresolved 3 — do the RFC 036 examples get simplified?
→ NOT IN THIS CHANGE.** `match-headers-and-body/` and
`status-codes-and-errors/` use `file_path` where `text` + headers would
now work. Simplifying them touches a shipped example set that
`apimock --init` scaffolds from and every release archive ships, for no
correctness gain. Out of scope.

**But do check for stale prose.** If any example README or config comment
explains *why* it avoids the pattern these fixes repair, that explanation
becomes false the moment this lands. I looked and found none, so I expect
this to be a no-op — confirm it rather than assume it, and say which
files you checked.

**Unresolved 2 — Goal 4, how far validation should go — stays open and is
yours to investigate.** See § 5.

## 3. Defect 1 — `respond.headers`

Established in the RFC against source, line-cited, so it is checkable —
**check it.** Two distinct faults:

| `respond` shape | today | required |
|---|---|---|
| `file_path` | honoured | unchanged |
| `text` alone | honoured, but `content-type` is overwritten | explicit `content-type` wins |
| `text` + `status` | all dropped | honoured |
| `status` alone | all dropped | honoured |

The precedence rule to apply, stated once and everywhere: **an explicitly
configured header wins over an inferred default.**

**The trap the RFC names, and I want it verified rather than reasoned
about:** `file_path` responses infer `content-type` from the file
extension and currently honour custom headers correctly. Do not let
"explicit wins over inferred" regress that path. The existing suite is
the guard — run it, don't argue from the code.

## 4. Defect 2 — the dead rule-set default

`RuleSet.default` is read only at `rule_set.rs:312`, for display.
`delay_response_milliseconds` is read only at `respond_response.rs:45`,
from `respond`. Make the rule-set default apply when a rule does not set
its own.

Measured symptom for your before/after: 2000 ms configured, ~4 ms
observed.

## 5. Goal 4 — the part that matters most, and the part I cannot specify

`apimock validate` certifies configuration that does nothing. RFC 026
shipped `validate` precisely so users could trust a config before running
it, so this is a defect in the thing users were told to trust.

The RFC lists three options. **Option 3 — a structural guarantee that a
parsed field cannot go unread, e.g. an exhaustiveness test asserting
every public config field is referenced on some runtime path — is the
only one that would have caught these before release.**

**Your task is to establish whether option 3 is practical, and report
that as a result either way.** Timebox the investigation; if it turns out
to be impractical, saying so with the reason is a legitimate and useful
outcome — do not force it, and do not silently fall back to option 1
without saying you did.

If option 3 is impractical, fall back to option 2 (targeted checks for
whatever remains inert after § 3 and § 4) or option 1 (nothing), and give
the reasoning. What I do not want is a decision made by omission.

Scope guard from the RFC's non-goals: a decision and a principle, **not**
an exhaustive audit of every field.

## 6. Testing

- One test per `respond` shape asserting a custom header survives:
  `file_path`, `text`, `text` + `status`, `status` alone.
- A test asserting an explicit `content-type` beats the default.
- A test asserting `[default].delay_response_milliseconds` delays, and
  that a per-rule value overrides it.
- Full suite green. Baseline is **409 tests** as of v5.16.0/v5.17.0;
  report the new count and what it comprises.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. Scope boundaries

- **Out:** changing the `respond` schema or adding fields; reworking
  `ResponseHandler` beyond what these fixes need; the trace channel and
  `guard` (separate dispositions); simplifying the examples.
- The `guard` stub is an **owner decision**, still open. Do not touch it.
- If the header-precedence work starts reaching into unrelated response
  construction, stop and escalate rather than widening.

## 8. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package, not only in chat — including a
§ 5 finding that option 3 is impractical, which is a result worth
recording properly rather than a footnote.
