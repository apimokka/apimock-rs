# RFC 085 — How a strategy is spelled, and a writer that cannot spell it

**Status.** **Implemented — shipped in 6.4.0** (2026-10-07). Owner approved
2026-10-07: § 2 plus Option B. Implemented at `b83b7b9`, review follow-ups
at `591eeca`.
**Tracks.** Configuration usability; a pre-existing library defect.
Found by the dev team in task 019 (their Q1); the documentation half
was found in the architect's review.
**Touches.** Depending on the option: `crates/apimock-routing/src/strategy.rs`,
`crates/apimock-config/src/toml_writer.rs`,
`crates/apimock-config/src/workspace/edit/root_setting.rs`,
`crates/apimock-config/src/view.rs`, `docs/src/reference/rule-set-schema.md`,
`docs/src/reference/apimock-toml-root-settings.md`, the strategy guide.
**Not a 6.3.0 blocker** (6.3.0 shipped without it). Library-only, and the failure is loud.

## 1. What is wrong today

A strategy appears at two levels: `[service] strategy` and a rule set's
top-level `strategy`. There are five strategies, and they come in two
shapes:

| strategy | takes options | loads as a bare string? |
|---|---|---|
| `first_match`, `round_robin` | no | **yes** |
| `uniform_random`, `weighted_random` | `seed` | **no** — *"invalid type: unit variant, expected struct variant"* |
| `priority` | `tiebreaker` | **no** — same error |

Three things in the code base disagreed with the loader:

1. **The schema reference** said `"uniform_random"` and
   `"weighted_random"` are bare strings. False since at least 5.18.0;
   corrected at `5bdc662`.
2. **The writer** (`crates/apimock-config/src/toml_writer.rs:124`, and
   the `[service]` equivalent near `:216`) serialises *every* strategy
   with `strategy.to_string()`, i.e. its bare name. For three of the
   five that writes a file that **does not load**. For any non-default
   `seed` or `tiebreaker` it would also **silently drop the option**.
   (Task 019 keeps an existing strategy table when the variant is
   unchanged, so `set` no longer breaks a hand-written table. But
   *newly setting* one is still broken.)
3. **The editor** (`root_setting.rs`, `UpdateRootSetting { ServiceStrategy }`)
   builds the *correct* model — `WeightedRandom { seed: None }` — and
   hands it to that writer. So a GUI choosing a strategy writes an
   unloadable file. Its API takes only a name, so a GUI cannot set a
   `seed` or `tiebreaker` at all.

And the loader's refusal speaks serde, not apimock: *"unit variant,
expected struct variant"* tells a user nothing they can act on.

## 2. Needed under every option: the writer must only write what loads

**The writer must emit a spelling the loader accepts, and must carry
every option it holds.** A tool that can write a file its own loader
refuses is the same defect class task 019 just closed, aimed at a
different key. A round-trip test — every strategy variant, with and
without options, written then loaded — pins it.

This is not optional and does not depend on § 3. What § 3 changes is
only *which* loadable spelling the writer picks.

## 3. The options — what users may write

### Option A — keep the syntax strict; make the refusal explain itself

Bare names stay refused for the three option-taking strategies. The
refusal says what to write instead, in the RFC 082 Amendment 1 / task
018 pattern:

```
strategy "weighted_random" takes options, so it is written as a table:
  strategy = { weighted_random = {} }            # defaults
  strategy = { weighted_random = { seed = 7 } }  # fixed seed
```

- **For:** no syntax change. Smallest. One spelling per strategy.
- **Against:** the user is still told *no* for something whose meaning is
  unambiguous. `{ weighted_random = {} }` — an empty table that means
  "the defaults" — remains the only way to ask for the defaults, and
  that is a shape a user can reasonably misread.

### Option B — accept the bare name as "that strategy, default options" *(chosen)*

`strategy = "weighted_random"` loads as `WeightedRandom { seed: None }`.
The table form keeps working, and is needed only to set an option. The
writer emits the bare name when the options are the defaults and the
table when they are not, so the shortest correct form is always what
comes out.

