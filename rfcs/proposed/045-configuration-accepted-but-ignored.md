# RFC 045 — Configuration that is accepted but ignored

**Status.** Proposed — design approved by the project owner 2026-08-04.
Unresolved questions 1 and 2 remain open and are decided at
implementation time, not here.
**Tracks.** Correctness. Two shipped configuration fields parse, pass
`apimock validate`, print at startup, and have no runtime effect.
**Touches.** `apimock-server` (response construction), `apimock-routing`
(rule-set default), and — the reason these belong in one RFC —
`apimock-config`'s validation surface.

## Summary

Two configuration fields are silently inert:

- `respond.headers` is dropped entirely on `status`-bearing responses,
  and its `content-type` entry is overwritten on `text` responses.
- `[default].delay_response_milliseconds` on a rule set never delays
  anything.

Fix both, and decide whether validation should have caught them.

## Motivation

### How these were found

Both surfaced during RFC 036, while writing examples that were required
to actually run. Neither had been noticed by code review, by the test
suite, or by `apimock validate`. They were found because someone tried
to demonstrate the feature and the demonstration did not work.

That is worth recording: the examples RFC was scoped as an onboarding
fix and paid for itself in defects.

### Defect 1 — `respond.headers` on non-file responses

Two distinct faults, verified against source:

**(a) Never passed.** `status_code_response` and
`status_code_response_with_message`
(`crates/apimock-server/src/response/status_code_response.rs:6-25`) take
no headers parameter at all, and `respond_response.rs:78-81` never
attempts to pass one. So on any rule with `status` set — with or without
`text` — **every** custom header is dropped.

**(b) Overwritten.** On a `text`-only response, custom headers are
applied, then `with_text(content, None)` unconditionally re-inserts
`content-type`, defaulting to `text/plain; charset=utf-8` and clobbering
an explicit override.

| `respond` shape | `respond.headers` |
|---|---|
| `file_path` | honoured |
| `text` alone | honoured, except `content-type` is overwritten |
| `text` + `status` | **all dropped** |
| `status` alone | **all dropped** |

The practical impact is on error responses: returning
`content-type: application/problem+json` with a 4xx, or any correlation
header alongside a status, silently does not work.

### Defect 2 — `[default].delay_response_milliseconds` is dead

`RuleSet.default` has exactly one field, and its name and position imply
a rule-set-wide default delay. `delay_response_milliseconds` is read in
exactly one place — `respond_response.rs:45`, from `respond` — so the
per-rule field works and the rule-set default does nothing.
`RuleSet.default` is read only at `rule_set.rs:312`, for display.

Measured: 2000 ms configured, ~4 ms observed. The per-rule equivalent
delays correctly.

### The finding underneath both

**`apimock validate` passes on configuration that does nothing.**

RFC 026 shipped `validate` so users could trust a config before running
it. Both defects above produce configuration that parses, validates
clean, and is printed back at startup as though it were in effect. A user
following the documentation, then validating, gets no signal at all.

That is the more serious problem. A silently-ignored field is a bug; a
validator that certifies it is a bug in the thing users were told to
trust.

## Goals

1. `respond.headers` is honoured on every `respond` shape.
2. An explicit `content-type` in `respond.headers` wins over the default.
3. `[default].delay_response_milliseconds` either works or is removed.
4. Decide whether validation should detect inert configuration, and if
   so what that means in general.

## Non-goals

- Changing the `respond` schema or adding fields.
- Reworking `ResponseHandler` beyond what these fixes need.
- The trace channel and `guard` — RFC 036's Escalation 004, separate
  dispositions.
- Retrofitting inert-config detection across every field; goal 4 asks for
  a decision and a principle, not an exhaustive audit.

## Proposed design

### Defect 1

Thread `respond.headers` into the status-response paths, and make an
explicit `content-type` take precedence over the default.

The precedence rule should be stated once and applied everywhere:
**an explicitly configured header wins over an inferred default.** That
is the behaviour a user expects and the only rule that makes
`respond.headers` meaningful.

Care is needed that this does not become "custom headers win over
`file_path`'s inferred content type" in a way that breaks existing
file-response behaviour — `file_path` currently honours headers
correctly, and its inferred content type comes from the file extension.
Verify against the existing suite rather than reasoning about it.

### Defect 2 — a decision, not just a fix

Two coherent options:

**A. Implement it.** A rule-set-wide default delay is a sensible feature
and the field's name promises it. Per-rule `respond.delay_response_milliseconds`
overrides it. Backward-compatible: configs that set it start behaving as
they always appeared to.

**B. Remove it.** If a rule-set-wide default is not wanted, deleting the
field is honest. But it is a **breaking change to the TOML schema** —
configs currently setting it would fail to parse — so it needs a
deprecation path, and it is hard to argue for given (A) is
straightforward.

**Recommend A.** Noted as a decision rather than assumed, because "make
the dead field work" and "delete the dead field" are both defensible and
the choice is the owner's if it changes user-visible behaviour.

Note that (A) is technically a behaviour change: a config that today
delays nothing would start delaying. Anyone relying on the *current*
behaviour is relying on a bug, but it should be called out in the
release notes.

### Goal 4 — validation of inert configuration

The open design question. Options, in increasing ambition:

1. **Nothing.** Fix the two defects; accept that validation cannot know
   what the runtime ignores.
2. **Targeted.** Add checks for the specific cases known to be inert, if
   any remain after the fixes. Cheap, but only ever catches what someone
   already noticed.
3. **Structural.** Make it impossible for a field to be parsed and never
   read — e.g. an exhaustiveness test asserting every public config field
   is referenced somewhere in the runtime path.

(3) is the only option that would have caught these before release, and
is the one worth exploring. It may be impractical; that is a legitimate
outcome, but it should be established rather than assumed.

## Testing and verification

- A test per `respond` shape asserting a custom header survives:
  `file_path`, `text`, `text`+`status`, `status` alone.
- A test asserting an explicit `content-type` overrides the default.
- A test asserting `[default].delay_response_milliseconds` actually
  delays, and that a per-rule value overrides it.
- The RFC 036 example sets that were **designed around** these defects
  (`match-headers-and-body/`, `status-codes-and-errors/` use `file_path`
  specifically because `text` + custom headers does not work) should be
  revisited: once fixed, they can demonstrate the simpler form.
- Full suite green — **409 tests** as of v5.16.0's baseline.

## Risks

| Risk | Notes |
|---|---|
| Header-precedence change breaks `file_path` responses | Currently correct; must stay correct. Existing suite is the guard |
| Implementing the default delay changes observed behaviour | Anyone relying on the current behaviour relies on a bug, but it belongs in release notes |
| Goal 4 expands without limit | Bounded by the non-goals: a decision and a principle, not an exhaustive audit |

## Unresolved questions

1. **Defect 2: implement or remove?** Recommend implement. Owner
   decision if it is treated as user-visible.
2. **Goal 4: how far?** Options 1–3 above. Worth at least establishing
   whether option 3 is practical before settling for 1.
3. **Do the RFC 036 examples get simplified once this lands?** They
   currently route around defect 1 deliberately. Simplifying them is
   optional and would touch a shipped example set.
