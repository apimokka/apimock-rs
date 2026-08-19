# Implementation Handoff — RFC 055, `apimock get`

**Governing RFC.** [RFC 055](../../proposed/055-get-command.md)
**Contract.** [RFC 053](../../proposed/053-v6-cli-contract.md)
**Milestone.** 6.0.0 — RFC 048 § 11 item 4
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)

---

## 1. What this is

`apimock get /users/1` answers *what would the server return for this
request* — body, status, headers — from configuration on disk, no server
running. `--why` explains which rule decided it.

**The value is that the answer is true.** A `get` that disagrees with the
running server is worse than no `get`, because U2 believes it.

## 2. All three of the RFC's open questions are decided

The owner accepted the RFC without overturning these; I am recording them
as decisions with reasoning, so you can implement and they can be
challenged.

### Q1 — Does `get` execute middleware? → **NO. Detect and disclose.**

Middleware is the **first** dispatch stage, so skipping it silently would
give an answer from a stage the server would never have reached. That is
wrong, not merely incomplete.

But executing it means **running Rhai scripts as a side effect of a read
command** — configuration a user wrote, possibly configuration an agent
generated moments earlier. This project's standing principle is to prefer
the safer option where the two conflict, and reading should not execute.

So take the RFC's third option:

- **No middleware configured** — the common case. `get` is exactly right,
  and says nothing special.
- **Middleware configured** — run the remaining stages, and **report in
  the response that middleware exists and was not simulated**, in both
  text and JSON. The answer is then honestly incomplete rather than
  quietly wrong.

Do **not** add a `--run-middleware` flag. If it is wanted it is its own
decision, with its own security reasoning, not a convenience bolted on
here.

### Q2 — `OPTIONS`? → **Handle it, like any other stage.**

It is the first branch in `service` and it is trivial. Return what the
server returns. It is listed only because "trivial" is how the file-tree
case would have been missed too.

### Q3 — Does `--why` default on? → **Off in text, on in JSON.**

RFC 048 § 1 already settled the general rule: where U1 and U2 conflict,
**U2 decides the machine-readable surface and U1 decides the default
human one.** This is that rule applied.

A person gets a clean answer and asks for `--why` when they want it. An
agent gets the explanation without having to know to ask — and the
near-miss detail is what lets it correct itself, so making it opt-in
would mean most agents never see it.

## 3. The trap this RFC exists to avoid

`service` dispatches: **OPTIONS → middleware → rule sets → dyn_route**.

**Zero-config mode is served entirely by `dyn_route`**, the last stage —
that is the README's opening promise, "drop JSON files into a folder and
your API immediately exists". A `get` that consults only rule sets
reports nothing matched while the server returns the file.

Cover every stage, in order. If you find a stage you cannot cover, that
is an escalation, not a footnote.

## 4. One implementation of matching, not two

Call `rule_set_response` and `dyn_route_content` — the same functions
`service` calls — on a `ParsedRequest` built the way `parsed_request_from`
builds one. **Do not reimplement matching.** A second implementation
drifts, and the drift is invisible until someone's mock behaves
differently from their `get`.

What that needs from `apimock-server` — reachability, and collecting a
response body rather than streaming it — **establish from source**. It may
be free; it may need a small additive surface. If it needs more than
that, escalate rather than widening.

## 5. Scope boundaries

- **In:** a new command under `crates/apimock/src/cmd/`, and whatever
  minimal surface `apimock-server` must expose.
- **Out:** `set`; talking to a running server; replacing or changing
  `match-test`; executing middleware (Q1).
- **`match-test` is not touched.** It keeps exiting **1** on no-match
  while `get` returns **0** with a result saying so, per RFC 053 — the
  two differ deliberately and both document it. Do not "fix" the
  inconsistency; aligning `match-test` breaks a documented exit code
  that 5.19.0's deprecation window never warned about.

## 6. Evidence required

- W1 and W2: correct body, status, headers for a rule-set config.
- **Zero-config: `get` on a file-tree path returns the file.** The case
  a rules-only implementation gets wrong.
- `--why` names the deciding rule; for a near-miss, names the **failing
  condition** — that specific output is the feature.
- **Middleware configured → the response says so** and the answer is
  marked incomplete.
- **`get` agrees with a running server.** Start one on the same config,
  issue the same requests across every dispatch stage, compare. This is
  the test that matters; everything else is a proxy for it.
- `--format json` emits a valid RFC 053 envelope with provenance.
- Full suite green; report the count against the **455** baseline.
- Gates: `cargo fmt --all --check`, `cargo clippy --workspace
  --all-targets --all-features -- -D warnings`.

## 7. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package — including a dispatch stage you
cannot cover, and any disagreement with § 2's decisions, which are mine
rather than the owner's and are meant to be challengeable.
