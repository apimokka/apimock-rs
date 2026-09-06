# Handoff — Tranche 5: observability and hygiene

**Governing RFCs.** [073](../../accepted/073-observability-correct-and-safe.md)
(trace channel, redaction), [079](../../accepted/079-dead-and-misleading-code.md)
(dead code). Accepted 2026-09-01.
**Milestone.** Next minor.
**Baseline.** **`main`'s head — cut from it.** No hash is pinned: this
document cannot name the commit that contains it, and every tranche so
far has shipped with a baseline that was stale on arrival. Tranches 1–4
are merged; 6.1.0 is tagged.
**Branch.** **Take one.** RFC 080 (adopted after this handoff was
drafted) makes `main` the working branch, but its § 3 carve-out keeps a
short-lived branch for anything that can behave differently on Windows
or macOS. 073's transport-access-control item is exactly that — Unix
socket permissions, with no Windows equivalent. Cut from `main`, merge
green, delete.

> **This handoff was refreshed on 2026-09-06 before being sent.** It was
> written on 2026-09-01, before tranches 2–4 landed. Line numbers, one
> count, one prediction and one justification had all gone stale; each
> correction is marked inline below rather than silently applied, so you
> can see what changed under it.

---

## 0. Why these two together

Both are about **code and output that assert something untrue**. The
trace feed says "miss" for every match; three public `validate()`
methods validate nothing. Different severity, same category, and both
are cheap.

## 1. 073 — the trace channel

**Verified:** the emit is
`grep -n 'Outcome::Miss' crates/apimock-server/src/server.rs` —
`Outcome::Miss { status: 0 }`, with the comment *"coarse-grained;
fine-grained tracing is a future pass"* — for **every** request, matches
included. The correct index is computed on the adjacent line and
discarded.

> **Corrected on refresh:** this said `server.rs:413`. Tranche 3 (RFC
> 071) restructured `server.rs`; the emit is now at line 565. Given as
> a search, not a number — the number will move again.

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

**Verified — and there are four, not three.** Find them yourself
rather than trusting a list:

```
grep -rn -A2 "pub fn validate(&self) -> bool" crates/apimock-routing/src/ | grep -B1 "true$"
```

which today returns `rule_set.rs`, `rule_set/guard.rs`,
`rule_set/default_respond.rs`, **and
`rule_set/rule/when/request/url_path.rs`**.

> **Corrected on refresh:** this said "all three", listing
> `rule_set.rs:343`, `guard.rs:10`, `default_respond.rs:10`. All three
> line numbers had moved (427, 11, 11), the `default_respond.rs` path
> was wrong, and **`url_path.rs` was missed entirely**. That is the
> fourth time a handoff of mine has shipped an incomplete file list, so
> this one gives the search instead. If the grep returns five, treat
> five as the answer — not this paragraph.

**Do not delete `bad_request_response`** — but the reason has changed.

> **Corrected on refresh:** this said "RFC 068 (tranche 1) gives it its
> first caller." **Tranche 1 has merged and it did not.**
> `grep -rn bad_request_response crates/ --include='*.rs'` returns only
> its own definition. Do not take the prediction on trust; re-run that
> grep, and if it is still uncalled, say so in your package.

Audit F-09 still wants a caller for it, so a comment recording that is
better than a removal. **But if you conclude it should simply go, make
that case** — "a function kept for a caller that never arrived" is a
finding too, and one this handoff was wrong about once already.

**The no-op `validate()` methods: decide, don't default.**

> **Corrected on refresh — this is the important one.** This said:
> *"removal would move the API baseline and break the additive-only
> promise."* **That justification has been retracted.** RFC 039 is a
> *declaration* gate, not an additive-only one — its own non-goals say
> deciding whether a break is allowed "is semver's job and the owner's,
> not forbidden" — and `docs/src/library/api-stability.md` was corrected
> accordingly at `52fa18b`. Since then 6.1.0 has removed
> `RuleSet::round_robin_counter` and `AppState`'s `Clone` impl, both
> declared and documented. A public removal inside 6.x is **permitted**,
> not forbidden.

So the constraint I gave you does not exist. What remains is a real
choice, and it is yours to make and state:

- **Keep and document them as intentionally trivial** — the RFC's own
  recommendation, and still defensible: they cost nothing and removal
  buys little.
- **Remove them**, declare the baseline change, and write the migration
  entry — now a legitimate option, on the same footing as the two
  removals 6.1.0 already shipped.

Either is acceptable. **Choosing by accident is not** — say which and
why, the same way tranche 4 handled the envelope decision.

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
- [ ] **API baseline diff empty *or* declared** — empty if you kept the
      no-op `validate()` methods; declared, with a migration entry, if
      you removed them. Either is a valid outcome of § 2's decision;
      what is not valid is a baseline that moved without you saying so.
- [ ] The § 2 `validate()` decision **stated with its reasoning**
- [ ] `Display` renders a value, pinned by a test
- [ ] `bad_request_response`: the grep re-run and its result reported,
      and either retained with a comment or removed with the case made

## 4. Report back

`.git-exclude/review-request/audit-t5-observability-hygiene/`, including
the redaction decision from § 1(c), the back-pressure decision from
§ 1(b), and anything you declined to remove under § 2 and why.
