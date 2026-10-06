# Handoff — RFC 085: accept a bare strategy name, and write only what loads

**Governing RFC.** [085](../../accepted/085-strategy-spelling.md),
**accepted 2026-10-07: § 2 plus Option B.** Options A, C, D, E and F are
*not* in scope; E (a typed editor API) is explicitly deferred.
**Milestone.** 6.4.0. Nothing is tagged, bumped or published (RFC 066 § 2).
**Baseline.** **`main`'s head — cut from it.**
**Branch.** **Take one.** RFC 080 § 3 lists *"line endings, file IO"*.
This work changes the config **writer**, and its tests write files and
compare them. Run CI on the branch (`workflow_dispatch`) before merging,
as you did for task 019 R-01.

---

## 0. The one-paragraph version

There are five strategies. `first_match` and `round_robin` take no options
and load as bare strings. `uniform_random`, `weighted_random` (a `seed`)
and `priority` (a `tiebreaker`) load **only** as tables:
`strategy = "weighted_random"` fails with *"invalid type: unit variant,
expected struct variant"*. Two changes:

1. **Option B — the loader:** a bare name loads as *that strategy with
   default options*. The table form keeps working unchanged, and is needed
   only to set an option.
2. **§ 2 — the writer:** it currently writes every strategy as its bare
   name (`strategy.to_string()`). That writes files that do not load, and
   would drop a `seed` or `tiebreaker`. It must write a form the loader
   accepts, carrying every option.

## 1. The loader — `crates/apimock-routing/src/strategy.rs`

`Strategy` is `#[derive(Clone, Deserialize, Debug, Default)]` with
`#[serde(rename_all = "snake_case")]` (`:17-20`). Externally tagged, so a
table is `{ <name> = { …options } }`.

- [ ] Accept **either** a string naming any of the five strategies, which
      gives that variant with its options at their defaults
      (`seed: None`; `PriorityTiebreaker::default()`), **or** the existing
      table form, with **identical** behaviour to today, including how
      unknown option keys inside the table are treated.
- [ ] **A typo must still be refused with the names listed.** Something
      like *unknown strategy `weigthed_random`, expected one of
      `first_match`, `round_robin`, `uniform_random`, `weighted_random`,
      `priority`*. Quote the actual message in your package. Do not
      regress to a generic *"data did not match any variant"*; a custom
      `Deserialize` makes that easy to do by accident.
- [ ] Both places a strategy appears: `[service] strategy`
      (`crates/apimock-config/src/config/service_config.rs:35`) and a rule
      set's top-level `strategy` (`crates/apimock-routing/src/rule_set.rs:118`).
      Both use the same type, so one change should cover both. **Prove it
      for both**, not one.
- [ ] `Display` (`strategy.rs:60-67`) is unchanged; it is used for
      display, not serialisation.

## 2. The writer — `crates/apimock-config/src/toml_writer.rs`

Today: `:123-124` (rule set) and `:214-217` (`[service]`) both write
`Value::String(strategy.to_string())`.

- [ ] Write the **shortest form that loads and round-trips**: the bare
      name when every option is at its default; the table, with its
      options, when any is not. One function, used at both sites.
- [ ] **Task 019's strategy guard must compare the whole model, not the
      variant name.** Today an existing table is kept when the *variant*
      is unchanged (see `a_strategy_table_survives_when_the_variant_is_unchanged`,
      `:1424`). That was safe while the writer could not carry options.
      Once it can, comparing names would **silently ignore a changed
      `seed` or `tiebreaker`**: the silent-wrongness class this project
      keeps closing. The rule must be:
      - **model unchanged** → leave the line exactly as the user wrote it,
        bare or table, comment included. An unrelated `set` must not
        rewrite it (Option C was rejected precisely for that);
      - **model changed** → write the shortest form from the first bullet.
- [ ] The library editor (`crates/apimock-config/src/workspace/edit/root_setting.rs`,
      `ServiceStrategy`) already builds the right model, e.g.
      `WeightedRandom { seed: None }`. **No change there**; it starts
      working once the writer does.

## 3. Docs

- [ ] `docs/src/reference/rule-set-schema.md`, section ``## `strategy` ``.
      It currently says the three option-taking strategies are *"always
      written as a table"*. That was true when written (`5bdc662`) and
      becomes false with this change. Rewrite it to the Option B rule:
      *name the strategy; add a table only to set options*. **Keep a
      dated correction note**, as that page does elsewhere, saying the
      bare names now load and from which release.
- [ ] `docs/src/reference/apimock-toml-root-settings.md:96` says *"string
      or table"*. Still true; make sure the linked guide agrees.
- [ ] `docs/src/guides/vary-the-response-for-one-path.md` shows the
      table forms (correct). Add the bare-name shorthand where it reads
      naturally. Do not remove the table examples.
- [ ] A line for the 6.4.0 migration guide. Nothing breaks (this only
      widens what loads), so say so plainly.

## 4. Acceptance

- [ ] **Every strategy × {default options, non-default options}:** loaded
      from both spellings where both exist; written by the writer;
      re-loaded; equal to the original model. Both levels (`[service]` and
      rule set).
- [ ] `"weighted_random"` and `{ weighted_random = {} }` load to the
      **same** model.
- [ ] **An unrelated `set` rewrites neither spelling.** A file using
      `strategy = "weighted_random"`, and one using
      `strategy = { weighted_random = {} } # comment`, are byte-identical
      after a `set` that changes a different rule. Run it in LF and CRLF,
      as task 019's whole-file tests do.
- [ ] **A changed option is written, not dropped:** the model's `seed`
      goes from `7` to `8` (via a test, since no editor command sets a
      seed today), and the file then says `8`.
- [ ] The editor path: `UpdateRootSetting { ServiceStrategy }` with each of
      the five names produces a file that **loads** — the defect this RFC
      exists for, verified with the real loader.
- [ ] A typo is refused with the five names listed — quoted.
- [ ] `apimock-routing` baseline: a hand-written `Deserialize` may or may
      not move it. **Declare the real diff, whatever it is.**
- [ ] Full suite green; CI green **on the branch**, then on `main`.

## 5. Report back

`.git-exclude/review-request/085-strategy-spelling/`. Include the typo
message, the round-trip matrix, the unrelated-`set` byte-identity result
in both line endings, the seed-change result, and the baseline diff.
