# Migrating to 6.4.0

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

**A misspelled bare name is refused in apimock's own words.** 6.3.0
already listed the five names, as serde's ``unknown variant `x`, expected
one of …``. 6.4.0 says `strategy`, and lists them in the order the docs
use:

```
unknown strategy `weigthed_random`, expected one of `first_match`, `round_robin`, `uniform_random`, `weighted_random`, `priority`
```

A misspelling *inside the table form* (`{ weigthed_random = {} }`) still
gets serde's message, unchanged from 6.3.0. It lists the same five names.

**What the editor writes.** `apimock set` has no option for choosing a
strategy, so this applies only to programs built on `apimock-config`. Such
a program, choosing a strategy through `Workspace`, used to write a file
apimock itself could not load, for the three strategies above. It now writes the shortest spelling
that loads back to the same strategy: the bare name when every option is at
its default, the table when one is not. And choosing the strategy a file
already has keeps its options. Choosing `weighted_random` again no longer
drops a `seed = 7`.

**An unrelated edit does not rewrite your strategy.** If a save does not
change the strategy, the line stays exactly as you wrote it, in whichever
spelling, comment included.
