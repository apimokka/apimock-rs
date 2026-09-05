# Handoff — Tranche 4: fidelity

**Governing RFCs.** [075](../../accepted/075-url-path-fidelity.md)
(URL path), [076](../../accepted/076-serve-responses-as-written.md)
(JSON bytes). Accepted 2026-09-01.
**Milestone.** Next minor.
**Baseline.** `main` @ `761bdb2` — this handoff's own refresh commit.
Tranches 1–3 are merged; § 2a below is a consequence of tranche 3 that
did not exist when this handoff was drafted. § 2 and § 2a's numbers
were measured at `c98013a`; the only commit between that and the
baseline (`f5f821e`) changed a doc comment and the migration guide's
wording, no code or test behaviour, so they still hold.
**Branch.** **Take one.** RFC 080 made `main` the working branch, but
its § 3 carve-out keeps a short-lived branch for anything that can
behave differently on Windows or macOS. This tranche is *entirely*
that: percent-decoding, filesystem case, and path resolution. Only
CI's `test` job is matrixed and the development machine is Linux.
Cut from `main`, merge when CI is green, delete.

> ## 🔒 Read § 1 before writing any code for 075
>
> 075 adds percent-decoding. **Done in the wrong order it reintroduces
> the path-traversal advisory this project published in August**
> (GHSA-72g6-wgrg-vhm7).

---

## 0. Why these two together

Both are the same promise: **what you put in is what comes out.** A file
whose name needs encoding should be reachable; a JSON file should arrive
as written. Both currently fail silently — a 404 with no explanation, or
a snapshot diff nobody caused.

## 1. The ordering rule in 075, stated as plainly as possible

**Percent-decode BEFORE dot-segment normalisation. Normalise after.**

Today `%2e%2e` does **not** traverse — the audit verified that — and the
only reason is that decoding never happens at all. So:

> Adding decoding without reordering converts a missing feature into a
> **security regression**. `%2e%2e` would decode to `..` *after*
> normalisation had already run, and reach path resolution unnormalised.

RFC 063's confinement is the backstop and **must stay**. It is not a
substitute for getting the order right — defence in depth means both,
and relying on the second layer because the first is wrong is how the
original advisory happened.

**Required before this ships:**

- [ ] `%2e%2e%2f`, `..%2f`, `%2e%2e/` and mixed-case `%2E%2E` all fail
      to traverse
- [ ] Assert on the **HTTP response**, not the resolved path — a
      resolved-path assertion can pass while the response still leaks
- [ ] Every existing traversal test passes **unchanged**
- [ ] The confinement check is still reached — prove it, do not assume
      the ordering made it unnecessary

If any of that is awkward to test, say so and stop. This is the one
place in the whole audit where a fix can be worse than the defect.

## 2. Case — decided 2026-09-01, and the reasoning shapes the implementation

Open when this handoff was first written; **now settled: uniformly
case-insensitive, enforced by apimock at every segment.** RFC 075
§ Design carries the reasoning. The part that changes what you build:

**Do not delegate any segment to the filesystem.** Measured on Linux,
with `sub/users.json` on disk — **re-measured on `c98013a`** for this
send, since tranche 3 restructured the resolver underneath it, and both
rows still hold:

| Request | Result |
|---|---|
| `/sub/USERS.json` | **200** — apimock folds the filename itself |
| `/SUB/users.json` | **404** — the parent segment goes to the filesystem |

Filesystem case behaviour is **not portable**: Linux is case-sensitive;
Windows (NTFS) and macOS (APFS default) are case-insensitive. So that
second row returns **404 on Linux and 200 on Windows and macOS** — same
config, same request, same binary.

**Linux is the outlier, and Linux is what CI runs**, so the failure mode
is "works on my laptop, 404s in CI".

Consequences:

