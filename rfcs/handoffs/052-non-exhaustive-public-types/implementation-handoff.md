# Implementation Handoff — RFC 052, `#[non_exhaustive]` on public types

**Governing RFC.** [RFC 052](../../accepted/052-non-exhaustive-public-types.md)
**Milestone.** 6.0.0. **A breaking change, deliberately.**
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

Mark five public structs `#[non_exhaustive]`, in one deliberate break, so
that adding a field stops being a breaking change.

Three RFCs this month added fields to these types and every one was a
breaking change nobody noticed — RFC 040's three fields on `TraceConfig`
are on `main` unreleased right now. This is the change that stops that
recurring.

## 2. Establish this first — and I have started it for you

`#[non_exhaustive]` blocks struct-literal construction **from other
crates**, not from the defining crate. So the question that decides the
work is: *which of the five are constructed across a crate boundary?*

I counted construction sites by crate. **Verify this rather than
inherit it** — but it is where I would start:

| Type | Defined in | Constructed from | Breaks our own workspace? |
|---|---|---|---|
| `RequestSummary` | `apimock-server` | `apimock-server` only | **No** |
| `TraceConfig` | `apimock-server` | `apimock-server` only | **No** |
| `LogConfig` | `apimock-config` | `apimock-config` only | **No** |
| `VerboseConfig` | `apimock-config` | `apimock-config`, **`apimock-server`** | **Yes — 1 site** |
| `ParsedRequest` | `apimock-routing` | `apimock-routing`, **`apimock-server`**, **`apimock`** | **Yes — 8 sites** |

So three of the five are free, and two need work:

- **`VerboseConfig`** — one site, a `const` in
  `crates/apimock-server/src/parsed_request.rs` (test support).
- **`ParsedRequest`** — the real work. Sites in `apimock-server`
  (`parsed_request_from` and tests) and in `apimock`
  (`cmd/match_test.rs`, `benches/routing.rs`).

**This is not only a downstream concern.** Marking `ParsedRequest`
`#[non_exhaustive]` stops *our own* code compiling until it has a
constructor. That is the bulk of this RFC.

## 3. What to build

**For the three free types:** add the attribute. Nothing else.

**For `ParsedRequest`:** it needs a constructor that every cross-crate
site can use. Shape is yours — the constraint is that
`parsed_request_from` builds it with real values while the test and
bench sites build it with mostly defaults, so a constructor taking every
field positionally will read badly at the latter.

Consider what the sites actually need before designing it. Do not add a
builder if a plain constructor serves; RFC 052 says explicitly not to
add API nobody needs.

**For `VerboseConfig`:** one `const` site. A constructor, or `Default`
plus a setter, or moving the constant into `apimock-config` — whichever
is smallest. Say which you chose and why.

## 4. G2 is unanswered, and here is how to proceed anyway

RFC 052's Unresolved 1 asks whether the **GUI** constructs any of these.
It is open, in `.git-exclude/tasks/owner/001-gui-integration-questions.md`.

**Do not wait for it, and do not guess it.** Build what our own
workspace needs — that is knowable from § 2 and is the floor. If the
answer later says the GUI constructs `TraceConfig`, that is an
*additive* constructor on top, not a redesign.

But **do report which types you gave constructors to and which you
didn't**, so that answer can be checked against the work when it comes.

## 5. Scope boundaries

- **In:** the five types, whatever constructors the cross-crate sites
  need, and the call sites themselves.
- **Out:** the error enums. `ConfigError` and friends are *not*
  `#[non_exhaustive]` either, and RFC 052 Unresolved 2 asks whether they
  want the same treatment. Larger blast radius, and it is RFC 041's
  question. Do not fold it in.
- **Out:** changing any field, default, or behaviour. This is an
  attribute and the constructors it forces, nothing more.
- If a constructor starts growing logic — validation, defaulting beyond
  `Default` — stop and escalate. It should be a way to build a value,
  not a place decisions happen.

## 6. Evidence required

- The workspace builds and the full suite passes. `#[non_exhaustive]`
  constrains *other* crates, so most of the proof is that our own
  cross-crate sites still compile.
- **Every site from § 2's table accounted for** — enumerate them and say
  what each now does.
- A test exercises each new constructor, so an unused one is visible as
  unused rather than shipped on faith.
- **Prove the attribute does what it is for**: show that a struct
  literal for one of the five now fails to compile from another crate.
  A `compile_fail` doctest is the natural place. Without this, nothing
  demonstrates the change achieved anything.
- Full suite green; report the count against the **450** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. This lands in the migration guide

`docs/src/guides/migrating-to-6-0.md` already names this change. Check
what it says is still accurate once you know the final shape — in
particular whether it should name the constructors a downstream caller
should now use. A migration guide that says "this breaks" without saying
"use this instead" is half a guide.

## 8. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including a § 2 count that
disagrees with mine, which is worth knowing either way.
