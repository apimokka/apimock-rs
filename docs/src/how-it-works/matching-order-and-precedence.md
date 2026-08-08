# Matching order and precedence

Every request is decided by the same four-stage sequence, and the first
stage to produce a response wins — nothing after it runs. This page
traces that sequence directly from `crates/apimock-server/src/server.rs`
so you can predict what a given request will do before you send it.

## The sequence

```
OPTIONS?  ──yes──▶  CORS preflight response (204), nothing else runs
   │ no
   ▼
Middleware  ──answered──▶  that response, nothing else runs
   │ unanswered
   ▼
Rule sets   ──matched───▶  that response, nothing else runs
   │ unmatched
   ▼
Fallback file tree  ──found──▶  the file
   │ not found
   ▼
404
```

The service entry point's own doc comment states this order in one
line (`server.rs:286-289`): *"OPTIONS → middleware → rule sets →
dyn_route (fallback)."* The implementation, `service()`
(`server.rs:290-358`), does exactly that:

1. **OPTIONS is checked before anything else** (`server.rs:296-298`) —
   before the request is even parsed. Any `OPTIONS` request gets a
   CORS-preflight response and nothing downstream ever runs. See
   [Response headers](../reference/response-headers.md) for what that
   response contains.
2. **Middleware** (`server.rs:322-324`, dispatching to
   `middleware_response`, `server.rs:361-379`): every loaded middleware
   script is offered the request, **in the order it's listed** in
   `service.middlewares`. The first one that returns a value answers
   the request; the rest are never called (`server.rs:365-377`, the
   loop returns on the first `Some`). If none of them answer it, the
   request falls through.
3. **Rule sets** (`server.rs:326-350`, dispatching to
   `rule_set_response`, `server.rs:382-397`): every configured rule set
   is checked, **in the order it's listed** in `service.rule_sets`. The
   first rule set with a matching rule answers the request
   (`server.rs:386-394`, the loop returns on the first `Some`) — *which*
   rule within that set answers it is decided by that rule set's
   strategy (see below). Rule sets after the first match are never
   consulted.
4. **Fallback file tree** (`server.rs:352-357`, `dyn_route_content`):
   reached only if nothing above answered. Serves a file directly from
   `service.fallback_respond_dir`, resolved by URL path. If no matching
   file exists, the response is a 404. Zero-config mode — no rule sets,
   no middleware — is just this one stage.

**Nothing here is configurable.** There is no setting that changes the
relative order of these four stages; every rule set and every
middleware in a config it participates in is layered underneath this
same sequence.

## What wins when more than one thing could match

**Across rule sets:** the first rule set in `service.rule_sets` that
has *any* matching rule wins outright — completely, not partially.
Even if a later rule set would have matched more specifically, it is
never consulted once an earlier one matches.

**Within one rule set:** its `strategy` decides which of its own
matching rules answers, independent of the other rule sets entirely.
Five strategies exist:

| `strategy` | Behaviour |
|---|---|
| `first_match` (default) | The first rule in file order that matches |
| `priority` | Among matches, the one with the highest `priority`; ties broken by its `tiebreaker` (`first_match` or `uniform_random`) |
| `weighted_random` | Random among matches, weighted by each rule's `weight` (default `1`) |
| `uniform_random` | Random among matches, unweighted |
| `round_robin` | Cycles through matches, one per request |

`service.strategy` sets the workspace default; a rule set's own
top-level `strategy` field **overrides** it for that rule set only. See
[Vary the response for one path](../guides/vary-the-response-for-one-path.md)
for worked examples of all five, and
[`apimock.toml` root settings](../reference/apimock-toml-root-settings.md)
for the exact syntax.

## `prefix`, `guard`, and per-rule-set `strategy`

- **`prefix.url_path`** strips a leading segment from the request path
  before that rule set's rules are matched against it — it changes
  *what a rule sees*, not the order rule sets are tried in.
- **Per-rule-set `strategy`** changes *which rule within that set*
  answers, as above — it has no effect on whether that rule set is
  reached in the first place.
- **`[guard]` does nothing.** It's a zero-field struct
  (`crates/apimock-routing/src/rule_set/guard.rs`) carrying only a
  `// todo:` comment for a condition that was never implemented. A
  `[guard]` table in a rule set has no effect on matching, on ordering,
  or on anything else — do not configure it expecting it to gate a rule
  set. Its future is an open decision, not something this documentation
  can describe as working.

## One more thing that looks like it should affect this page, but doesn't

`[default].delay_response_milliseconds` — a rule set's own top-level
`[default]` table — is parsed and shown in the startup log, but is
**never applied to any response**. It has no effect on timing, on
matching, or on anything else a request experiences. The per-rule
`respond.delay_response_milliseconds` field works correctly; the
rule-set-wide one does not. See
[Simulate slow or flaky backends](../guides/simulate-slow-or-flaky-backends.md)
for the field that actually works.
