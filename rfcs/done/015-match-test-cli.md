# RFC 015 — `apimock match-test` CLI subcommand

**Status.** Implemented (v5.10.0)
**Tracks.** RFC 008 future possibility — a CLI dry-run tool that
evaluates a body condition (or full rule `when` clause) against a
JSON file without starting the server.
**Touches.** `crates/apimock` (`args.rs`, new `cmd/match_test.rs`),
`apimock-routing` (no change — uses existing `is_match` paths),
documentation.

## Summary

`apimock match-test` lets a user verify that a rule's `when`
conditions match (or don't match) a given request on the command
line, without booting the full server. It is the natural diagnostic
companion to the trace channel: when a trace event shows a rule
didn't match, the user can run `match-test` to find out which
condition failed and why.

## Motivation

Debugging non-matching rules today requires:

1. Running the server.
2. Issuing a `curl` against it.
3. Reading the server log.
4. Iterating on the TOML rule.

With `match-test`, the feedback loop collapses to one command:

```sh
apimock match-test \
  --rule-set apimock-rule-set.toml \
  --rule 3 \
  --body '{"action":"create","items":[{"qty":5}]}' \
  --header "Content-Type: application/json" \
  --method POST \
  --path /api/orders
```

Output:

```
Rule #3: /api/orders (POST)
  url_path   ✓  equal /api/orders
  method     ✓  POST
  headers    ✓  Content-Type contains json
  body.json  ✗  items.0.qty: greater_than 10  (actual: 5)

Result: NO MATCH
```

The exit code is 0 on match, 1 on no-match, 2 on error — so CI
scripts can assert without parsing stdout.

## Guide-level explanation

### Flags

| Flag | Required | Description |
|------|----------|-------------|
| `--rule-set <path>` | yes | Rule set TOML file to load |
| `--rule <n>` | no | 1-based rule index; default: test all rules |
| `--body <json>` | no | Request body as inline JSON string |
| `--body-file <path>` | no | Request body from file |
| `--header <name: value>` | no, repeatable | Request header |
| `--method <GET\|POST\|…>` | no | HTTP method; default: GET |
| `--path <url-path>` | no | URL path; default: `/` |
| `--quiet` | no | Only print result line and exit code |

### Output format

Per-condition result lines use icons:
- `✓` — condition matched
- `✗` — condition did not match
- `-` — condition absent (rule has no such condition)

For body conditions, the actual resolved value is shown alongside
the configured value when the condition fails.

```
Rule #3: POST /api/orders
  url_path   ✓  equal /api/orders   → "/api/orders"
  method     ✓  POST
  headers    ✓  content-type contains "json"
             ✗  x-tenant-id equal "acme"  (header absent)
  body.json  ✗  items.0.qty greater_than 10  (actual: 5)

Result: NO MATCH  (2 of 5 conditions failed)
```

On match:

```
Rule #3: POST /api/orders — MATCH
```

### Testing multiple rules

Without `--rule`, all rules are tested and the first match is
highlighted:

```
Rule #1: GET /api/users        NO MATCH
Rule #2: GET /api/orders       NO MATCH
Rule #3: POST /api/orders   ★  MATCH  ← would be selected (FirstMatch strategy)
Rule #4: POST /api/orders      MATCH
```

## Reference-level explanation

### New subcommand

`Args` gains a `MatchTest(MatchTestArgs)` variant:

```rust
pub enum Args {
    Run(RunArgs),
    Init(InitArgs),
    MatchTest(MatchTestArgs),
}

pub struct MatchTestArgs {
    pub rule_set: PathBuf,
    pub rule: Option<usize>,            // 1-based
    pub body: Option<String>,
    pub body_file: Option<PathBuf>,
    pub headers: Vec<String>,           // "Name: Value"
    pub method: Option<String>,
    pub path: Option<String>,
}
```

### Implementation (`cmd/match_test.rs`)

```
1. Load rule set via RuleSet::new().
2. Build a ParsedRequest from flags:
   - url_path: --path flag (default "/")
   - http_method: --method flag (default GET)
   - headers: parse --header flags
   - body_json: parse --body / --body-file flag as serde_json::Value
3. For each rule (or the specified rule):
   a. For each condition (url_path, method, headers.*, body.*):
      - Call the individual is_match predicate.
      - Record pass/fail + actual value for body conditions.
   b. Print per-condition result lines.
4. Print summary line.
5. Exit 0 (all specified rules match), 1 (no match), 2 (error).
```

The per-condition drill-down is the key novelty. It requires calling
`UrlPath::is_match`, `HttpMethod::is_match`, `Headers::is_match`,
and the per-path `BodyOperator::is_match` individually, not the
combined `When::is_match`. This requires the per-condition types
to be publicly accessible — they already are (`pub mod` throughout
the routing crate).

For body conditions, `json_value_by_jsonpath` is called directly to
retrieve the actual value for display.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | At least one rule matched (or the specified rule matched) |
| 1 | No rule matched |
| 2 | Error (rule-set file not found, invalid JSON body, etc.) |

## Drawbacks

1. **Adds a new subcommand to the CLI surface.** The CLI is
   currently just `run` and `init`. A third subcommand raises
   discoverability expectations (tab-completion, help text, etc.).
2. **Per-condition output is informational, not machine-readable.**
   The stdout format is human-friendly prose. If a scripting use
   case needs machine-readable output (JSON result per condition),
   a `--json` flag would be a natural follow-up.
3. **Does not simulate strategy.** `match-test` shows which rules
   match, but does not apply the configured strategy to select one.
   Showing "which rule would win" requires knowing the strategy,
   which lives in `apimock.toml`, not the rule set file. A
   `--config` flag could address this in a follow-up.

## Rationale and alternatives

**Alternative A: verbose server log mode.** Emit per-condition
pass/fail to the log for every request. Useful but requires
running the server, and the log is already noisy.

**Alternative B: GUI-side dry-run.** The GUI builds a synthetic
`ParsedRequest` and calls the routing crate in-process. This works
for GUI users but not for CLI/CI workflows.

**Alternative C (this RFC): CLI subcommand.** Usable in CI, shell
scripts, and local debugging without a running server.

## Unresolved questions

1. **`--config` flag to load strategy from `apimock.toml`.** Would
   make `match-test` aware of which rule would actually be selected.
   Adds complexity; deferred.
2. **`--json` output flag.** Machine-readable results for CI
   assertion. Deferred to a follow-up.
3. **`--path` default.** Using `/` as the default URL path may mask
   misconfigurations if the user forgets `--path`. Should the
   default be `None` (require explicit path)? TBD at implementation.
