# Handoff — Tranche 1: security and availability

**Governing RFCs.** [067](../../accepted/067-cors-credential-reflection.md)
(CORS), [068](../../accepted/068-bound-per-request-resources.md)
(request resource bounds), [074](../../accepted/074-tls-availability.md)
(TLS). All accepted 2026-09-01.
**Source.** Independent audit of 6.0.0, 2026-08-31 → 09-01. Review at
`.git-exclude/reviewed/external-audit-2026-09-01/REVIEW-001.md`.
**Milestone.** Next minor. **First tranche — do this one first.**
**Baseline.** `main` @ `5d9e5bc`.

---

## 0. Why these three together, and why first

They are one shippable unit: every one is *external input the process
does not bound or check*, and none changes matching or config semantics,
so they can land without the migration story tranche 2 needs.

**067 is the highest-ranked finding in the audit** and the reason this
tranche is first.

## 1. Read the RFCs; this document does not repeat them

Each RFC carries the motivation, the verified reproduction, the design
and the risks. This handoff covers what they do not: how the three
interact, what will trip you, and what "done" means.

**All three RFCs' headline claims were re-verified against a running
server before they were written** — by the auditor, then independently
by me. You should still reproduce each before fixing it; a fix for a
defect you have not seen is a guess.

## 2. Order within the tranche, and why it is not arbitrary

**074's S-08 first** (TLS failure must not silently degrade to HTTP).
It is the smallest, it is independent, and leaving it last risks it
being dropped if the tranche runs long — which would be the wrong thing
to drop, because a user who thinks they have TLS and does not is worse
off than one with no timeout.

**Then 068**, then **067**. 067 is the largest because it adds a config
setting and a threat-model section.

## 3. The traps

**067 — do not make credentialed reflection configurable-but-default-on.**
The RFC's design is: unlisted credentialed origins degrade to
`ACAO: *` *without* credentials. The response is still served. If you
find yourself returning an error for an unlisted origin, re-read § Design
— many requests carry a `Cookie` incidentally and need no CORS at all.

`Vary: Origin` must be sent whenever the origin is reflected, or a
shared cache will hand one origin's response to another.

**068 — the operation limit is not the fix on its own.** It is the
easier half and it looks sufficient. `spawn_blocking` is what turns
"one fewer worker, permanently" into "one slow request". Implement both
or neither, and say which you did.

**068 — assert on memory, not status.** A 413 can be returned *after*
the body was buffered, which passes a status assertion while leaving the
defect. The audit measured RSS; so should the test.

**074 — do not add a "HTTPS if possible, else HTTP" flag.** If someone
asks for it later it is a feature with a design. Today it is a failure
mode, and preserving it behind a flag preserves the bug.

## 4. Where to look — search, do not trust a list

Starting points, **not a complete set**:

- `crates/apimock-server/src/response_handler.rs` — `is_likely_authenticated_request` is at `:274`; the CORS block above it
- `crates/apimock-server/src/parsed_request.rs` — body collection
- `crates/apimock-server/src/middleware/middleware_handler.rs` — `Engine::new()` and `eval_ast_with_scope`
- `crates/apimock-server/src/server.rs`, `tls.rs` — listener setup

**Then grep** for every other place each concern appears: other response
constructors that set CORS headers, any other body collection, any other
`Engine::new()`. My file lists have been incomplete three times in this
project; treat these as a floor.

**Verified for you so you need not re-derive it:** Rhai's `sync` feature
*is* already enabled (`Cargo.toml:62`), so the engine is `Send` and
`spawn_blocking` is available without a dependency change. And
`bad_request_response` exists and is called from nowhere — 068 gives it
its first caller.

## 5. Acceptance

**067**
- [ ] The four rows of RFC 067 § Design's table, against a live server
- [ ] `Vary: Origin` present whenever an origin is reflected
- [ ] A loopback origin works with credentials and **no** configuration
- [ ] A non-loopback origin gets no credentials until listed
- [ ] `threat-model.md` gains the CORS subsection (audit D-04)

**068**
- [ ] Body one byte over the limit → **413**, and **RSS does not grow by
      the body's size**
- [ ] A `while true` script fails that request, **and the server answers
      a later request on a different connection** — the second assertion
      is the finding
- [ ] N concurrent such scripts do not reduce throughput to zero
- [ ] Both limits configurable; defaults documented

**074**
- [ ] Malformed PEM → server **exits**, names the file, **and did not
      bind an HTTP listener**
- [ ] An incomplete handshake is dropped, **and the server serves
      throughout**
- [ ] Existing TLS tests, including reload, unchanged

**All three**
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `cargo audit`, `mdbook build docs`
- [ ] **The public-API baseline diff is empty**, or every change is
      declared and explained (RFC 039's gate)
- [ ] CI green on all 12 jobs before merge

## 6. Not in scope

- Connection-count limits beyond what 074 specifies.
- Sandboxing Rhai's capabilities — code execution is a documented,
  deliberate allowance; this is about termination and thread ownership.
- Anything in tranches 2–6.

## 7. Report back

`.git-exclude/review-request/audit-t1-security-availability/`, including:

- The **before** reproduction of each defect, not only the after.
- The RSS numbers for 068, and the concurrency result.
- Anything in § 4's search that the file list missed.
