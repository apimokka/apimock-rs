# Implementation Handoff — RFC 050, body presence in trace events

**Governing RFC.** [RFC 050](../../proposed/050-non-json-body-capture-decision.md)
**Milestone.** M3 — P2, targeting **v5.19.0**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

`body_json: None` in a trace event means *either* "no body" *or* "a body
arrived and wasn't JSON". A consumer cannot tell which. Close that gap —
**without capturing body content.**

RFC 050 asked whether non-JSON bodies should be captured at all and the
answer is **presence only**. The owner confirmed the GUI wants this.

## 2. Everything is decided. Nothing is open.

- **Answer (2), presence only.** Not content, not a snippet, not base64.
- **Populated always**, not gated on tracing being active — the values
  are already computed, so there is nothing to gate.
- **Content type needs no capturing.** It is an ordinary header, it is
  not on RFC 040's denylist, and `RequestSummary.headers` already
  carries it. A consumer can read `content-type` from the event today.
  Do not add a second copy.

If any of that turns out to be wrong when you read the code, that is a
contradiction worth reporting rather than working around.

## 3. The work

Small, and the values you need already exist.

`parsed_request_from` (`crates/apimock-server/src/parsed_request.rs`)
already holds `body_bytes` and derives `has_body` from it, at the point
where the bytes still exist. Nothing needs measuring; it needs
propagating.

- **`apimock_routing::ParsedRequest`** gains one small **additive** field
  carrying body presence and byte length.
- **`parsed_request_from`** populates it from what it already computes.
- **`RequestSummary`** surfaces it, distinguishing three states:
  **no body** · **JSON body captured** · **non-JSON body present, size
  known, not captured.**

The three states are the point. Two of them exist today and are
conflated; a consumer must be able to tell them apart, which is the
whole reason this RFC exists.

## 4. The part that is real work, not a formality

**`ParsedRequest` is the type the matcher, middleware and `dyn_route`
all consume.** The change is additive, so there should be no effect on
any of them.

*Should be* is not *was verified*. Check every consumer and say in your
review request which ones you checked. If the field turns out not to be
purely additive for some consumer — a struct literal somewhere, an
exhaustive destructure — that is exactly the kind of thing this
instruction exists to surface, and it may mean the change is larger than
RFC 050 estimated.

## 5. Do not capture content

Stated plainly because it is the one way this work could go wrong in a
way that matters. **No bytes, no snippet, no truncated preview, not even
"just the first few characters for debugging".** A
`application/x-www-form-urlencoded` login body is
`username=alice&password=hunter2`, and RFC 040's name-based redaction
cannot help — there are no header names to match, and parsing bodies to
find field names is the value-scanning problem both RFCs declared a
non-goal.

Length and presence only.

## 6. Evidence required

- A request with **no body**, one with a **JSON body**, and one with a
  **non-JSON body** produce three distinguishable trace events —
  asserted on the **serialised** form, since that is what reaches a
  consumer.
- The non-JSON case reports a byte length and **no content**. Assert
  that a recognisable string from the body does **not** appear anywhere
  in the serialised event.
- The JSON path is unchanged: `body_json` and `body_truncated` behave
  exactly as before, existing tests untouched.
- Consumers of `ParsedRequest` enumerated and checked (§ 4).
- Full suite green; report the count against the **430** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. Scope boundaries

- **In:** `apimock-routing`'s `ParsedRequest`, `apimock-server`'s
  `parsed_request_from`, `trace.rs`'s `RequestSummary`, documentation.
- **Out:** body content of any kind; response bodies; JSON capture
  behaviour; matching, dispatch, response construction; `log.verbose`
  logging — that is [RFC 051](../../proposed/051-verbose-log-header-redaction.md).
- If the additive field turns out not to be additive, **stop and
  escalate** rather than reshaping consumers to fit.

## 8. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package.
