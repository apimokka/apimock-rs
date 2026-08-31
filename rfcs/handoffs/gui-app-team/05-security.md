# 5. Security — you are already an actor in the threat model

`docs/src/reference/threat-model.md` names you explicitly:

> **The GUI application.** A long-lived session against the
> `apimock-config`/`apimock-routing` library API directly — not through
> the CLI. Its trust model is *"the library API keeps working"*, not
> anything CLI-specific.

**Read that page.** It is short, it is honest about what apimock does
*not* defend against, and it opens with a "Non-goals — read this first"
section that will save you assuming protection that is not there.

## The headline: apimock is a development tool

It is not hardened for production or for exposure to a network you do
not control. The default bind is `127.0.0.1`. If your GUI offers to
bind `0.0.0.0` — a reasonable feature, for testing from a phone on the
same LAN — **make that an explicit, informed choice in the UI**, not a
default and not a checkbox with no explanation.

A remotely reachable path traversal was found and fixed during 6.0.0's
development ([GHSA-72g6-wgrg-vhm7](https://github.com/apimokka/apimock-rs/security/advisories/GHSA-72g6-wgrg-vhm7),
fixed in 4.8.1 / 5.19.1 / 6.0.0). It was bounded largely *because* the
default bind is loopback. That bound is worth preserving.

## Write confinement — the one you must not silently opt out of

`apimock set` confines its writes to the config's own directory tree
(RFC 062). Writing outside requires an explicit `--allow-outside` flag
on the CLI.

**Your GUI is a config writer too.** The same question applies: when a
user picks a rule-set path outside the workspace, does your GUI write
there silently?

RFC 064 Amendment 1 added a hard rule on the CLI side worth
understanding, because the reasoning transfers directly: a **no-value
flag given `=value` is rejected**, specifically so `--allow-outside=false`
can never be silently read as "present" and disable confinement while
the author wrote `false` to keep it on. The principle is that a
security control must not be disabled by a value the parser ignores.

The GUI analogue is a confirmation the user actually saw, not a setting
they toggled once and forgot. **How you present it is yours; that there
is a decision here is not.**

## Rhai middleware executes

`service.middlewares` lists Rhai scripts, and `apimock-server` compiles
and runs them. **A config file is therefore executable content**, not
inert data.

If your GUI ever opens a configuration a user did not author — an
example downloaded from somewhere, a file in a shared repository, a
template — that is code they are about to run. The threat model treats
operator-authored config as trusted; a GUI that makes opening arbitrary
configs easy changes who "the operator" is.

**We have not designed for that case.** If your product does it, tell
us — it may need a real answer rather than an assumption.

## Response bodies can be any file the process can read

A rule's `respond.file_path` serves a file. RFC 065 confined and
validated this considerably, but the model remains: an operator-authored
config can serve any file the process can read.

For a GUI this mostly means: the file picker you offer defines what a
user is likely to serve. Rooting it at the respond directory is a
better default than the filesystem root. `Workspace::list_directory`
exists for exactly this.

## What we would ask of you

- **If you find something that looks like a security issue, do not open
  a public issue.** The reporting process is in
  [`.github/SECURITY.md`](../../../.github/SECURITY.md) — GitHub's
  private vulnerability reporting, which becomes a GHSA draft.
- **Tell us if your product changes an actor's trust level** — the
  "opens configs the user did not write" case above being the obvious
  one. The threat model is a living document and it already has a row
  for you; it should describe what you actually do.
