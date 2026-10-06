# Migrating to 6.4.0

**Filename and version number are a placeholder** — the release number is
decided by the release process, not by this page (RFC 066 § 2). `6.4.0` is
written here as the next minor after 6.3.0. Rename this file and its
`SUMMARY.md` entry to match whatever the release actually is.

**Nothing breaks.** This release only widens what loads; every
configuration that loaded before loads the same way.

| change | who it affects | what to do |
|---|---|---|
| [A strategy can be written as a bare name](#a-strategy-can-be-written-as-a-bare-name) | config authors | nothing; the table form still works |

## A strategy can be written as a bare name

**RFC 085.** There are five strategies. `first_match` and `round_robin`
take no options and always loaded as bare strings. `uniform_random` and
`weighted_random` (a `seed`) and `priority` (a `tiebreaker`) loaded only as
tables, so `strategy = "weighted_random"` failed with *"invalid type: unit
variant, expected struct variant"*. It now loads, as that strategy with its
options at their defaults:

```toml
strategy = "weighted_random"                      # new: the default options
strategy = { weighted_random = { seed = 7 } }     # unchanged: to set an option
strategy = { weighted_random = {} }               # unchanged, and the same as the bare name
```

This works for `[service] strategy` and for a rule set's own `strategy`.

**A typo now says what was wrong.** A name that is none of the five is
refused with the five listed, where it used to give a message about serde
variants:

```
unknown strategy `weigthed_random`, expected one of `first_match`, `round_robin`, `uniform_random`, `weighted_random`, `priority`
```

**What the editor writes.** `apimock set` never writes a strategy, so it
does not apply there. A program built on `apimock-config` that chooses a
strategy through `Workspace` used to write a file apimock itself could not
load, for the three strategies above. It now writes the shortest spelling
that loads back to the same strategy: the bare name when every option is at
its default, the table when one is not.

**An unrelated edit does not rewrite your strategy.** If a save does not
change the strategy, the line stays exactly as you wrote it, in whichever
spelling, comment included.
