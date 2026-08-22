# Threat model

This page states, deliberately, what apimock's actual security surface
is in 6.0.0 — what each actor can do, what apimock allows on purpose and
why, and what apimock is not trying to protect against. It supersedes
[RFC 048](https://github.com/apimokka/apimock-rs/blob/main/rfcs/accepted/048-v6-cli-interface-concept.md)
§ 9, which was written before `apimock set` existed and never revisited
once it shipped — the gap this page exists to close, by living
somewhere it will actually be read again.

## Non-goals — read this first

**apimock is a development tool.** It is not hardened against hostile
input, and it is not designed for multi-tenant use. **It should not be
exposed to an untrusted network.** Nothing below changes that. If you
need a mock server a stranger can safely send traffic to, this isn't
it — bind it to `localhost`, run it behind something that is designed
for that job, or don't expose it at all.

apimock also does not defend a user against their own commands. If you
type `apimock set --rule-set /etc/whatever.toml --allow-outside`, it
does what you asked — the same way `cp` or `rm` would.

## Actors

- **A person at a shell.** Trusted, along with the filesystem they own.
  Explores by trial, reads output, adjusts. apimock does not second-guess
  a command this actor typed themselves.
- **An AI CLI agent.** This is the actor 6.0.0's CLI surface is designed
  for, and the one that changes the picture from earlier releases: it
  composes commands from material it did not author — a spec being
  mocked, a filename in a task description, a path another tool handed
  back — runs them non-interactively, and builds on the result without a
  human reviewing any single step. *"The user asked for it"* does not
  hold the same way here, because the user did not type the command; an
  agent acting on untrusted input can be induced to run one it shouldn't.
- **CI.** Runs a fixed set of commands, asserts exit codes, never answers
  a prompt, has no hidden network dependency.
- **The GUI application.** A long-lived session against the
  `apimock-config`/`apimock-routing` library API directly — not through
  the CLI. Its trust model is "the library API keeps working," not
  anything CLI-specific.
- **An MCP host.** Behaves and fails the same way the AI CLI agent does,
  through an adapter. No separate model is needed for it.

## Surface

**What the server reads.** The root config (`apimock.toml`), every file
listed in `service.rule_sets`, every file listed in
`service.middlewares` (`.rhai` scripts, compiled once at startup — no
file-watch or hot-reload), and TLS certificate/key files when
configured. All are read from paths an operator put in their own config
file. A file actually *served* in response to a request — whether found
via the dyn-route fallback, a rule's `respond.file_path`, or a path a
Rhai middleware returns — is confined to the directory it was resolved
against; see T3, below.

**What the CLI writes.** `apimock set` creates or rewrites the root
config and a rule-set file, in place, preserving comments and key order
(RFC 056). `--init` writes a starter config (and optionally a rule-set
file, a middleware file, a TLS section) non-interactively or
interactively. Nothing else in the CLI writes to the filesystem.

**What middleware can do.** Rhai's engine is constructed with
`Engine::new()` — the default, unsandboxed configuration; apimock
registers no filesystem, network, or environment-variable access, and
Rhai's standard library doesn't expose those on its own, so a script's
*own* code cannot open sockets or read arbitrary files directly. A
script receives exactly two values: the request's `url_path` and its
parsed JSON `body` — no headers, no method. Its return value drives the
response: a string names a file to serve, or a map selects `file_path`
/ `json` / `text`. A script that fails to compile or panics at runtime
is logged and the request falls through to the next stage — it cannot
crash the process, but it can silently degrade routing.

**What TLS touches.** apimock terminates TLS itself via `rustls` — this
is not a reverse-proxy setup. Certificates can hot-reload without
rebinding the listener (RFC 020): in-flight handshakes finish on the old
cert, new connections get the new one. There is no client-certificate
(mTLS) support; enabling or disabling TLS itself still requires a full
restart.

## Deliberate allowances, with reasons

**`apimock set` creates a file containing rule-set TOML at a
caller-named path — confined by default (RFC 062).** The underlying
capability is real: `set` is, in the abstract, a file-creation
primitive. For a person at a shell this is unremarkable — the same
category of thing `cp`'s destination argument is. For an AI CLI agent
composing an unreviewed path, it stops being unremarkable, because the
"caller" who named the path and the user who will be blamed for what
happened aren't reliably the same judgment. **`set` refuses a
`--rule-set` (or any other caller-supplied write target) that resolves
outside the root config's own directory tree** — `usage`, exit 2,
nothing written, not even a bootstrap file — unless `--allow-outside`
opts back in. Resolution is by canonicalised path where the target
exists, and by canonicalised parent where it doesn't, since `set`
legitimately creates files that don't exist yet and a naive
existence-requiring check would break ordinary bootstrapping. Refusing
rather than warning follows the precedent already set for `--dry-run`
(RFC 057 REVIEW-001 § 4): a safety affordance that sometimes acts anyway
is worse than one that declines outright, because the exception is
invisible at the call site.

**This confinement is CLI-layer only.** `apimock-config`'s library API
— and so the GUI, once it consumes it directly — does not inherit it.
This is deliberate, not an oversight: pushing the check into `Workspace`
would change a published library API to protect against a threat model
(an untrusted caller composing paths) that doesn't describe the GUI's
own actor. If confinement should hold for every caller of the library,
that's a follow-up RFC's decision, not something bundled quietly into
this one.

**`--file` (on `get`/`set`) is out of scope for this confinement, on
purpose.** `set --file <path>` never reads that path — it stores the
string as `respond.file_path` in the rule-set TOML, for the *server* to
read later, at serve time. It's a reference, not a write target, so the
write-path confinement above doesn't apply to it the way it applies to
`--rule-set`. (`get --body-file <path>` is unrelated: it's a genuine
local read, used only to build a synthetic request for `apimock get`'s
own dry-run matching — never anything the server itself touches.)

**Verbose header logging redacts; verbose body logging does not, yet.**
`log.verbose.header` (default off) prints every request header, with
anything matching the credential-shaped denylist (`authorization`,
`cookie`, `set-cookie`, `proxy-authorization`, `x-api-key`, or a
configured allowlist/denylist) replaced with a redacted marker — RFC 051.
**`log.verbose.body`, independently gated and also default off, prints
the raw query string and the full JSON body with no redaction at all.**
RFC 051 flagged this itself (its own Unresolved Question 2) and
deliberately left it for a later RFC rather than scope-creeping into it.
Stated here so it isn't only findable by reading that RFC: turning on
body logging can put credentials or other sensitive body fields on the
console, verbatim, today.

## Settled decisions, restated in full

**T2 — a configuration write becomes code execution — decided
2026-08-17: deferred, not refused.** `service.middlewares` lists Rhai
scripts the server compiles and runs; `set` could, in principle, attach
one. It does not: `set`'s first cut never adds, changes, or removes
`service.middlewares` entries — existing entries pass through untouched.

This was **not** decided on maintenance cost, though it was first
argued that way. Checking the source showed the machinery `set` would
need already exists: `Server::new` already compiles and propagates
middleware failures loudly at startup, middleware paths already resolve
against the config directory, and `requires_reload` already models the
"changes take effect on restart" semantics `set` would need. The
maintenance argument was asserted without checking the code, and it was
wrong.

The real argument is about a capability, not effort: a caller who can
invoke `set` could cause the server to run a file of their choosing on
its next boot. For a person at a terminal that's unremarkable — they
could edit the file directly. For an agent acting on untrusted input, it
is the difference between changing what a mock *returns* and running
*code* in the process — worth a deliberate scope decision rather than an
incidental default.

That argument has a hole worth stating honestly: **refusing does not
prevent it.** An agent that can be induced to run `apimock set` can be
induced to write the `.rhai` file directly instead. The refusal is a
speed bump against a capable attacker, not a barrier — its value is
against the *inadvertent* case (a "just set this field" verb quietly
gaining code-execution as a side effect), not a determined one.

The real cost of building this later isn't maintenance either — it's
correctness. A `set` that writes a middleware path can leave a workspace
that no longer boots (a missing file, or one that doesn't compile) —
discovered only when the server next starts, long after `set` reported
success. Building this means pulling Rhai compilation into `set`'s own
path so success is verified before it's reported, not treating
`service.middlewares` like every other field.

**If middleware attachment is built later**, it must be through an
explicit command or flag — never through a generic "set this field"
verb — so code execution is never a side effect of an ordinary config
edit, and intent is visible in whatever composed the command.

**T1 — path traversal through a caller-supplied write path — status as
of 6.0.0: enforced.** RFC 048 required this without specifying a
mechanism; RFC 062's confinement (above) is that mechanism, for `set`'s
one caller-supplied write target.

**T3 — path traversal through the serve path (the read side) — status
as of 6.0.0: enforced.** Complementary to T1, and the gap this page
itself flagged when it first shipped (RFC 062) — now closed (RFC 063).
A resolved file is served only if it stays within the directory it was
resolved against, at every site that can produce one: the dyn-route
fallback (a request-derived path), a rule's `respond.file_path`, and a
path a Rhai middleware script returns (both operator-authored). Each
checks by canonicalising the resolved candidate and confirming it
remains inside the canonicalised base directory for that site — the
fallback respond dir, the rule set's own respond dir, or the middleware
script's own directory, respectively. A violation is a bare 404,
indistinguishable from an ordinary not-found, so a prober learns
nothing about whether the target exists.

Unlike T1, **this has no opt-out.** RFC 062 gave `set --rule-set` an
escape hatch because a caller naming an outside path is asking for it
and is the only one exposed; the serve path is reachable by anything
that can send a request, so no config toggle turns it off. If files
genuinely live elsewhere, point `respond_dir` at them directly —
explicit, per rule set, already supported.

Unlike T1, **this is not CLI-layer only.** It's enforced inside
`apimock-server` itself — the running server, and `apimock get` (RFC
055), which calls the exact same dispatch functions the server does, so
neither can answer differently than the other for the same request. The
asymmetry the previous version of this page flagged — write path
confined, read path open — no longer exists.

As defence in depth, `normalize_url_path` also strips a `..` segment
from the request path before it reaches file resolution at all — this
closes the ordinary case earlier, but it is not the fix: it cannot help
`respond.file_path` or a Rhai-returned path (neither is built from a
URL), and a symlink escaping the base is caught only by
canonicalise-and-compare. Two independent controls, deliberately: the
RFC's own framing was "neither alone is the fix."

**This was a vulnerability in released versions, not only a v6
hardening.** Before the fix, the dyn-route fallback joined a
request-derived path onto the response directory and checked only that
the result existed, so a request carrying an un-normalised `..` segment
could read a file outside it. Affected: **5.0.0 through 5.19.0**. Fixed
in **5.19.1** and **6.0.0**, released together with
[<!-- GHSA-ID -->](<!-- advisory URL -->). If you are on the 5.x line,
5.19.1 is the fix — you do not need to move to 6.0.0 for it.

Practical exposure was bounded: apimock binds `127.0.0.1` by default, so
it was not reachable off-host unless the listener had been pointed
elsewhere, and it required a client that does not normalise `..` before
sending (browsers and most proxies and HTTP libraries do). Bounded is
not the same as absent, which is why it was fixed rather than
documented.

Per-request cost: the base directory is canonicalised once, when the
server starts (or `apimock get` runs) — not per request. The only
per-request work is canonicalising the resolved candidate, measured at
under a microsecond on a warm filesystem cache, immaterial next to the
network I/O already in every request.

**Other threats RFC 048 named, current status:**

- **Indirect prompt injection reaching `set` through an AI agent** isn't
  solvable inside the CLI — apimock's obligation is not to *amplify* it:
  no shell evaluation of arguments, no implicit writes, and destructive
  operations stay explicit rather than inferred. Unchanged by this RFC.
- **Secret leakage through verbose output** — see body logging, above;
  partially addressed (headers), not fully.
- **Symlink / TOCTOU on a configuration write** — `set`'s atomic
  write-then-rename (RFC 056) and external-change detection (RFC 024,
  042) cover the write side; nothing here re-litigates that.
- **A server-hosted configuration API** — never built. Not a live
  surface.
- **Supply chain of new dependencies** — covered by the existing
  `cargo audit` / lockfile CI gates (RFC 033), no new mechanism needed
  per dependency.
