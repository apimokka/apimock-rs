# Implementation Handoff — 6.0.0: `apimock serve`, and `--allow-outside` in help

**Governing RFC.** [RFC 053](../../accepted/053-v6-cli-contract.md)
§ "Bare `apimock` keeps working, and gains an explicit form".
**Also closes.** Pre-cut audit F-1
(`.git-exclude/release/6.0.0/PRE-CUT-AUDIT.md`).
**Milestone.** 6.0.0. **Blocking the cut.**
**Baseline.** `main` @ `68df8c3`.

**Self-contained.** Everything binding is restated here.

**Two items in one handoff** because both land in
`crates/apimock/src/args.rs` — one in the dispatch block, one in the
help text a few hundred lines below. Separate branches would collide.

---

## 1. `apimock serve` — approved, never built

RFC 053 specifies:

> *"`apimock serve [flags]` becomes the explicit spelling. **Bare
> `apimock` remains an alias for it.**"*

It was never implemented. Until yesterday `apimock serve` appeared to
work — because **any** bare word started a server. That defect is now
fixed, so `apimock serve` is a usage error, and shipped behaviour
contradicts an accepted RFC more than it did before.

This is not new scope. RFC 053 is accepted; this delivers it.

### What it must do

| Invocation | Expected |
|---|---|
| `apimock serve` | Identical to bare `apimock` — zero-config server |
| `apimock serve -c <path>` | Identical to `apimock -c <path>` |
| `apimock serve -p <port>` / `-d <dir>` | Identical to the same flags without `serve` |
| `apimock serve --init [--yes] [--middleware]` | Identical to `--init` without `serve` |
| `apimock serve --help` / `--version` | Same as the root command's |
| bare `apimock` | **Unchanged** |

"Identical" is the requirement, not "similar". `serve` is a spelling of
the existing path, not a second path.

### The likely shape — establish it, don't take it from me

`crates/apimock/src/args.rs:114` now rejects any bare token at position
1 via `exit_unknown_subcommand`. `serve` needs to be recognised before
that check so it is not rejected.

Beyond that, **I believe** nothing else is needed: `EnvArgs::from_args`
and `args_option_value` scan `env::args()` for flags, no positional is
consumed anywhere, and `reject_unknown_arguments` tolerates a bare token
(`strict_bare_tokens: false` for root). So a stray `serve` may simply be
ignored by everything downstream.

**I have not tested that.** Verify it; if a stray `serve` does confuse
any downstream scan, strip it instead and say so. *(A worked example in
a handoff is a claim — mine were wrong twice this week, so this one is
labelled as untested rather than stated as fact.)*

### Do not

- Add flags to `serve` that the root command does not have.
- Make `serve` mandatory, or deprecate bare `apimock` — RFC 053 is
  explicit that bare `apimock` **remains** an alias.
- Give `serve` its own `--help` text that could drift from the root's.

## 2. F-1 — `--allow-outside` is missing from `set --help`

`apimock set --help` does not list `--allow-outside`. It is documented
in `docs/src/reference/cli-reference.md` (usage line ~365, flag table
~399) and in the threat model, and `args.rs:463`'s own doc comment says
the help text **matches** that page.

It matters because `--allow-outside` is **RFC 062's write-path
confinement opt-out** — the flag that lets `set` write outside the
workspace — and `--help` is the first place a user or agent looks.

Add it to `set`'s help, in the same style as its neighbours, matching
the reference's wording: *"Permit `--rule-set` to resolve outside the
config directory."*

**While you are there:** `serve` must appear in the root help's
`Subcommands:` list too, and in the CLI reference. Keep the two in step
— that is the invariant `args.rs:463` states and F-1 is a case of it
having drifted.

## 3. Search, don't trust this list

The named files are a **floor**:

- `crates/apimock/src/args.rs` — dispatch, root help, `set` help.
- `docs/src/reference/cli-reference.md` — document `serve`; check the
  `set` flag table still matches the help after § 2.
- `docs/src/guides/migrating-to-6-0.md` — it currently says *"there is
  no `serve` subcommand to be an alias for."* **That becomes false.**
  Rewrite it.

**Then grep** for `serve` across `docs/`, `README.md`, examples and
source comments, and for `--allow-outside`. Report everything you find,
including what you decide needs no change.

Two places found by the last sweep that are **not yours**: `ROADMAP.md`
(architect-owned) and `rfcs/done/054-…` (historical record). Report, do
not edit.

## 4. Not in scope

- Any other subcommand or flag.
- Changing bare `apimock`'s behaviour in any way.
- Version bump, CHANGELOG, README notice — RFC 066 § 2.

## 5. Acceptance

- [ ] Each row of § 1's table verified **by running both spellings** and
      comparing — not by reading the code path
- [ ] `apimock serve` with a config that would fail to load fails the
      **same way** bare `apimock` does
- [ ] Bare `apimock` unchanged; `apimock banana` still exit 2
- [ ] Every other subcommand unchanged
- [ ] `serve` in the root help's `Subcommands:` list and in the CLI
      reference
- [ ] `--allow-outside` in `set --help`, wording matching the reference
- [ ] The migration guide's "no `serve` subcommand" sentence corrected
- [ ] § 3's sweep reported
- [ ] `cargo test --workspace`, `fmt`, `clippy -D warnings`,
      `mdbook build docs`
- [ ] CI green on all 9 jobs before merge

## 6. Report back

`.git-exclude/review-request/6-0-0-serve-and-help/`, including the
**side-by-side output** of each § 1 row (`serve` form vs bare form),
captured from real runs — that equivalence is the deliverable.
