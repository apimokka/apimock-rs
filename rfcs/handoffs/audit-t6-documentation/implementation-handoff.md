# Handoff — Tranche 6: documentation

**Governing RFC.** [078](../../accepted/078-documentation-corrections.md).
Accepted 2026-09-01.
**Milestone.** Split — see § 1. **Part A can start now.**
**Baseline.** `main` @ `5d9e5bc`.

---

## 0. Why this is last, and why half of it is not

Four documented statements are false and three user-visible behaviours
are documented nowhere. But **most of those statements are false about
behaviour that tranches 2–5 change** — so writing the correction now
would document something about to be untrue again.

The RFC splits accordingly, and so does this handoff.

## 1. Part A — unblocked, do it now

None of these depends on another RFC:

- **D-03** — the TLS reload guide suggests an embedder "could reach
  `ServerHandle` and `reload_tls_certs` itself". `ServerHandle` is
  `#[non_exhaustive]`, so out-of-crate construction is a compile error,
  and nothing returns one. **The stated workaround is impossible.**
  **Attempt it and confirm it does not compile before writing the
  correction** — the point of this tranche is that we stop asserting
  things we have not checked.
- **D-06** — `response-headers.md` lists `connection: keep-alive` as
  always present. It is absent over HTTP/2; hyper strips it correctly
  per RFC 9113 §8.2.2. The table is accurate for HTTP/1.1 only.
- **D-04** — `threat-model.md` omits CORS entirely (RFC 067 adds the
  subsection; coordinate so it is written once, not twice).
- **D-05** — the same page says a failing middleware script "cannot
  crash the process, but it can silently degrade routing". Right about
  crashing; a non-terminating script wedges a worker (RFC 068).
- **The troubleshooting page** — organised by **symptom**, because that
  is what a user has: *"my file 404s"*, *"my rule matches everything"*,
  *"my snapshot test broke"*. Cross-link the error-`kind` taxonomy,
  which is closed, documented, and a genuine strength.

## 2. Part B — blocked, and what unblocks it

| Item | Blocked on | If that RFC lands | If it does not |
|---|---|---|---|
| **D-01** round-robin "Deterministic" | 070 | Remove the caveat | **Must** document the single-group constraint |
| **D-02** trace back-pressure | 073 | Becomes true | **Must** correct to describe `broadcast` |
| **D-07(a)** JSON key order | 076 | Item disappears | **Must** document the transformation |
| **D-07(b)** percent-decoding | 075 | Item disappears | **Must** document the limitation |
| **D-07(c)** case-insensitivity scope | 075 | Document whichever rule was chosen | Same |

> **If any of those RFCs is declined or deferred, its Part B item
> becomes mandatory and urgent** — an undocumented wrong behaviour is
> worse than a documented one. Do not let a deferral silently drop the
> documentation too.

## 3. The standard this tranche is held to

**Every corrected statement must be checked against a running server or
a compiler — not against the code by reading it.**

That is not boilerplate. Three of these four statements were written by
people reading the code and describing what they believed it did. D-02
describes a mechanism that does not exist; D-03 describes a workaround
that cannot compile. Reading is how they got here.

Likewise for the troubleshooting page: **reproduce each symptom and
apply each stated fix before writing it down.** An entry that names a
symptom and a check the reader can run fails visibly when it goes stale.
An entry that explains a cause misleads quietly.

## 4. Acceptance

**Part A**
- [ ] D-03 correction written **after** confirming the workaround does
      not compile — quote the compiler error
- [ ] D-06 corrected, scoped to HTTP/1.1, verified over both protocols
- [ ] D-04 and D-05 in `threat-model.md`, coordinated with RFCs 067/068
- [ ] Troubleshooting page, symptom-organised, every entry reproduced
- [ ] `mdbook build docs` clean; link check and stub check pass

**Part B**
- [ ] Each item either removed (its RFC landed) or documented (it did
      not) — **with the state of its RFC named**, so no item is silently
      dropped

## 5. Report back

`.git-exclude/review-request/audit-t6-documentation/`, including the
compiler error for D-03, and a Part B table stating each item's
disposition against its RFC's actual state.
