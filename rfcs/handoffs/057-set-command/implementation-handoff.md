# Implementation Handoff — RFC 057, `apimock set`

**Governing RFC.** [RFC 057](../../accepted/057-set-command.md)
**Contract.** [RFC 053](../../accepted/053-v6-cli-contract.md) — **restated in full in § 3**, so this package is self-contained
**Write path.** [RFC 056](../../accepted/056-toml-edit-migration.md) — already merged
**Umbrella.** [RFC 048](../../accepted/048-v6-cli-interface-concept.md) § 11 item 5 — the last of the portfolio
**Companion doc.** [`acceptance-qa-checklist.md`](./acceptance-qa-checklist.md)
**Milestone.** 6.0.0

---

## 1. Read this first — three things the RFC could not know

RFC 057 was written from the design, and the design is sound. Reading
the source to settle its Unresolved 3 turned up three facts that change
what you build. **None of them invalidate the RFC; all of them would
have cost you a rewrite if found during implementation.**

### 1.1 `--dry-run` cannot emit `diff_summary` as it stands

RFC 057 § Preview says `SaveResult` "already carries `changed_files`,
`diff_summary` and `requires_reload`", and proposes returning that as
the preview. But:

```rust
// crates/apimock-config/src/view.rs:538
pub struct DiffItem {
    pub kind: DiffKind,
    pub target: NodeId,   // <-- and NodeId is #[serde(transparent)] over Uuid
    pub summary: String,
}
```

Serialising a `DiffItem` puts a **bare per-process UUID** in `set`'s JSON
output. That is precisely the thing RFC 057 exists to keep out of the
contract — *"`NodeId` must never appear in `set`'s contract"* — hiding
inside the RFC's own preview design.

**Do not serialise `DiffItem` directly.** `set` renders its own preview
row: resolve each `target` back through the id index to an address, and
emit that address plus `kind` and `summary`. The UUID never leaves the
process.

This is the single highest-risk item in the RFC, because the wrong
version works perfectly in local testing — a UUID is only meaningless
to the *next* process, not to the one that printed it.

### 1.2 `get`'s `matched` block has no rule-set path, so the pair does not compose yet

RFC 057's § Addressing rests on `get --why` and `set` sharing an
address. Today they cannot:

```rust
// crates/apimock/src/cmd/get.rs:493 — the "which rule answered" block
serde_json::json!({ "rule_set_index": rule_set_index, "rule_index": rule_index })
```

Indices only. The per-rule-set *explanation* block does carry the path
(`get.rs:439`, `rule_set_file`), but the primary answer does not.

So `get` must gain `rule_set_file` in its `matched` object as part of
this work. **This is an additive change to an accepted-but-unreleased
command** — RFC 055 is merged to `main` and has never shipped, so
changing its output now costs nothing and after 6.0.0 costs a breaking
change. Do it here.

### 1.3 The two output formats disagree on base

JSON is 0-based (`get.rs:493`). The text renderer prints `rule_set_index
+ 1` and `rule_index + 1` (`get.rs:571-577`). A person reading text sees
"Rule #3"; a parser reading JSON sees `2`.

`set --rule` therefore needs an explicit, documented base. **Decision:
`set` takes 0-based indices, matching the JSON contract**, because the
machine consumer is the one addressing rules programmatically. Say so in
`--help` and in the docs, in those words. Leave the text renderer's
1-based display alone — it is for humans and changing it helps nobody —
but make sure the docs note the difference rather than letting a reader
discover it.

## 2. The three Unresolved questions, decided

### Unresolved 1 — how much of `EditCommand` does the first cut expose?

**Decided: exactly what W7 needs.** `AddRuleSet`, `AddRule`,
`UpdateRule` / `UpdateRespond`, `AddHeaderCondition`. Nothing else.

The RFC recommended this as conservatism. **It is more than that, and
the reason should be in your head while you build.** `AddRule` appends:

```rust
// crates/apimock-config/src/workspace/edit.rs:288
let new_rule_idx = rule_set.rules.len();
rule_set.rules.push(new_rule);
```

`AddRuleSet` likewise (`edit.rs:151`). So every command in this cut
**preserves existing indices**, which is what makes a positional address
survive from one invocation to the next.

The excluded commands are exactly the ones that would break that:
`DeleteRule`, `MoveRule`, `RemoveRuleSet` all renumber. The scope
boundary and the addressing contract are the same boundary. If you find
yourself wanting to add `DeleteRule` "while you're in there", that is
the moment to stop and escalate — it changes the contract, not just the
surface.

### Unresolved 2 — one change per invocation, or a batch?

**Decided: one change per invocation.** W7 is written as separate
invocations and reads perfectly well that way. A batch multiplies the
failure modes for a benefit — process startup — that is not the
bottleneck for any of U2, U3 or U4.

RFC 053 § 6 reserved room for a transaction boundary and it stays
reserved. Note that RFC 056 already checks the whole write set before
writing anything, so the all-or-nothing property a batch would need is
*already there* when we want it. We are deferring the surface, not
foreclosing it.

### Unresolved 3 — does `NodeAddress` become public?

**Decided: no.** Source settles it:

```rust
// crates/apimock-config/src/workspace/id_index.rs:25
pub(crate) enum NodeAddress {
    Root,
    RuleSet { rule_set: usize },
    Rule { rule_set: usize, rule: usize },
    ...
}
```

Three reasons to keep it private:

