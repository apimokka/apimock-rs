# 1. Crates and architecture

## The four crates, and which you depend on

| Crate | Owns | You depend on it? |
|---|---|---|
| **`apimock-routing`** | The rule model: `RuleSet`, `Rule`, `Respond`, conditions, match strategies. No I/O, no HTTP. | **Transitively**, and you will name its types |
| **`apimock-config`** | Reading `apimock.toml` and everything it references; path resolution; validation; **the `Workspace` façade you edit through** | **Yes — this is your main dependency** |
| **`apimock-server`** | HTTP response construction, TLS, middleware, the running server, the trace channel | **Yes** — if your GUI runs a mock (it presumably does) |
| **`apimock`** | The CLI binary and its argument parsing | **No.** Its library surface is CLI internals — `args`, `app`, `init_interactive`. Nothing you want |

Dependency direction, which is also the layering:

```
apimock-routing   (no internal deps)
      ↑
apimock-config    (depends on routing)
      ↑
apimock-server    (depends on config + routing)
      ↑
apimock           (the CLI; depends on all three)
```

**Do not depend on `apimock`.** It re-exports the other three
(`pub use apimock::config` etc.), but it also drags in the CLI. Depend
on what you need directly.

```toml
[dependencies]
apimock-config  = "6"
apimock-server  = "6"
apimock-routing = "6"   # only if you name its types explicitly
```

## Why the split exists, and why it matters to you

The 5.0 refactor moved HTTP-response construction *out* of the routing
crate specifically so routing could be a clean dependency target for a
future GUI. `Respond`'s own module documentation says so:

> *"the routing crate must stay free of hyper body / response helpers
> so that it can be a clean dependency target for a future GUI.
> `Respond` now just describes what the user wrote in their TOML; the
> server consumes that description and builds the actual HTTP
> response."*

**The practical consequence:** you can model, display and edit a
configuration using `apimock-routing` + `apimock-config` **without
linking a server at all**. If your GUI has a "design rules" mode that
does not need a live mock, it does not need `apimock-server`, and does
not pull in hyper, rustls or tokio's networking.

Whether that separation is worth exploiting is your call — but it was
built deliberately, and it is there.

## Where the authority lives for each question

When this package and the code disagree, these are the sources of
truth, in order:

| Question | Authority |
|---|---|
| What is in the public API | `crates/<name>/public-api.txt` — generated, CI-gated |
| What a config file may contain | `docs/src/reference/apimock-toml-root-settings.md`, `rule-set-schema.md` |
| What a rule may match on | `docs/src/reference/operator-reference.md` |
| What apimock protects against | `docs/src/reference/threat-model.md` |
| Why a design is the way it is | `rfcs/done/` — every accepted design, with its reasoning |

The RFCs are worth knowing about. They are not marketing documents;
they record what was rejected and why, which is usually the part you
need when a design looks odd.
