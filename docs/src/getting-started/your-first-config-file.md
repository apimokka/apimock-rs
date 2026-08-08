# Your first config file

File-based routing alone can't vary a response by header, method, or
body content — for that, apimock needs a config file.

## Generate one

```sh
apimock --init
```

Interactive by default — it prompts for the port, IP, fallback
directory, and whether to scaffold a rule-set file, a middleware file,
and a TLS section, then writes `apimock.toml` accordingly (plus
whichever of `apimock-rule-set.toml` / `apimock-middleware.rhai` you
opted into). Passing `--yes` skips the prompts and writes the defaults
directly — see the [CLI reference](../reference/cli-reference.md#--init)
for both.

Run it, and the startup log names every config file it loaded:

```sh
apimock
# @ rule_set #1 (./apimock-rule-set.toml)
# Listening on http://127.0.0.1:3001 ...
```

## TOML, briefly

`apimock.toml` and rule-set files are ordinary TOML. A few things worth
knowing if you haven't used it before:

- `[table]` starts a section; everything until the next `[...]` header
  belongs to it.
- Nested tables can be written with dotted headers:
  `[rules.when.request.headers]` is shorthand for a `headers` table
  inside a `when` table inside a `request` table inside `rules`.
- `[[rules]]` (double brackets) is an *array* of tables — each
  `[[rules]]` block starts a new entry in a list, which is why a rule
  set can have several `[[rules]]` sections.
- Keys with characters TOML treats specially (like the `.` in a dotted
  body path) need quoting: `"customer.tier" = { ... }`, not
  `customer.tier = { ... }`.

The full TOML specification is at [toml.io](https://toml.io/) if you
want more than this project needs day to day.

## Where files live, relative to what

If you move `apimock.toml` and its rule-set files into a subdirectory
and point `apimock` at it explicitly:

```sh
apimock -c tests/apimock.toml
```

paths *inside* that config — `service.rule_sets`,
`service.fallback_respond_dir` — resolve relative to `apimock.toml`'s
own location, not the directory you ran `apimock` from. (TLS
certificate paths are the one exception — see
[Serve over HTTPS](../guides/serve-over-https.md).)

Next: [Your first rule](./your-first-rule.md).
