# Handoff — Tranche 5: observability and hygiene

**Governing RFCs.** [073](../../accepted/073-observability-correct-and-safe.md)
(trace channel, redaction), [079](../../accepted/079-dead-and-misleading-code.md)
(dead code). Accepted 2026-09-01.
**Milestone.** Next minor.
**Baseline.** `main` @ `5d9e5bc`.

---

## 0. Why these two together

Both are about **code and output that assert something untrue**. The
trace feed says "miss" for every match; three public `validate()`
methods validate nothing. Different severity, same category, and both
are cheap.

## 1. 073 — the trace channel

**Verified:** `server.rs:413` emits `Outcome::Miss { status: 0 }` with
the comment *"coarse-grained; fine-grained tracing is a future pass"* —
for **every** request, matches included. The correct index is computed
on the adjacent line and discarded.

Nothing is emitted at all for middleware, fallback-directory, or 404
responses.

**Three sub-problems, and the RFC treats them separately for a reason:**

**(a) Wrong events.** Emit `Outcome::Matched` with the indices already
in hand, and add the missing emit sites.

**(b) Documented back-pressure that does not exist.** `trace.rs:26-34`
describes a drop-and-count mechanism `broadcast` does not have —
`send` fails only when there are **no receivers**, and `Lagged(n)` is
discarded at `:532-537`. The RFC recommends **implementing** the
documented behaviour rather than correcting the docs, because a consumer
otherwise cannot distinguish a quiet server from a dropped feed.

**(c) Redaction stops short.** Header redaction is genuinely good —
RFC 051 built it with allowlist, denylist and a mode. Bodies and query
strings are printed **unredacted**. A bearer token in a header is
redacted; the same token in `?access_token=` is not.

> **(c) is the one with a privacy consequence, and the easiest to
> under-deliver.** If full body redaction proves impractical, a size cap
> plus opt-in is acceptable — **but the documentation must then say
> which was chosen.** Silent partial redaction is worse than none,
> because it invites trust the implementation does not earn.

**Transport access control** is the fourth item: restrict Unix-socket
permissions, and document that the TCP transport has no authentication.
State the platform behaviour rather than assuming it — Windows has no
equivalent and the docs should say so.

## 2. 079 — the hygiene tail, with two judgement calls

**Verified:** all three are literally `pub fn validate(&self) -> bool
{ true }` — `rule_set.rs:343`, `guard.rs:10`,
`default_respond.rs:10`. And `bad_request_response` really is called
from nowhere.

**Do not delete `bad_request_response`.** RFC 068 (tranche 1) gives it
its first caller, and audit F-09 wants another. A dead function about to
be used should get a comment saying so, not a removal.

**Do not remove the no-op `validate()` methods within 6.x.** They are
public API; `RuleSet::validate()` is reachable by a consumer, and
removal would move the API baseline and break the additive-only promise.
The RFC recommends **documenting them as intentionally trivial** and
revisiting at the next incompatible release.

**Do the minimum on M-03a.** 32 sites of `let _ = write!` swallow a
`fmt::Error` that cannot occur when writing to a `String`. A comment at
the pattern's definition is the honest fix; 32 edits is churn that will
obscure the rest of this tranche in review.

**If removing anything changes behaviour, it was not dead — stop and
report.**

## 3. Acceptance

**073**
- [ ] A matched request emits `Matched` with the **correct rule-set and
      rule index** — assert the indices, not just the variant
- [ ] Middleware, fallback and 404 paths each emit
- [ ] Dropped events reported, **or** the docs corrected — say which,
      and pin it with a test either way
- [ ] `?token=secret` and a secret in a body appear in **neither** under
      default settings
- [ ] Existing header-redaction tests pass unchanged
- [ ] Transport permissions set; platform behaviour documented

**079**
- [ ] Full suite green, **no behaviour change anywhere**
- [ ] **API baseline diff empty** — if it moved, something public was
      removed and needs the § 2 decision
- [ ] `Display` renders a value, pinned by a test
- [ ] `bad_request_response` retained, with a comment

## 4. Report back

`.git-exclude/review-request/audit-t5-observability-hygiene/`, including
the redaction decision from § 1(c), the back-pressure decision from
§ 1(b), and anything you declined to remove under § 2 and why.