1. **It is positional, not natural.** `rule_set: usize` is an index into
   `service.rule_sets`. Making it public freezes index-based addressing
   into a stable contract — the weaker half of what `set` needs.
2. **It carries variants `set` does not expose** — `BodyCondition`,
   `FallbackRespondDir`, `Middleware`. A public enum drags all of them
   onto the contract, and `Middleware` is explicitly out of scope by T2.
3. It is `pub(crate)` today with no external caller. Publishing an
   internal shape onto a surface that must stay stable is the trade RFC
   052 just spent a breaking change to get out of.

**Build `set`'s own address instead**, from the two natural keys the
config already uses:

- **rule set** — by **file path**, as written in `service.rule_sets`.
  This is already how `AddRuleSet` addresses (`EditCommand::AddRuleSet
  { path: String }`, `view.rs:200`) and how `get` reports it
  (`rule_set_file`). A path is a real natural key: a person can read it,
  write it down, and use it tomorrow.
- **rule within that set** — by **0-based index**, stable under this
  cut's append-only commands (§ Unresolved 1).

Resolve that pair to a `NodeId` internally. What `apimock-config` needs
to expose is a **resolution function and an address renderer**, not the
enum: enough to turn (path, index) into a node and back, and no more.
Keep the returned type `#[non_exhaustive]` per RFC 052.

## 3. The contract, restated — so this package stands alone

RFC 053 is the authority and lives at
`rfcs/accepted/053-v6-cli-contract.md`. Everything you need to build
`set` is copied here, because this handoff is given to you on its own.
**If the two ever disagree, RFC 053 wins — say so rather than following
this section.**

### The envelope

```json
{ "schema": 1, "apimock": "6.0.0", "result": { … } }
{ "schema": 1, "apimock": "6.0.0", "error": { "kind": "…", "message": "…", "detail": { … } } }
```

- An **object, never a bare array**. A collection goes *inside* `result`.
- `schema` is an integer, so a consumer can branch on it.
- `apimock` is the running binary's version.
- **Exactly one** of `result` / `error`.

`crates/apimock/src/cmd/envelope.rs` already exists and already produces
this — RFC 054 built it and `get` reuses it. **Use it; do not write a
second one.**

### `error.kind` — a closed set

| `kind` | Meaning | Exit |
|---|---|---|
| `usage` | Bad invocation — unknown option, missing value | 2 |
| `config_invalid` | Configuration read but not valid | 1 |
| `config_unreadable` | Configuration missing or unreadable | 1 |
| `io` | Filesystem failure that is not the config | 1 |
| `conflict` | State changed underneath — **`set` only** | 1 |
| `internal` | A bug in apimock | 1 |

`set` is the command `conflict` was reserved for. Map
`SaveError::Conflict` to `conflict` and `SaveError::Read` to `io` — the
library distinguishes them precisely so the CLI can, and collapsing them
throws away the distinction a caller needs to decide whether retrying
helps.

### Exit codes

`0` success, `2` usage error, `1` everything else. **`error.kind` is not
encoded into the exit code** — the taxonomy lives in the payload, where
it can evolve; the exit code only says *whether* it failed.

### Output streams

Diagnostics to stderr; stdout carries only the result. A parser reading
stdout must never see a warning.

## 4. Scope

**In:** a new `crates/apimock/src/cmd/set.rs`; the minimal
`apimock-config` accessors from § 2 Unresolved 3; `rule_set_file` added
to `get`'s `matched` block (§ 1.2); the W7 CI script; documentation.

**Out:**
- `service.middlewares` — T2. Not added, changed or removed. **Existing
  entries pass through untouched**, and that is a test, not an
  assumption.
- `DeleteRule`, `MoveRule`, `RemoveRuleSet`, root settings, body
  conditions.
- Batching (§ Unresolved 2), interactive editing, talking to a running
  server.
- Re-deriving all-or-nothing write semantics. RFC 056 has it. Use it.

## 5. Evidence required

- **The W7 script from RFC 057 runs green in CI**, every exit code
  asserted. If it is awkward to write, say so — the RFC states that as
  the design's own falsification test, and we would rather hear it.
- **Round trip:** `set` a rule, `get` it back, receive it.
- **A config with comments survives a `set`** — show the file before and
  after. RFC 056 guarantees it; `set` is the surface that promises it.
- **`--dry-run` writes nothing**, and its reported changes match what a
  real run then produces.
- **No UUID anywhere in `set`'s output**, for any invocation including
  `--dry-run` and every error path. Grep the JSON for a UUID pattern and
  assert zero matches — that is a cheap regression test for § 1.1 and
  worth keeping.
- **Conflict:** modify a file after load, run `set`, receive `conflict`
  (not `io`), and confirm **no file was modified**.
- **`service.middlewares` untouched**, including when entries exist.
- **An address from `get --why` feeds `set` unmodified.** This is § 1.2's
  whole point; test the composition, not just the two commands.
- Full suite green, with the count against `main`'s baseline. Gates:
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`.

## 6. Escalation

Per project convention, blocking issues and design questions go in a
`.git-exclude/review-request/` package rather than only in chat.

Escalate specifically if: the (path, index) address proves insufficient
for anything W7 needs; exposing the resolution function pulls more of
`NodeAddress` onto the public surface than § 2 describes; or the W7
script is awkward to write. The last one is a finding about the design,
not a failure of the implementation, and it is wanted.
