# Benchmarks

4.8.0 adds two benchmark suites plus a load-sampling example. All three are dev-only (not shipped in release builds) and pick up no new runtime dependencies.

## `cargo bench --bench routing` — pure-CPU routing cost

Microbenchmarks for `RuleSet::find_matched` in isolation — no HTTP, no tokio, no file I/O. Three scenarios (first-rule hit, last-rule hit, miss-all-specific-rules), parametrised over rule-set sizes of 1 / 10 / 100.

Use this when you're changing the matcher (new `RuleOp`, prefix handling, etc.) and want a sub-microsecond-resolution answer to "did my change help or hurt?". Finishes in under a minute.

## `cargo bench --bench response_latency` — end-to-end HTTP latency

Stands up a real apimock server on a random port once per run, then benches five response kinds through a reqwest client:

| Bench | What it covers |
| --- | --- |
| `text_rule` | Static text response from a rule — no file I/O after startup. |
| `status_rule` | Status-only response — the shortest response path. |
| `file_rule_warm` | File-backed rule, page cache warm — steady-state real-world latency. |
| `dyn_route_fallback` | Zero-config "just drop JSON in a folder" path. |
| `not_found` | 404 path — worth tracking separately because misconfigured clients hit this a lot. |

The gap between `text_rule` and `file_rule_warm` is the honest measure of "file I/O cost per request" — instead of inventing an artificial I/O microbench we expose it where it actually shows up.

## `cargo run --release --example bench_load` — memory / CPU / RPS sampler

criterion measures per-iteration wall-time; it can't tell you what happens to process RSS or CPU utilisation when you sustain a given RPS for 30 seconds. `bench_load` is a small standalone binary that does that:

```sh
cargo run --release --example bench_load -- \
    --rps 500 --duration 10 --endpoint /text
```

Output is CSV on stdout, one line per 100 ms sample, plus a final `# summary` line:

```
# apimock bench_load: rps=500 duration=10s endpoint=/text concurrency=256 sample_every_ms=100
t_ms,rss_kb,cpu_user_ticks,cpu_sys_ticks,inflight_requests,completed,errors,avg_latency_us
0,18760,1,0,0,1,0,332
102,18760,9,0,1,51,0,245
...
# summary duration_s=10.02 target_rps=500 achieved_rps=480.1 completed=4807 errors=0 avg_latency_us=290 peak_rss_kb=19200 ...
```

Flags:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--rps <N>` | 500 | Target request rate. |
| `--duration <SEC>` | 10 | How long to sustain the load. |
| `--endpoint <PATH>` | `/text` | URL path to hit. `/text` / `/status` / `/file` / `/hello` are preconfigured by the fixture. |
| `--concurrency <N>` | 256 | Max in-flight requests. When exceeded, the client drops the request and increments `errors` — so it's visible when you've outpaced the server. |
| `--sample-ms <MS>` | 100 | How often to sample RSS / CPU. |

RSS and CPU ticks come from `/proc/self/{status,stat}`, so those two columns are Linux-only; the program prints a notice and continues with zeros on macOS / Windows. The latency and throughput columns work everywhere.

Pipe into pandas / awk / your tool of choice. To compare two builds, run at multiple target RPS values (e.g. 100, 500, 1000, 2000) and plot `achieved_rps` vs `peak_rss_kb` / `avg_latency_us`.
