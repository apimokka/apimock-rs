# RFC 055 — `apimock get`: what will this request return?

**Status.** Implemented (v6.0.0). Accepted — approved by the project owner 2026-08-19.
Implemented and merged to `main`; awaiting the 6.0.0 release.

**Tracks.** v6. [RFC 048](./048-v6-cli-interface-concept.md) § 11 item 4 —
the read half of the CLI, and the first command built against
[RFC 053](./053-v6-cli-contract.md)'s contract.
**Touches.** `crates/apimock/src/cmd/` (a new command), and whatever
`apimock-server` must expose to answer without a listener.
**No change to the mock-serving hot path.**

## Summary

`apimock get /users/1` answers *what would the server return for this
request* — the body, status and headers — from configuration on disk,
with no server running. `--why` explains which rule decided it and, for
a near-miss, which condition failed.

## Motivation

### Most of this exists, in the wrong shape

`apimock match-test` (RFC 015) already takes a path, method, headers and
body, walks the rules, and prints a per-condition breakdown —
tick-or-cross for `url_path`, `method`, `body`, each header, per rule.
That is RFC 048's **W3** in human form, and it is the expensive part.

Three things are missing, and they are what this RFC adds.

**It tells you the rule, not the answer.** `match-test` prints
`Result: MATCH (rule #2)`. A user asking *"what will I get for
`/users/1`?"* wants the body, the status and the headers — the thing the
client would receive. That is a different question with a different
answer.

**It works on one rule-set file, not the configuration.** `--rule-set`
is required. `get` should answer from `apimock.toml` and everything it
pulls in, because that is what the server would do.

**It has no machine-readable output.** RFC 053's envelope now exists and
`validate` already emits it (RFC 054). `get` is the command that most
needs it — U2, the AI CLI agent, is the user this whole release is for.

### The trap: answering for only part of dispatch

`service` dispatches in four stages (`server.rs:324`):

```
OPTIONS → middleware → rule sets → dyn_route (file-tree fallback)
```

**A `get` that consults only rule sets would be wrong for the most
common case there is.** Zero-config mode — the thing the README opens
with, *"drop JSON files into a folder and your API immediately
exists"* — is served entirely by `dyn_route`, the last stage. A user
running `get /api/v1/hello` on a zero-config workspace would be told
nothing matched, while the running server returns their file.

That is worse than not shipping the command: it is a confidently wrong
answer, which is precisely the U2 failure mode RFC 048 § 1 is built
around.

**So `get` must cover the same stages the server does, in the same
order** — or state plainly which it does not.

Middleware is the hard one, and § Unresolved 1 puts it to the owner
rather than deciding it here: simulating it means **executing Rhai
scripts**, and this RFC will not decide to execute code as a side effect
of a read command.

## Goals

1. **W1/W2** — answer for a path, and for a full request (method,
   headers, body).
2. Answer with the **response**: status, headers, body.
3. **W3 via `--why`** — which rule set, which rule, which condition
   decided it; for a near-miss, which condition failed.
4. Cover the **same dispatch stages** as the server, or say which are
   excluded and why.
5. Emit RFC 053's envelope under `--format json`, including **provenance**
   — which configuration answered.
6. Reuse the matching engine. **No second implementation of matching.**

## Non-goals

- `set`. Read-only here.
- Talking to a running server. Static, per RFC 048 § 4 — with the drift
  caveat that provenance exists to expose.
- Replacing `match-test`. It keeps working; see § 4.
- Executing middleware, unless § Unresolved 1 says otherwise.

## Design

### Shape

```
apimock get <path> [-c <config>] [-m <METHOD>] [-H 'Name: value']... [-b <json>|--body-file <p>]
                   [--why] [--format text|json]
```

Flags reuse `match-test`'s spellings — `-m`, `-H`, `-b`, `--body-file` —
because a user who knows one should not have to learn the other.

### Reuse the engine, do not reimplement it

`rule_set_response` and `dyn_route_content` are what the server calls.
`get` calls the same functions, on a `ParsedRequest` it builds the same
way `parsed_request_from` does. **Matching must have exactly one
implementation** — a second one would drift, and a `get` that disagrees
with the server is worse than no `get`.

What that requires of `apimock-server` is a question for implementation:
those functions may need to be reachable, and the response body needs
collecting rather than streaming. Establish it from source rather than
assuming it is free.

### `--why`

`match-test`'s per-condition output, in both formats. In JSON it is
structured — rule set, rule index, and per condition a name, an
expectation, the actual value, and whether it held. RFCs 016 and 029
already produce this granularity internally.

**The near-miss case is the point.** *"Rule #3 matched the path but its
`x-api-key` header condition failed"* is what lets an agent fix its own
configuration. *"No match"* alone forces it to guess.

### Provenance

Per RFC 053 Layer 4, every response names the configuration it answered
from, as absolute paths. A static answer can disagree with a running
server started elsewhere, and the user must be able to see which one
they asked.

## 4. `match-test` stays, and one thing about it needs deciding

`match-test` is not replaced. It answers a narrower question against a
single rule-set file, and RFC 054 already decided 6.0.0 *adds*
`--format json` to it rather than reshaping its text output.

**But the two disagree on "no match", and that is a trap.**
`match-test` exits **1** when nothing matched. RFC 053 decided `get`
returns **exit 0** with a result saying nothing matched, because it is a
legitimate answer to a legitimate question.

Both defensible; together, confusing. Two commands answering nearly the
same question with opposite exit semantics is exactly what U3, a CI
pipeline, gets wrong.

**Recommendation: `get` follows RFC 053 (exit 0), `match-test` keeps
exiting 1, and both document the difference explicitly.** Aligning
`match-test` would be a breaking change to a documented exit code that
5.19.0's deprecation window did not warn about — and inventing a warning
for it after the window has closed is worse than documenting the
difference.

## Testing and verification

- W1 and W2 against a rule-set config: correct body, status, headers.
- **Zero-config: `get` on a file-tree path returns the file** — the case
  § Motivation says a rules-only implementation gets wrong.
- `--why` names the deciding rule; for a near-miss, names the failing
  condition.
- **`get` agrees with a running server**, on the same config, for a set
  of requests covering each dispatch stage. This is the test that matters
  — the whole value is that the answer is true.
- `--format json` emits a valid RFC 053 envelope with provenance.
- Full suite green; report the count against the **455** baseline.

## Risks

| Risk | Mitigation |
|---|---|
| `get` disagrees with the server | Same engine, and a test that compares against a live server rather than asserting from the code |
| Covering `dyn_route` pulls in filesystem behaviour | It must — that is where zero-config answers come from. The alternative is being wrong by default |
| Middleware makes `get` execute Rhai | Unresolved 1. Not decided here, and not by accident |
| `--why`'s JSON shape becomes a contract prematurely | RFC 053's `schema` covers it; additive change is the default path |

## Unresolved questions

1. **Does `get` simulate middleware?** Doing so means **executing Rhai
   scripts** during a read command — configuration the user wrote, but
   still code, and possibly code an agent just generated. Not simulating
   it means `get` is wrong for any request a middleware would have
   handled. **Owner's call**, and it is the same data-versus-code
   distinction as threat T2. A third option exists: run rule sets and
   `dyn_route`, and **report that middleware was skipped**, so the answer
   is incomplete rather than wrong.
2. **What does `get` do about `OPTIONS`?** The first dispatch stage, and
   trivial — but "trivial" is how the file-tree case would have been
   missed too.
3. **Does `--why` default on or off?** Off is quieter; on is more useful
   to U2, which never has to ask twice. Cheap either way, so it should be
   chosen rather than defaulted.
