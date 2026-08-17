# Questions for the GUI team

**Raised.** 2026-08-17, by the architect.
**Why this file exists.** These questions accumulated across seven RFCs
over two weeks, were referred to repeatedly as "the GUI round-trip", and
were never written down anywhere. That made them easy to defer and
impossible to act on. This is the list.

**How to use it.** Every question is answerable by someone reading the
GUI application's source. None needs a meeting. Answers go in the
**Answer** column, dated, and the RFC named in **Decides** is then
unblocked.

**Status.** Q6 and Q7 answered 2026-08-17. Q1–Q5 open.

---

## The GUI is the one consumer we cannot see

`apimock-config` and `apimock-server` are published libraries, and the
GUI application is the only consumer we know by name. Everything below
is a question about **how it actually uses our API**, not about what it
should do.

Several of these decide whether a piece of work is large or small — and
**Q1 could remove work rather than add it**, which is why it is first.

---

## Open

### Q1 — What does the GUI do today when config files change underneath it?

Reload wholesale? Ignore until the user acts? Prompt?

**Decides.** [RFC 042](../ROADMAP.md) — `sync_from_disk` incremental
reconciliation, still unwritten and blocked on this.

**Why it matters.** RFC 042 exists to make reconciliation *incremental*.
If the GUI is content to reload the whole workspace, the RFC may not be
needed at all — and it is currently one of the larger unwritten pieces.
This is the question most likely to **delete** work.

It also grows more important in v6: the CLI's `set` family will write the
same files a GUI session has open, so this stops being a GUI detail and
becomes a shared concern.

### Q2 — Does the GUI *construct* any of these types, or only read them?

```
TraceConfig      RequestSummary     (apimock-server)
ParsedRequest    (apimock-routing)
LogConfig        VerboseConfig      (apimock-config)
```

**Decides.** [RFC 052](./proposed/052-non-exhaustive-public-types.md).

**Why it matters.** `#[non_exhaustive]` forbids struct-literal
construction from outside the crate. For a type the GUI only *reads*,
the attribute costs nothing. For one it builds, we owe a constructor or
builder — and RFC 052 says explicitly not to add builders nobody needs.
The answer decides how much API we write.

### Q3 — Does the GUI match on *variants* of these error enums?

```
ConfigError    WorkspaceError    SaveError    ApplyError
```

Or does it only format them for display?

**Decides.** [RFC 041](../ROADMAP.md) — boxing the large error variants,
deferred to 6.0.0 because it is breaking.

**Why it matters.** Boxing changes each variant's shape. If the GUI only
calls `.to_string()` or `Display`, the practical break is near zero and
041 is cheap. If it matches on variants, it breaks at compile time and
needs coordinating.

### Q4 — Does anything in the GUI use `apimock validate --json`?

**Decides.** Whether `--json`'s removal in 6.0.0 hurts a known consumer.

**Context.** v5.19.0 deprecated it and ships `--format json` alongside,
emitting the new envelope. If the GUI uses `--json`, it has a migration
path available *today* and can move before 6.0.0 removes the old flag.

### Q5 — Would the GUI eventually consume v6's CLI contract, or keep the library API?

**Decides.** [RFC 048](./proposed/048-v6-cli-interface-concept.md) § 12,
and whether we maintain one interface or two.

**Why it matters.** Not urgent, but it shapes v6's direction. If the GUI
would eventually shell out to the CLI, the library API becomes an
internal detail over time. If not, both surfaces are permanent and both
need supporting.

---

## Answered

### Q6 — Does the GUI display trace-event headers? ✅ **Yes** (2026-08-17)

**Consequence.** [RFC 040](./proposed/040-trace-capture-and-redaction.md)
redacts credential headers, so GUI users now see `[redacted]` where a
credential value was. **No GUI code change needed** — the event's shape
is unchanged, only values differ.

**But the escape hatch is unreachable from the GUI.** `header_denylist`
lives on `TraceConfig`, which has no config-file surface, so a GUI user
cannot opt a header back in even deliberately. Giving `TraceConfig` a
configuration surface is its own piece of work, not yet scheduled.

### Q7 — Would the GUI want request body metadata? ✅ **Yes** (2026-08-17)

**Consequence.** [RFC 050](./proposed/050-non-json-body-capture-decision.md)
reports body presence and byte length — never content. Trace events now
distinguish *no body* from *body present but not captured*, which they
could not before.

---

## If a question turns out to be the wrong question

Say so. Q6 and Q7 were both answered "yes, unless it costs too much",
and checking the cost changed the design — Q7's turned out to be roughly
half what the RFC first estimated, because `content-type` was already
carried in the event and the byte length was already computed.

An answer of *"we don't use that at all"* is the most valuable one
available here. It deletes work.