- **For:** the rule becomes one sentence: *"name the strategy; add a
  table only to set options"*. That is the string-or-table shorthand
  TOML users already know from Cargo (`serde = "1"` versus
  `serde = { version = "1", features = [...] }`). The old docs claim,
  the editor's output, and a user's natural first guess all become
  valid at once. Nothing is ambiguous: every name maps to exactly one
  variant.
- **Against:** a custom `Deserialize` for a public type
  (`apimock_routing::Strategy`), and it must keep good messages for
  typos. It widens accepted syntax, which is a one-way door: once a
  spelling is accepted, it is permanent.

### Option C — Option B, and the writer always canonicalises to one form

Like B, but every write normalises to a single spelling (for example,
always the table). A user who wrote `"weighted_random"` would see `set`
rewrite it as `{ weighted_random = {} }`.

- **For:** one spelling on disk.
- **Against:** it rewrites a line the user did not ask to change. That
  is exactly what task 019 just stopped `set` doing to other keys. Not
  recommended.

### Option D — redesign: every strategy is a name; options move out

`strategy = "weighted_random"` always, with options in sibling keys
(`strategy_seed = 7`, or a `[strategy_options]` table). The current
table form is deprecated with a warning (RFC 054 pattern) and later
removed.

- **For:** the most uniform model: a strategy is always a string.
- **Against:** a config migration with a deprecation cycle, every doc
  example rewritten, and two keys that must be kept consistent (a
  `strategy_seed` set beside `first_match` means nothing — a new way to
  write something meaningless). It costs a lot for a modest gain.

### Option E — fix the library API: a typed strategy setter

Independent of A–D, and combinable with any of them.
`UpdateRootSetting { ServiceStrategy }` takes a typed value (the
variant **with** its `seed` / `tiebreaker`) instead of a name string,
and `view.rs` exposes the options instead of `Option<String>`.

- **For:** a GUI can set a seed and a tiebreaker. An invalid strategy
  becomes unrepresentable at the API boundary, rather than refused at
  load.
- **Against:** an `apimock-config` public API change, and **no consumer
  needs it yet** — the GUI defers workspace file I/O by design. Building
  it now is building for a guess.

### Option F — writer fix only (§ 2), nothing else

Fix the writer and stop. The syntax stays as it is, and so does the serde
message.

- **For:** cheapest. Removes the actual defect.
- **Against:** leaves the cryptic refusal that already trapped this
  project's own documentation.

## 4. Recommendation

**§ 2, plus Option B.** Defer E until a GUI asks for it. Of the
rest, B is the only one that removes the trap rather than explaining it
(A) or relocating it (D). It matches a convention users already know,
and it keeps `set` from rewriting lines it was not asked to touch
(unlike C).

If the owner prefers not to widen accepted syntax, **§ 2 plus Option A**
is the strict alternative. It is cheaper, and still fixes everything
that is actually broken.

## 5. Testing and verification (for whichever option is chosen)

- Every variant × {default options, non-default options}: written by
  the writer, loaded by the loader, equal to the original model.
- The editor path: a GUI-style `UpdateRootSetting` for each strategy
  name produces a file that loads.
- Typos are still refused with the enumerated list of names.
- Option B only: `"weighted_random"` and `{ weighted_random = {} }`
  load to the same model; a file using either is **not** rewritten by
  an unrelated `set`.
- Option A only: the refusal is quoted verbatim in the review package.
- Baselines: declared, whatever they do.

## 6. Unresolved

1. ~~**Which option?**~~ **Decided 2026-10-07: § 2 plus Option B.**
   Option E stays deferred until a GUI needs to set a `seed` or a
   `tiebreaker`.
2. ~~**Release.**~~ **The next minor (6.4.0).** It widens the syntax
   the loader accepts, which is a feature, not a patch. 6.3.0 shipped
   without it.
