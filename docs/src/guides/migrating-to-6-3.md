# Migrating to 6.3.0

**Start with the first section if you have ever run `apimock set`**, or
saved configuration through a program built on `apimock-config`. A
defect in every release from 6.0.0 to 6.2.1 could silently delete
settings from your files. Upgrading stops it happening again, but it
**does not put back** anything already removed.

| change | who it affects | what to do |
|---|---|---|
| [`set` no longer deletes settings](#check-files-that-apimock-set-rewrote) | anyone who ran `apimock set`, or a GUI save, on 6.0.0–6.2.1 | **check your files** |
| [PATCH is matchable; `HttpMethod` is non-exhaustive](#http-method-is-now-non-exhaustive-and-patch-is-matchable) | config authors (new ability); Rust consumers of `apimock_routing::HttpMethod` | add a `_` arm to an exhaustive `match` |
| [`[guard]` is deprecated](#guard-is-deprecated) | rule-set files containing `[guard]` | delete the line |
| [Clearer refusals for a rule's `method`](#a-mistyped-method-now-says-what-was-wrong) | nobody, unless you parse the message | nothing |

## Check files that `apimock set` rewrote

**This is the one change that can need action from you.**

When `apimock set` rewrote a configuration file, it kept the keys it
knew how to edit and **silently deleted every other one**. It exited 0,
printed nothing, and the result still validated. The same applied to
`Workspace::save` in the `apimock-config` library, which is how a GUI
saves. In 6.3.0 both change only the keys they manage and leave
everything else exactly as written, comments and line endings included.

**What may have been deleted from a file rewritten on 6.0.0–6.2.1:**

| file | setting | if it was deleted, you now have |
|---|---|---|
| `apimock.toml` | `[listener.tls] handshake_timeout_seconds` | the default, **10** seconds |
| `apimock.toml` | `[listener.tls] max_connections` | the default, **256** |
| `apimock.toml` | `[service] max_request_body_bytes` | the default, **32 MiB** |
| `apimock.toml` | `[service] middleware_max_operations` | the default, **10,000,000** |
| `apimock.toml` | `[service] cors_allow_credentials_origins` | none: credentialed CORS from those origins stops working |
| rule-set file | `[default] delay_response_milliseconds` | no rule-set-wide delay |
| rule-set file | every rule's `weight` | weight `1` for every rule, so `weighted_random` picks evenly |
| either | comments above settings `set` manages | (comments only) |

Two more forms of damage were possible. A rule-set `strategy` written as
a table, such as `{ weighted_random = { seed = 7 } }`, could be rewritten
to a bare name **that does not load**. You would have noticed that one:
the server refuses to start. And a file with Windows (CRLF) line endings
was rewritten with LF throughout.

`apimock.toml` was rewritten only when `set` changed something in it,
for example when adding a rule to a new rule-set file. A rule-set file
was rewritten on every `set` that targeted it.

**What to do.** If the files are under version control, compare them
with the last version you wrote by hand and restore anything in the
table above. **The first four rows matter most.** They are limits you
may have tightened, and a deleted limit reverts to a *looser* default.

## HTTP method is now non-exhaustive, and PATCH is matchable

**RFC 082.** A rule's `when.request.method` accepted only `GET`, `POST`,
`PUT` and `DELETE`, so a `PATCH` rule was refused at load even though
PATCH is in common REST use. It now matches:

```toml
[[rules]]
when.request.url_path = "/users/42"
when.request.method = "PATCH"
respond = { text = "patched" }
```

Two things came with it:

- **`access-control-allow-methods` now includes `PATCH`.** Without that, a
  browser's CORS preflight refuses the PATCH you just configured.
  `response-headers.md` shows the new value.
- **A refused method now says why, when there is a reason.** A method
  that is deliberately not matchable — `OPTIONS`, `HEAD`, `TRACE`,
  `CONNECT` — is refused with the valid set and the reason, instead of the
  set alone:

```
unknown variant `OPTIONS`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
  — OPTIONS is answered by the built-in CORS preflight handler before rule sets are consulted, so it cannot be matched by a rule
```

A typo still gets the set alone, with the list now including `PATCH`.

**If you match `HttpMethod` exhaustively in Rust**, the enum is now
`#[non_exhaustive]`, so the compiler needs a wildcard arm. Add one
arm — `_ => …` — and the next method added to the enum will not break
your code. This is the same one-time change as `Outcome` in 6.2.0, and
for the same reason. It applies to `apimock_routing::HttpMethod` only;
`Outcome` is unaffected by this release.

**If you rely on the text of the `set --method` refusal**, it now lists
`PATCH` and carries the same reason clause as the config refusal.

## `[guard]` is deprecated

`[guard]` in a rule-set file has never done anything: it is an empty
table with nowhere to put a setting. It now loads with a warning, once,
on stderr, and the exit code stays 0:

```
apimock: warning: `[guard]` in rule-set file `./rs/rules.toml` has no effect and will be rejected in a future release; remove it.
```

**Delete the line.** Removing it cannot change how your rules behave. A
future release will refuse the key outright, and a file still carrying
it will then fail to load, with the key named.

(Before 6.3.0, `apimock set` silently deleted `[guard]` when it rewrote
a file. It now leaves the line alone and warns instead.)

## A mistyped `method` now says what was wrong

A `method` that differs from a valid one only in its letter case is
refused with a hint, rather than with the bare list:

```
unknown variant `patch`, expected one of `GET`, `POST`, `PUT`, `DELETE`, `PATCH`
  — did you mean `PATCH`? method values are upper-case
```

The hint changes only the message. Exactly the same spellings are
accepted and refused as without it.

## What isn't changing

- **Config spelling stays case-sensitive.** `method = "patch"` in TOML is
  still refused, now with the hint above. A request's method on the wire
  still matches case-insensitively, so a client sending `patch` still
  matches a `PATCH` rule.
- **`HEAD` and `OPTIONS` are still not matchable.** Each has a reason in
  the refusal, and the [rule-set schema](../reference/rule-set-schema.md#which-methods-a-rule-can-match-and-why-not-all-of-them)
  explains why.
- **The `set` CLI still accepts any casing of `--method`.** It uppercases
  the value before reading it, as it always has, and writes the
  upper-case form.