- Extend the case-insensitive comparison the final segment already gets
  to **every** segment. It lives in `find_by_case_insensitive_listing`
  in `crates/apimock-server/src/dyn_route.rs` — `eq_ignore_ascii_case`
  over the directory listing. (This handoff originally cited
  `dyn_route.rs:118-127`; tranche 3 restructured the file and that range
  is now unrelated code. Search, don't trust the number.)
  **Do not construct a path and let the OS resolve it** — but see § 2a,
  because tranche 3 now does exactly that on the fast path.
- Case-sensitivity was never a free alternative: enforcing it would need
  an explicit post-resolution case check on every segment, because a
  case-insensitive filesystem opens `SUB/users.json` whatever apimock
  intended — and it would reverse the deliberate, documented
  accommodation at `dyn_route.rs:18-25`.
- **Test on all three platforms.** RFC 061's matrix is exactly what this
  finding needs; a Linux-only test passes with the defect intact on the
  other two.
## 2a. What tranche 3 changed under this RFC — read before § 2's plan

This section did not exist when the handoff was drafted. Tranche 3's
RFC 077 P-06 (merged at `c98013a`) restructured `dyn_route.rs` to avoid
listing the directory on every request. The new order is:

1. exact-path `stat` (`is_existing_file`, a plain `path.is_file()`)
2. extension inference
3. the case-insensitive listing — now the *last* resort

**Two consequences for 075, neither of them the dev team's doing.**

**(a) The fast path now delegates final-segment case folding to the
OS** — the one thing § 2 says not to do. On a case-insensitive
filesystem, step 1's `stat` for `/USERS.json` resolves `users.json` at
the OS level and returns before the listing is ever reached. The
*outcome* still happens to be case-insensitive everywhere, so nothing
is broken today; but the mechanism is now split — apimock folds the
case on Linux, the OS folds it on macOS/Windows. § 2's plan to "extend
the comparison to every segment" has to decide what to do about a fast
path that resolves some segments without consulting that comparison at
all.

**(b) That split is not equivalent for non-ASCII names — and 075 is
what will expose it.** `eq_ignore_ascii_case` folds `A–Z`/`a–z` and
nothing else. A case-insensitive APFS or NTFS volume folds Unicode.

**Measured on this baseline (Linux, `c98013a`)**, with `café.json` on
disk in the fallback dir:

| request | today |
|---|---|
| `/café.json` | **404** |
| `/CAF%C3%89.json` | **404** |

So a non-ASCII filename does not resolve **at all** right now, on any
platform — the percent-decoding 075 adds is exactly what makes it
reachable. Which means this is **a divergence 075 creates, not one it
inherits**, and that is the more useful way to hold it: once decoding
lands, `/CAFÉ.json` decodes to a name that step 1's `stat` can resolve
on a case-insensitive volume, while Linux falls through to a listing
comparison that cannot. Same config, same request, different answer per
OS — the precise failure § 2 exists to prevent, arriving through the
fix rather than despite it.

**This is distinct from § 4's Unicode-normalisation scope-out.** NFC/NFD
is how a character is *encoded*; this is **case folding** of a
correctly-encoded one. § 4 scopes out the first and does not answer the
second, and 075 cannot leave it unanswered, because uniform case
behaviour is the entire point of the RFC.

**Honest status of this analysis.** The `404`s above are measured. (a)
is read directly from the merged code. The macOS/Windows half of (b) is
*reasoned* — `str::eq_ignore_ascii_case` is ASCII-only by definition and
case-insensitive APFS/NTFS fold Unicode — but **I could not run it on
either platform**, so treat it as a hypothesis with a test attached, not
a finding. The same reasoning about case-insensitive `stat` was raised
against tranche 3 and CI confirmed it, so the prior is good. Confirm it
anyway, and tell me if it does not hold.

**What to do — a decision for you to make and state, not to infer:**

- [ ] **Reproduce (b) first** on all three CI platforms, before
      changing anything. If it does not reproduce, say so and this
      section collapses.
- [ ] Decide how case folding is defined for non-ASCII, and say which:
      ASCII-only everywhere (uniform, and `/CAFÉ.json` 404s on every
      platform — which would mean *removing* the OS's folding from the
      fast path), or Unicode-aware everywhere (uniform, and it resolves
      everywhere — which means apimock does the folding, not the OS).
- [ ] Whichever you pick, the fast path must not be able to disagree
      with the listing. A `stat` that resolves a name apimock's own
      comparison would reject is the bug, whichever direction it goes.
- [ ] Do not silently undo P-06's optimisation to get there. If
      uniformity costs the fast path, say what it costs — the
      `directory_scaling` bench tranche 3 added is how you measure it.

## 3. The decision 076 does not make for you

Enabling `serde_json/preserve_order` for inline `respond.json` changes
**every** `Value` in the workspace — including the RFC 053 CLI envelope,
whose field order becomes insertion order rather than alphabetical.

That order was documented as alphabetical during RFC 064, and a consumer
may have adapted to it.

**Either** accept the change and note it in the migration guide, **or**
scope `preserve_order` so it does not reach the envelope. Both are
legitimate. **Choosing by accident is not** — say which you did and why.

Verified for you: `preserve_order` is currently **not** enabled
(`Cargo.toml:83`, plain `serde_json = "1"`).

## 4. Other traps

**076 — compare bytes, not parsed equality.** A test asserting the
response parses to the same JSON will pass with the defect present. The
finding is about *bytes*: minification and key order.

**076 — serving bytes skips a validity check, and that is fine** —
RFC 065 moved `.json` validation to load time. Depend on that, and say
so in a comment, so anyone who later reconsiders RFC 065 sees what
relies on it.

**075 — Unicode *normalisation* is out of scope**, and the docs should
say so. NFC/NFD is filesystem-dependent and a rabbit hole; a decoded
path that still does not match a differently-normalised filename is a
known limitation, not a bug to chase here.

> **This scope-out does not cover Unicode *case folding*** — see § 2a
> (b). Normalisation is how a character is encoded; case folding is
> `É` versus `é`. The second is squarely 075's subject and needs a
> decision, even though the first does not.

## 5. Acceptance

**075**
- [ ] Everything in § 1 — this gates the tranche
- [ ] `%20`, a non-ASCII filename and `+` each resolve
- [ ] Case behaviour identical across first, middle and last segments
- [ ] **`/SUB/users.json` and `/sub/USERS.json` both resolve, identically
      on Linux, macOS and Windows** — the cross-platform assertion is the
      finding
- [ ] `/api` prefix matches `/api` and `/api/x`, **not** `/apixyz`
- [ ] Existing rule-set scoping tests unchanged
- [ ] The § 2 decision documented
- [ ] **§ 2a reproduced on all three platforms, and the non-ASCII
      case-folding decision made, stated and pinned by a test** — the
      test detecting the running filesystem's actual case sensitivity
      rather than assuming it from `cfg!(target_os)`, the way tranche 3's
      `bare_differently_cased_file_vs_extension_match_...` does
- [ ] The fast path and the listing cannot disagree — whichever folding
      rule you choose, both reach it

**076**
- [ ] A `.json` file with non-alphabetical keys and pretty formatting is
      served **byte-identical** — assert bytes
- [ ] An invalid `.json` still fails at **load** (RFC 065 unchanged)
- [ ] Inline `respond.json` preserves key order
- [ ] The § 3 envelope decision made, stated, and pinned by a test
- [ ] `serve-json-files-from-a-folder`'s tests pass, or their
      expectations change **because the bytes are now correct** — say
      which

**Both**
- [ ] Gates green; **API baseline diff empty or declared**
- [ ] CI green on all 12 jobs before merge

## 6. Report back

`.git-exclude/review-request/audit-t4-fidelity/`, including the § 1
traversal evidence **quoted in full**, the § 2 cross-platform case results from all three CI platforms, and
the § 3 envelope decision.
