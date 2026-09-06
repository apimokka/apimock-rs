# Handoff — Tranche 6: documentation

**Governing RFC.** [078](../../accepted/078-documentation-corrections.md).
Accepted 2026-09-01.
**Milestone.** Next minor.
**Baseline.** **`main`'s head — cut from it.** No hash pinned; every
tranche so far has shipped with a baseline stale on arrival, and this
document cannot name the commit containing it.
**Branch.** **Not needed.** RFC 080 makes `main` the working branch,
and its § 3 carve-out is for changes that can behave differently on
Windows or macOS. This tranche is documentation plus verification —
nothing platform-dependent — so commit to `main` directly, and verify
`main`'s own CI run after each push (RFC 066 Amendment 3).

> ## ⚠️ This handoff was rewritten on 2026-09-07, and most of it is gone
>
> It was written on 2026-09-01 as a nine-item tranche split into "Part A
> (unblocked)" and "Part B (blocked on tranches 2–5)". **Tranches 1–5
> have all landed, and their authors wrote these corrections as they
> went.** Six of the nine items are already complete.
>
> The Part A / Part B split is therefore obsolete and has been removed.
> What follows is what is *actually* left, each item re-verified against
> `main` on the date above.

---

## 0. What is already done — do not redo these

Verified against `main` at rewrite time. If you find one of these
undone, trust the tree and tell me.

| Item | Was | Done by | Where |
|---|---|---|---|
| **D-01** round-robin "Deterministic" | claimed deterministic without qualification | RFC 070 (tranche 2) | `vary-the-response-for-one-path.md` — "Rotation is per *match group*" now states it |
| **D-02** trace back-pressure | described a drop-and-count `broadcast` does not have | RFC 073 (tranche 5) | `trace.rs` module docs — now describes per-subscriber `Lagged` accumulation, including the in-process caveat |
| **D-04** CORS absent from threat model | omitted entirely | RFC 067 (tranche 1) | `threat-model.md:184+` |
| **D-05** middleware "silently degrade routing" | wrong about non-termination | RFC 068 (tranche 1) | `threat-model.md:74+` — and it quotes the old wrong sentence while correcting it, which is the right way to do this |
| **D-07(a)** JSON key order | claimed reserialisation | RFC 076 (tranche 4) | `serve-json-files-from-a-folder.md:18` — "exactly as written — same key order, same whitespace" |
| **D-07(b)/(c)** percent-decoding, case scope | undocumented | RFC 075 (tranche 4) | `serve-json-files-from-a-folder.md:27+` — a "URL-to-file resolution" section covering decoding, case folding at every segment, and the filesystem caveat |

**This is the write-it-as-you-go discipline working.** Each tranche
wrote its own documentation rather than deferring it here, which is why
this tranche shrank to a third of its planned size.

## 1. What is actually left

### D-03 — the TLS guide contradicts itself

`docs/src/guides/reload-tls-certificates-without-restart.md` says, at
`:26`:

> `ServerHandle` is never constructed anywhere in this repository

and then, at `:41`:

> [an embedder] could reach `ServerHandle` and `reload_tls_certs` itself

Both cannot be true for an out-of-crate caller. `ServerHandle` is
`#[non_exhaustive]`, so literal construction from outside is a compile
error, and nothing returns one.

**Attempt it before writing the correction.** Write a throwaway crate
that depends on `apimock-server` and tries to obtain a `ServerHandle`
by any route you can find. Confirm it does not compile, **quote the
compiler error**, and only then write what the page should say. If it
*does* compile — if there is a route neither the audit nor I found —
that is a finding, and the page needs a working example rather than a
retraction.

This is the item the tranche exists for: a documented workaround nobody
tried.

### D-06 — `connection: keep-alive` is HTTP/1.1 only

`docs/src/reference/response-headers.md:14` lists `connection` /
`keep-alive` in a table of headers presented as always present. It is
absent over HTTP/2 — hyper strips it correctly per RFC 9113 § 8.2.2.

**Verify over both protocols against a running server** before writing
the qualification. `curl --http2-prior-knowledge` against a TLS
listener will do it; the point is to observe the absence, not to
reason about it from the spec.

### The troubleshooting page — new, and the substantial piece

Does not exist yet. Organise it by **symptom**, because that is what a
user arrives with:

- *"my file 404s"* — case, extension inference, percent-encoding,
  `fallback_respond_dir` scope, confinement
- *"my rule matches everything"* — the RFC 069 unknown-key rejection is
  now the likely cause and gives a clear error; before 6.1.0 it was
  silent
- *"my snapshot test broke"* — RFC 076 serves bytes verbatim now
- *"my CORS request fails"* — `cors_allow_credentials_origins`, the
  6.1.0 change most likely to be reported as a regression
- *"my request is refused with 413"* — `max_request_body_bytes`

Cross-link the error-`kind` taxonomy, which is closed, documented, and
a genuine strength of this project.

**Reproduce each symptom and apply each stated fix before writing it
down.** An entry naming a symptom and a check the reader can run fails
visibly when it goes stale; an entry explaining a cause misleads
quietly.

## 2. The standard this tranche is held to

**Every corrected statement must be checked against a running server or
a compiler — not against the code by reading it.**

That is not boilerplate, and D-03 is the proof: it describes a
workaround that cannot compile, written by someone reading the code and
describing what they believed it did. Reading is how it got there.

## 3. Acceptance

- [ ] **D-03** — compile attempt made, compiler error quoted, page
      corrected (or a working example added, if it compiles)
- [ ] **D-06** — observed over HTTP/1.1 *and* HTTP/2 against a running
      server; table qualified
- [ ] **Troubleshooting page** — every symptom reproduced and every fix
      applied before being written down; registered in `SUMMARY.md`
- [ ] The § 0 table spot-checked — if any "already done" item is not
      actually done, say so rather than silently fixing it, because
      that means this handoff was wrong again
- [ ] `mdbook build docs` clean; link check green
- [ ] CI green on `main` after each push (RFC 066 Amendment 3)

## 4. Report back

`.git-exclude/review-request/audit-t6-documentation/`, including D-03's
quoted compiler error and D-06's observed output on both protocols.

This is the last tranche of the external audit.
