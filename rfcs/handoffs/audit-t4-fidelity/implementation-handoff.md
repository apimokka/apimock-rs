# Handoff — Tranche 4: fidelity

**Governing RFCs.** [075](../../accepted/075-url-path-fidelity.md)
(URL path), [076](../../accepted/076-serve-responses-as-written.md)
(JSON bytes). Accepted 2026-09-01.
**Milestone.** Next minor.
**Baseline.** `main` @ `5d9e5bc`.

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
with `sub/users.json` on disk:

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

- Extend the comparison `dyn_route.rs:118-127` already performs for the
  final segment (`eq_ignore_ascii_case` over the directory listing) to
  every segment. **Do not construct a path and let the OS resolve it.**
- Case-sensitivity was never a free alternative: enforcing it would need
  an explicit post-resolution case check on every segment, because a
  case-insensitive filesystem opens `SUB/users.json` whatever apimock
  intended — and it would reverse the deliberate, documented
  accommodation at `dyn_route.rs:18-25`.
- **Test on all three platforms.** RFC 061's matrix is exactly what this
  finding needs; a Linux-only test passes with the defect intact on the
  other two.
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

**075 — Unicode normalisation is out of scope**, and the docs should
say so. NFC/NFD is filesystem-dependent and a rabbit hole; a decoded
path that still does not match a differently-normalised filename is a
known limitation, not a bug to chase here.

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
