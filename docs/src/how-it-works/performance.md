# Performance

Three ways to measure apimock-rs yourself, rather than a claim to take
on trust — see [Design notes](./design-notes.md) for the read-on-demand
model these exist to verify. All three are dev-only: no new runtime
dependency, nothing shipped in a release build.

## `cargo bench --bench routing` — pure matching cost

Microbenchmarks `RuleSet::find_matched` in isolation — no HTTP, no
tokio, no file I/O. Three scenarios (`first_rule_hit`, `last_rule_hit`,
`miss_all_specific_rules`), each parametrised over rule-set sizes of 1,
10, and 100 rules (`crates/apimock/benches/routing.rs`). Useful when
changing the matcher itself — a new operator, prefix handling — for a
sub-microsecond-resolution before/after comparison. Finishes in under a
minute.

## `cargo bench --bench response_latency` — end-to-end HTTP latency

Stands up a real apimock server on a random port once per run, then
benches five response kinds through a `reqwest` client
(`crates/apimock/benches/response_latency.rs`):

| Bench | What it covers |
|---|---|
| `text_rule` | Static text response from a rule — no file I/O after startup |
| `status_rule` | Status-only response — the shortest response path |
| `file_rule_warm` | File-backed rule, page cache warm — steady-state real-world latency |
| `dyn_route_fallback` | Zero-config "just drop JSON in a folder" path |
| `not_found` | 404 path — worth tracking separately since misconfigured clients hit it often |

The gap between `text_rule` and `file_rule_warm` is the measured cost
of the per-request file read described in
[Design notes](./design-notes.md), on your own machine, rather than an
asserted number.

## `cargo run --release --example bench_load` — sustained-load sampler

Criterion measures per-iteration wall time; it can't show what happens
to process RSS or CPU while sustaining a given request rate for a
while. `bench_load` (`crates/apimock/examples/bench_load.rs`) is a
standalone binary that does that, by constructing a `Server` in-process
via the public `App` API and sampling `/proc/self/{status,stat}`
alongside HTTP-level latency and throughput.

```sh
cargo run --release --example bench_load -- \
    --rps 500 --duration 10 --endpoint /text
```

Output is CSV on stdout, one line per sample interval, plus a final
`# summary` line. **The shape below is illustrative of the output
format — it is not a captured measurement of this project**; run the
command yourself for real numbers on your own machine:

```
# apimock bench_load: rps=500 duration=10s endpoint=/text concurrency=256 sample_every_ms=100
t_ms,rss_kb,cpu_user_ticks,cpu_sys_ticks,inflight_requests,completed,errors,avg_latency_us
0,18760,1,0,0,1,0,332
102,18760,9,0,1,51,0,245
...
# summary duration_s=10.02 target_rps=500 achieved_rps=... completed=... errors=... avg_latency_us=... peak_rss_kb=...
```

| Flag | Default | Meaning |
|---|---|---|
| `--rps <N>` | 500 | Target request rate |
| `--duration <SEC>` | 10 | How long to sustain the load |
| `--endpoint <PATH>` | `/text` | URL path to hit — `/text` / `/status` / `/file` / `/hello` are preconfigured by the fixture |
| `--concurrency <N>` | 256 | Max in-flight requests; exceeding it drops the request and increments `errors`, so outpacing the server is visible rather than silently absorbed |
| `--sample-ms <MS>` | 100 | How often to sample RSS / CPU |

RSS and CPU-tick columns come from `/proc/self/{status,stat}` and are
Linux-only — the program prints a notice and reports zeros for those
two columns on macOS / Windows. Latency and throughput columns work
everywhere. To compare two builds, run both at the same target RPS
values and compare `achieved_rps`, `peak_rss_kb`, and `avg_latency_us`.
