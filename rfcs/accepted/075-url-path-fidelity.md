# RFC 075 — URL path fidelity: decoding, case, and prefix boundaries

**Status.** Accepted — owner approved 2026-09-01.
**Tracks.** Correctness. External audit 2026-09-01, F-03, F-05, F-02.
**Touches.** `crates/apimock-routing/src/util/http.rs`,
`crates/apimock-server/src/json_path_util.rs`, prefix matching in
`rule_set.rs`, `docs/src/reference/`.

## Summary

Three defects in how a URL becomes a file or a rule match:

1. **Paths are never percent-decoded.** A fixture whose name contains a
   space or a non-ASCII character is permanently unreachable.
2. **Case-insensitive matching applies only to the final path segment**,
   not to parent directories.
3. **URL-prefix scoping leaks across segment boundaries** — a prefix of
   `/api` also matches `/apixyz`.

All three present to a user as an inexplicable 404, and none is
documented.

## Motivation

**F-03** is the one every reference implementation gets right —
json-server, WireMock and Prism all percent-decode. A user creating
`my file.json` and requesting `/my%20file.json` gets a 404 with nothing
in the docs explaining why. The zero-config promise is "the JSON you put
on disk is what a client gets back"; this is a class of filename where
that is false.

**F-05** means `/API/users` may resolve while `/api/USERS` does not, or
vice versa, depending on which segment differs — behaviour no user would
predict and no page describes.

**F-02** is a matching-correctness bug: a rule set scoped to `/api`
claiming requests it was not meant to own. Unlike the other two it fails
*permissively*, which makes it the most dangerous of the three even
though it is the least visible.

## Goals

1. Percent-encoded paths resolve to the file they name.
2. Case-insensitivity is consistent across the whole path, whatever it
   is.
3. Prefix scoping respects segment boundaries.
4. All three behaviours documented — including whatever is deliberately
   left as-is.

## Non-goals

- Unicode normalisation (NFC/NFD). A filesystem-dependent rabbit hole;
  out of scope, and worth saying so in the docs.
- Changing the operator set for `url_path` conditions.

## Design

**Order matters and is the security-critical part of this RFC.**

> **Percent-decode *before* dot-segment normalisation, and normalise
> after.** Decoding after normalising would let `%2e%2e` survive
> normalisation and become `..` afterwards — reintroducing
> GHSA-72g6-wgrg-vhm7 by a different route.
>
> The audit notes today that `%2e%2e` does **not** traverse precisely
> because decoding never happens. Adding decoding without reordering
> turns a missing feature into a security regression.
>
> The confinement added by RFC 063 is the backstop and must stay; it is
> not a substitute for getting the order right.

**F-05** — apply the same case rule to every segment. Which rule is the
open question below.

**F-02** — compare prefixes segment-wise: `/api` matches `/api` and
`/api/x`, not `/apixyz`.

## Testing and verification

- `%20`, a non-ASCII filename, and `+` in a path each resolve.
- **`%2e%2e%2f` and `..%2f` do not traverse** — assert against the
  confinement, and assert the *response*, not just the resolved path.
- The existing traversal tests still pass unchanged.
- Case behaviour is identical for the first, middle and last segments.
- `/api` prefix: matches `/api`, `/api/x`; does **not** match `/apixyz`
  or `/apix`.
- Existing rule-set scoping tests unchanged.

## Risks

| Risk | Mitigation |
|---|---|
| **Decoding reintroduces traversal** | The ordering rule above, plus RFC 063's confinement, plus explicit encoded-traversal tests. This is the risk that matters |
| Case change breaks an existing config | Possible; it is currently inconsistent, so some configs depend on the inconsistency. Release note, and pick the rule deliberately |
| Prefix fix un-matches something | Anything it un-matches was matched by accident |

## Unresolved questions

1. **Case-insensitive or case-sensitive, uniformly?** The current
   behaviour is neither. Case-sensitive is what a filesystem does on
   Linux and what a URL spec implies; case-insensitive is friendlier and
   is what the last segment does today. **Establish which is documented
   as intended** before implementing — this is a behaviour choice, not a
   bug fix, and the answer decides whether existing configs break.
