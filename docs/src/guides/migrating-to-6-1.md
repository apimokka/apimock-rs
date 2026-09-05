# Migrating to 6.1.0

**Filename and version number are a placeholder** — no release number
has been decided for this cycle yet; RFC 066 § 2 keeps that decision
outside this page's author entirely (versions, tags, and publishing
are never touched without explicit instruction). `6.1.0` is written
here only as "the next minor after 6.0.0," to give the entries below
somewhere to live as they land, per each RFC's own instruction to
write its migration note as it ships rather than at the end. Rename
this file and its `SUMMARY.md` entry to match whatever the release
process actually settles on.

Six RFCs land here so far, from the external audit's second and third
tranches. Each is a **fix that changes what an existing setup does**,
which is why this is a minor, not a patch — RFC 070 and RFC 071
additionally change the public API, which affects library consumers
only:

| RFC | What breaks |
|---|---|
| [069](#config-an-unknown-key-in-a-rule-condition-or-respond-block-now-fails-to-load) | A config that loads today stops loading |
| [070](#round_robin-now-rotates-per-match-group-not-per-rule-set) | A `round_robin` rule set returns a different sequence |
| [070](#library-api-a-public-field-on-ruleset-is-removed) | *Library consumers only:* a public field on `RuleSet` is removed |
| [072](#header-matching-now-fails-closed-on-non-utf-8-values) | A header condition that passes today starts failing |
| [071](#library-api-serverapp_state-is-now-shared-not-cloned) | *Library consumers only:* `Server::app_state`'s type changes, `AppState` loses `Clone` |
| [077](#a-narrow-disclosed-precedence-change-in-the-zero-config-fallback) | A contrived directory layout could serve a different file, **on a case-sensitive filesystem only** (see below — almost certainly doesn't apply to you) |

Every one of these is a genuine correctness fix for behaviour the
external audit found; none is a style or convenience change. If your
setup changes under one of them, it was already answering incorrectly
— see each RFC for the reproduction.

## Config: an unknown key in a rule, condition, or respond block now fails to load

**RFC 069.** A mistyped key inside `[[rules]]` — `headerz` instead of
`headers`, or any other typo in a rule, `when`/`request` condition, or
`respond` block — used to be silently discarded. The rule still
loaded, `apimock validate` reported success, and the rule then matched
**more** requests than it was written to, because the condition the
author intended simply wasn't there:

```
$ apimock validate -c ./cfg.toml
Validation passed (1 rules across 1 rule set(s)).      # before: wrong

$ apimock validate -c ./cfg.toml
apimock validate: failed to load config: ... unknown field `headerz`,
expected one of `url_path`, `method`, `headers`, `body`
(did you mean `headers`?)                               # after: correct
exit 2
```

**If a config that worked before now fails to load, this is the
likely reason.** The error names the exact key and, where the edit
distance makes one plausible, suggests the field you probably meant —
the same near-match courtesy an unknown CLI flag already gets. Fix the
key name (or remove it, if it was never meant to do anything) and the
config loads again, this time actually enforcing what it always looked
like it enforced.

**Scope**: this applies to the rule-facing surface only — `[[rules]]`
and everything under it, plus a rule set's own `[prefix]`, `[default]`,
and `[guard]` blocks. Root `apimock.toml` sections (`[listener]`,
`[service]`, `[log]`, `[file_tree_view]`) are unaffected by this
change — an unknown key there is still accepted, unchanged, for now
(RFC 069's own recorded, deliberate deferral, revisited once the
settings RFCs 067/068 added there have settled).

**Every config under `examples/` and this project's own test corpus
was checked directly against this change** — none contained a dead
key; nothing needed fixing beyond the fix itself.

## `round_robin` now rotates per match group, not per rule set

**RFC 070.** `round_robin` kept one counter for the whole rule set,
not one per distinct set of matching rules. A rule set that only ever
served one shape of request never noticed; a rule set serving more
than one did, and for some shapes never rotated at all:

```
# a rule set with 2 rules matching /a, 3 matching /b
# requesting /a alone, four times — this part was always correct
a1 a2 a1 a2

# alternating /a and /b — the bug
/a: a1 a1 a1 a1     # before: never rotates
/a: a1 a2 a1 a2     # after: rotates independently of /b
```

**If you have a `round_robin` rule set that serves more than one
distinct request shape, its rotation sequence changes** under this
fix — from a broken one to the one the strategy was always documented
as providing. A rule set with only one match group (every rule
matches the same request shape) is unaffected; its sequence is
unchanged. See [Vary the response for one path](./vary-the-response-for-one-path.md#round_robin)
for the corrected general-case description.

**No config or code change is required to adopt this fix** — it's a
matching-behaviour correction, not a new setting. If something
downstream was asserting on the old (broken) sequence specifically,
that assertion needs updating; nothing could have been correctly
depending on it, since the old sequence was undocumented and wrong.

## Library API: a public field on `RuleSet` is removed

**RFC 070 — library consumers only.** This section does not affect
running `apimock` as a server, or any configuration. It matters only
if you depend on the `apimock-routing` crate directly.

`RuleSet` carried its round-robin position as a public field:

```rust
pub round_robin_counter: Arc<AtomicUsize>,
```

Per-group rotation (above) cannot be expressed by a single counter, so
that state is now a map keyed by the matched rule set — and it is
**private**. The public field is removed, and no replacement field
takes its place:

```rust
// 6.0.0
let n = rule_set.round_robin_counter.load(Ordering::Relaxed);

// 6.1.0 — no equivalent: the rotation state is internal
```

**If this breaks your build, please tell us.** The field held
`RuleSet`'s own scheduling bookkeeping and was public only because
`RuleSet` is a plain data struct. It appears nowhere outside
`apimock-routing` in this workspace and we know of no reason to read
it — but if you had one, we would rather hear it than assume.

This is a breaking change to the public API within a major version.
Those are rare and we avoid them; the project does not promise they
are impossible. What it does promise is that none of them reaches a
release undeclared — see
[API stability](../library/api-stability.md).

## Header matching now fails closed on non-UTF-8 values

**RFC 072.** A header condition (`when.request.headers`) against a
request header whose value isn't valid UTF-8 used to **match
unconditionally** — logged an error, then treated the condition as
satisfied regardless of operator. A gate that can't evaluate its input
was silently opening rather than staying closed:

```toml
[rules.when.request.headers]
x-token = { op = "equal", value = "expected" }
```

```
$ curl -H 'x-token: <invalid-utf-8-bytes>' http://127.0.0.1:PORT/gated
# before: matched anyway, "expected" or not
# after: does not match — the condition cannot be satisfied by a
#        value it cannot read, regardless of which operator it uses
```

**`exists`/`absent` are unaffected** — both check only whether the
header key is present, before ever attempting to read its value, so a
present-but-undecodable header still satisfies `exists` and still
fails `absent` (the header genuinely is present; "cannot be read" and
"not present" are different things, and this fix does not conflate
them).

`apimock match-test` and `apimock get --why` now agree with the
server on this input too — before this fix, `match-test` treated a
non-UTF-8 header value as an empty string and answered independently
of the operator, which for a `not_equal` (or similar) condition could
disagree with what the server actually did. An agreement test now
pins both paths to the same corpus so this cannot silently drift
again.

**If a rule was relying on this to match a request whose header value
happens to not be valid UTF-8, that rule now correctly refuses it.**
This was already a bypass of whatever the condition was gating; there
is no supported way to opt back into the old behaviour, by design.

## Library API: `Server::app_state` is now shared, not cloned

**RFC 071 — library consumers only.** This section does not affect
running `apimock` as a server, or any configuration.

Every request used to clone the whole `AppState` (and therefore the
whole `Config`, rule sets included) out from behind a lock — cost
proportional to configuration size, on every request, serialised
through that lock. `AppState` is now held once and shared:

```rust
// 6.0.0
pub struct Server {
    pub app_state: AppState,
    // ...
}

// 6.1.0
pub struct Server {
    pub app_state: std::sync::Arc<AppState>,
    // ...
}
```

`apimock_server::service`'s second parameter changes the same way
(`Arc<Mutex<AppState>>` → `Arc<AppState>`), and `AppState` no longer
implements `Clone` — nothing needs to clone it any more, and removing
the impl closes the door on silently reintroducing the per-request
clone this RFC exists to remove. `AppState::config`'s field type and
`AppState::new`'s constructor signature are **unchanged** — every
existing read through those was already by reference, so only the
handle around `AppState` needed to change, not `AppState` itself.

**If this breaks your build**, replace an owned `AppState` (e.g. from
cloning `Server::app_state` before this change) with an `Arc<AppState>`
and read its fields through the `Arc` rather than a lock.

This is a breaking change to the public API within a major version, in
the same sense as RFC 070's field removal above — declared here and in
the baseline, not undeclared. See
[API stability](../library/api-stability.md).

## A narrow, disclosed precedence change in the zero-config fallback

**RFC 077.** The fallback `respond_dir` used to resolve a request by
listing the whole directory on every request, even when the exact file
already existed. It now tries the exact path, then extension inference
(`/hello` → `hello.json`, the shape this zero-config mode exists for)
before ever listing the directory — the listing is now reached only for
a case mismatch (e.g. a URL that canonicalised differently than the
filesystem did).

This is behaviour-identical for every configuration we could construct
a test for, **and the one exception it has depends on the filesystem's
own case sensitivity, not only on the directory layout**: a directory
containing **both** a bare, differently-cased file (e.g. `FOO`, no
extension) **and** an extension match for the same request (e.g.
`foo.json`), where the request has no extension.

- **Case-sensitive filesystem (Linux, the usual case for a server
  deployment):** before, the bare differently-cased file won (found by
  the listing, which ran first); now, the extension-inferred file wins
  (found by the cheaper stat, which now runs first). This is the actual
  behaviour change.
- **Case-insensitive filesystem (macOS APFS by default, Windows NTFS by
  default):** the new exact-path stat for the extension-less request
  already resolves to the differently-cased file at the OS level, so
  the bare file wins on both sides of this change — **no behaviour
  change there**, even for this exact layout.

Nothing in this project's own test corpus or examples has ever
exercised this layout; a dedicated test
(`bare_differently_cased_file_vs_extension_match_resolves_per_filesystem_case_sensitivity`
in `dyn_route.rs`) now pins both outcomes, detecting the running
filesystem's actual case sensitivity rather than assuming it from the
OS, and CI's three-platform matrix confirms both branches occur.

**If you don't keep both a bare, extension-less file and an
extension-inference-eligible file with the same name in the same
fallback directory, this does not affect you on any platform.**

## What isn't changing

Every other strategy (`first_match`, `priority`, `weighted_random`,
`uniform_random`) is unchanged — the audit found no defect in any of
them, and RFC 070 doesn't touch them. Header matching for a value that
*is* valid UTF-8 is unchanged — RFC 072 only closes the non-UTF-8 gap.
File content-type detection (text vs. binary) is unchanged — RFC 077's
P-05 removed a redundant second file read but kept the exact same
UTF-8-validity decision. No config setting is required to get any of
these fixes; all are corrections to existing behaviour or internal
performance work, not new opt-in features.
