# RFC 069 — Reject configuration we do not understand

**Status.** Proposed — awaiting owner approval.
**Tracks.** Correctness / U2 safety. External audit 2026-09-01, F-17.
**Touches.** `crates/apimock-routing` (the rule-facing `Deserialize`
types), `crates/apimock-config`, `docs/src/guides/migrating-to-*`.

## Summary

A mistyped key in a rule is silently discarded. The rule still loads,
`apimock validate` reports success, and the rule then matches **more**
requests than the author wrote — because the condition they intended
simply is not there.

## Motivation

**Verified 2026-09-01.** A rule with a correct `url_path` and `headerz`
instead of `headers`:

```
$ apimock validate -c ./cfg.toml
Validation passed (1 rules across 1 rule set(s)).

$ curl http://127.0.0.1:PORT/secret          # no header at all
SENSITIVE
$ curl -H 'x-token: WRONG' …/secret
SENSITIVE
```

The header condition vanished. The rule answers everything.

**Why this is the highest-consequence config error class:**

- **The failure direction is wrong.** A typo that made a rule match
  *less* would be noticed immediately — the mock stops responding and
  the author investigates. A typo that makes it match *more* looks like
  success until something depends on the condition.
- **`validate` actively endorses it.** Our own tool, whose entire
  purpose is answering "will this serve correctly", says yes.
- **It targets the user v6 was built for.** RFC 048 designed the v6 CLI
  around U2 — an agent generating configuration. A generator emitting a
  near-miss key is not hypothetical; it is the expected failure mode of
  a machine author working from a schema it half-remembers.

This is the silent-wrongness class the whole v6 CLI programme existed to
remove, sitting one layer below where that programme was looking. RFC 064
made the *front door* refuse what it does not understand. The config
loader still accepts it.

## Goals

1. An unknown key in a rule, condition, or respond block fails to load,
   naming the key and where it is.
2. The message suggests the intended key where one is close — the same
   near-match courtesy RFC 059 gave unknown flags.
3. `validate` reports it as an error, not a pass.

## Non-goals

- Unknown keys in *root* config sections. Same argument applies, but the
  blast radius and compatibility story differ; keep this change to the
  rule-facing surface where the security consequence lives.
- Schema versioning or migration tooling.

## Design

`#[serde(deny_unknown_fields)]` on the rule-facing structs — `Rule`,
`When`, `Request`, `Respond`, the condition payloads.

Error text should name the key and the path to it, and use
`crate::args::near_match` — already used for flags — to suggest a
correction. `headerz` → `headers` is exactly the distance that machinery
handles well.

> **This is a breaking change, and the RFC is not pretending otherwise.**
> A configuration that loads today can stop loading. That is the entire
> point — but it means someone's working setup, with a stray key they
> never noticed, fails after upgrading.
>
> **Therefore: a minor release, with the break named in the release
> notes and the migration guide.** Not a patch. The 6.x additive-only
> promise is about the *API*, not config acceptance, so this is
> permissible — but it is user-visible and deserves the announcement a
> minor gets.

### Interaction with `deny_unknown_fields` and forward compatibility

`deny_unknown_fields` makes adding a new optional key a breaking change
for anyone on an older binary reading a newer config — they now get a
hard error instead of ignoring it.

That trade is worth taking here: silently ignoring an unknown key is
precisely the defect. But it should be a recorded consequence, not a
discovery, and it argues for the setting being on the rule-facing
structs only rather than everywhere.

## Testing and verification

- A mistyped condition key fails `validate` **and** fails to load,
  naming the key.
- The near-match suggestion fires for `headerz` → `headers`.
- **The reported scenario end to end**: after the fix, the `/secret`
  rule above must not serve to an unconditioned request — assert on the
  served response, not only on the loader.
- Every config under `examples/` and in the test corpus still loads.
  **If any does not, that is a finding, not a test to update** — it
  means we shipped an example with a key that does nothing.
- A valid config with every documented key still loads.

## Risks

| Risk | Mitigation |
|---|---|
| Breaks working configs | Deliberate; minor release, release note, migration guide entry |
| An example or fixture has a dead key | Then we have been shipping it. Report before fixing — it is the same bug in our own material |
| Forward compatibility narrows | Documented above as an accepted consequence, scoped to rule-facing structs |

## Unresolved questions

1. **Should root-level sections get the same treatment?** The argument
   is identical; the compatibility cost is higher because root config is
   where new settings land most often (RFC 067 and 068 each add one).
   **Recommend deferring** and revisiting once those settle — but
   deferring *with a recorded reason*, not by omission.
