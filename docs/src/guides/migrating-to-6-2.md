# Migrating to 6.2.0

**Filename and version number are a placeholder** — no release number
has been decided for this cycle yet; RFC 066 § 2 keeps that decision
outside this page's author entirely (versions, tags, and publishing are
never touched without explicit instruction). `6.2.0` is written here
only as "the next minor after 6.1.0" — 6.1.0 is already tagged and
carries tranches 1–3 of the external audit, so this tranche's entries
land in a new page rather than being folded into that one. Rename this
file and its `SUMMARY.md` entry to match whatever the release process
actually settles on.

Two RFCs land here, from the external audit's fourth tranche. Both are
**fixes that change what an existing setup does** — the same reasoning
that made tranches 1–3 a minor, not a patch:

| RFC | What breaks |
|---|---|
| [075](#url-paths-are-now-percent-decoded-and-case-folded-at-every-segment) | A URL segment's case that used to matter (or not) may now resolve differently — see below for exactly when |
| [075](#a-rule-sets-url_path-prefix-now-matches-at-a-segment-boundary) | A rule set scoped to a prefix like `/api` stops matching a similarly-spelled sibling path like `/apiv2` |
| [076](#json-files-are-now-served-exactly-as-written) | A `.json` `file_path` response is no longer minified or key-reordered |
| [076](#library-api-the---format-json-envelope-field-order-changed) | *Library and script consumers only:* `--format json`'s field order changed from alphabetical to `schema`, `apimock`, `result`/`error` |

Every one of these is a genuine correctness fix for behaviour the
external audit found; none is a style or convenience change. If your
setup changes under one of them, it was already answering inconsistently
or unfaithfully — see each RFC for the reproduction.

## URL paths are now percent-decoded, and case-folded at every segment

**RFC 075, F-03 and F-05.** Two related fixes to how a URL path becomes
a file or a rule match.

**Percent-decoding (F-03).** A URL segment containing `%XX` escapes is
now decoded before matching — `%20` becomes a space, `%C3%A9` becomes
`é`. Before this, a fixture whose name needed encoding (a space, a
non-ASCII character) was permanently unreachable, however it was
requested:

```
$ mkdir -p api && echo '{}' > 'api/my file.json'
$ curl http://localhost:3001/api/my%20file.json
# before: 404 — decoding never happened at all
# after:  200 — resolves api/my file.json
```

**This cannot reintroduce path traversal.** Decoding runs *before*
dot-segment normalisation, so a percent-encoded `..` (`%2e%2e`, any
mix of case, with the slash encoded or not) is stripped by the same
mechanism that already strips a literal `..` — and the confinement
check added for
[GHSA-72g6-wgrg-vhm7](https://github.com/apimokka/apimock-rs/security/advisories/GHSA-72g6-wgrg-vhm7)
still runs regardless, as an independent backstop. Both layers were
verified together before this shipped.

**Case-folding, extended to every segment (F-05).** Case-insensitive
filename matching already existed; it's now applied to every segment
of the path, not only the last one:

```
$ mkdir -p API && echo '{}' > API/users.json
$ curl http://localhost:3001/api/users.json
# before: 404 on Linux, 200 on Windows/macOS — same config, same
#         request, different answer depending on the filesystem
# after:  200 on every platform — apimock folds the case itself
```

**If your setup depended on the old inconsistency** — a config that
only worked because a differently-cased path segment happened to 404
on your platform (or resolve on it) — this now resolves the same way
everywhere. That is the fix, not a regression: a committed rule set
should not depend on which OS runs it.

**Unicode case-folding, not just ASCII.** `É` and `é` are treated as
the same case, matching what a case-insensitive filesystem (macOS
APFS, Windows NTFS) already does for free — Linux now does the same
folding itself rather than 404ing where the other two platforms
wouldn't have. This is **case folding**, not Unicode *normalisation*
(NFC vs NFD — how an accented character is *encoded*, not how its case
is folded): normalisation remains explicitly out of scope, a
filesystem-dependent question this project doesn't chase.

**One narrow, disclosed exception**, in the same spirit as tranche 3's
own precedence disclosure: if a directory holds both a bare,
extension-less file and a same-named file with an extension (e.g.
`foo` and `foo.json`), for an extension-less request the exact-path
match and the extension-inferred match are now tried in that order
before falling back to a case-insensitive listing scan. In the
overwhelming common case (one file, one name) this changes nothing;
it can only matter for a directory layout intentionally holding two
same-stemmed candidates, which nothing in this project's own test
corpus or examples does.

## A rule set's `url_path` prefix now matches at a segment boundary

**RFC 075, F-02.** A rule set's `[prefix].url_path` used to be compared
against the request path with a plain string prefix check — meaning a
rule set scoped to `/api` also claimed `/apiv2`, `/apixyz`, or any other
path that merely started with the same characters:

```toml
[prefix]
url_path = "/api"
```

```
$ curl http://localhost:3001/apiv2/users
# before: matched by /api's rule set, however unrelated apiv2 was
# after:  not matched — /api only ever matches /api itself or /api/...
```

**If a request that used to reach this rule set now 404s (or falls
through to a different rule set or the dyn-route fallback), this is
why.** Anything this un-matches was matched by accident — the fix is to
scope the request under the correct prefix, not to work around the
correction. A prefix of exactly `/` is unaffected: it was already, and
remains, a deliberate catch-all matching every request.

## `.json` files are now served exactly as written

**RFC 076, F-04 and P-04.** A `.json` `file_path` response used to be
parsed and re-serialised on every request — minified, and with object
keys sorted alphabetically, regardless of how the file was actually
written:

```
$ echo '{
  "zebra": 1,
  "apple": 2
}' > data.json
$ curl http://localhost:3001/data
# before: {"apple":2,"zebra":1}      — reordered and minified
# after:  {
#   "zebra": 1,
#   "apple": 2
# }                                   — served exactly as written
```

**If your setup, or a snapshot/golden-file test built against it,
depends on the old minified-and-alphabetised output, this changes what
you get.** That output was never documented and was always an
unannounced side effect of how the file happened to be parsed and
rebuilt — the zero-config promise is "the JSON you put on disk is what
a client gets back," and this is what makes it true. A `.json5`
`file_path` is unaffected: JSON5 syntax isn't valid JSON, so converting
it remains the point, not a defect. Inline `respond.json` is also
unaffected in the sense that matters here — it still converts (JSON5
tolerant, and it may be minified), but its **key order** now survives
the conversion too, for the same underlying reason as the next section.

## Library API: the `--format json` envelope field order changed

**RFC 076 § 3 — library and script consumers only.** This section
matters if you parse `--format json` output by comparing serialised
text (rather than by key, which is what the format is actually for) or
if you depend on `apimock-routing`/`apimock-config`/`apimock-server`
serialising a `serde_json::Value` map in a particular order.

Fixing `.json` file fidelity (above) and inline `respond.json`'s key
order both required enabling `serde_json`'s `preserve_order` feature —
a workspace-wide switch, since it changes how every `serde_json::Value`
map serialises, not something scopable to one call site. This also
changed the RFC 053 CLI envelope's (`--format json`'s) field order:

```
// before: alphabetical (serde_json's default without preserve_order)
{"apimock":"6.1.0","result":{...},"schema":1}

// after: insertion order — matches every example this project's own
// docs have shown since RFC 053
{"schema":1,"apimock":"6.1.0","result":{...}}
```

**This was accepted deliberately, not absorbed as a side effect** — RFC
076 § 3 required an explicit choice between accepting the change and
scoping `preserve_order` away from the envelope. Accepting it was
chosen because the new order **matches what this project's own
documentation already showed as the example output** on every page
covering `--format json`; the old alphabetical order was the thing
quietly disagreeing with the docs, not the other way round.

**If your consumer parses the envelope as a JSON object** (reading
`.schema`, `.apimock`, `.result`/`.error` by key, as the format's own
`--format json` name implies), this does not affect you — JSON objects
are unordered by specification, and nothing about this changes which
keys exist or what they mean. It only affects a consumer comparing
serialised bytes directly, or relying on iteration order over a parsed
map.

## What isn't changing

`.csv` conversion is unaffected by RFC 076 — it's already a
transformation, and stays one. Row order was always source order (a
JSON array, never subject to the alphabetical-keys issue this RFC
fixes); each row's own object now keys its fields in the CSV's column
order rather than alphabetically, the same `preserve_order` side
effect as everywhere else on this page — a cosmetic change for CSV
specifically, since which columns exist and what they contain is
unchanged. Rule-set scoping semantics other than the prefix
segment-boundary fix above are unchanged. No config setting is required
to get any of the fixes on this page; all are corrections to existing
behaviour, not new opt-in features.
