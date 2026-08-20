//! Load sampler: drive the apimock server at a target RPS and record
//! memory (RSS), CPU (user+system jiffies), request throughput, and
//! latency over time.
//!
//! # Why this is an `example` rather than a criterion bench
//!
//! criterion measures per-iteration wall-time; it has no concept of
//! "sustain this much load for ten seconds and tell me how memory
//! behaves". Process-level metrics like RSS and CPU percent need a
//! sampling loop running alongside the workload, and criterion's API
//! gets in the way of that shape. A purpose-built binary is the right
//! tool.
//!
//! # Why no new dependencies
//!
//! The sampling reads `/proc/self/status` and `/proc/self/stat`
//! directly — two parse routines, ~40 lines. A crate like `procfs`
//! would be overkill here, and the project intentionally keeps its
//! dep footprint small.
//!
//! # Why we drive the server in-process instead of via `exec`
//!
//! A separate server process would need its own IPC channel for
//! metric sampling, which would immediately pull in new deps. Running
//! apimock in the same process (on its own tokio runtime) lets us
//! sample RSS/CPU for a clean process that has exactly one workload
//! in it — which is what you want for this kind of metric anyway.
//!
//! # Usage
//!
//! ```text
//! cargo run --release --example bench_load -- \
//!     --rps 500 --duration 10 --endpoint /text
//! ```
//!
//! Output is CSV on stdout: `t_ms,rss_kb,cpu_user_ticks,cpu_sys_ticks,\
//! inflight_requests`. The final line is a summary prefixed with
//! `# summary` (so shell tools treating `#` as a comment still work).
//!
//! # Platform note
//!
//! Uses `/proc/self/{status,stat}`, which is Linux-only. On macOS or
//! Windows it prints a notice and runs the load portion without the
//! memory/CPU columns populated. The latency/throughput portion works
//! everywhere.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use apimock::{App, EnvArgs};
use tokio::sync::Semaphore;

#[derive(Clone)]
struct CliArgs {
    rps: u32,
    duration_secs: u64,
    endpoint: String,
    /// Maximum concurrent in-flight requests. Guards against pathological
    /// slowness where the client would otherwise launch unbounded tasks.
    concurrency: usize,
    /// Sampling interval for memory/CPU metrics.
    sample_every_ms: u64,
}

impl CliArgs {
    fn parse() -> Self {
        // Minimal manual parser — keeps us dep-free. Unknown flags are
        // ignored, which makes the tool forgiving of accidental typos
        // in ad-hoc local runs.
        let mut rps = 500u32;
        let mut duration = 10u64;
        let mut endpoint = "/text".to_owned();
        let mut concurrency = 256usize;
        let mut sample = 100u64;

        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut i = 0;
        while i < args.len() {
            let take = |idx: usize| args.get(idx + 1).cloned().unwrap_or_default();
            match args[i].as_str() {
                "--rps" => rps = take(i).parse().unwrap_or(rps),
                "--duration" => duration = take(i).parse().unwrap_or(duration),
                "--endpoint" => endpoint = take(i),
                "--concurrency" => concurrency = take(i).parse().unwrap_or(concurrency),
                "--sample-ms" => sample = take(i).parse().unwrap_or(sample),
                "--help" | "-h" => {
                    eprintln!(
                        "Usage: bench_load --rps <N> --duration <SEC> --endpoint <PATH> \
                         [--concurrency <N>] [--sample-ms <MS>]"
                    );
                    std::process::exit(0);
                }
                _ => {}
            }
            i += 1;
        }

        Self {
            rps,
            duration_secs: duration,
            endpoint,
            concurrency,
            sample_every_ms: sample,
        }
    }
}

/// Per-sample snapshot we keep around for the end-of-run summary.
///
/// The CSV also emits t_ms / inflight, but those aren't needed after
/// printing — so they don't live in the struct, to keep clippy quiet
/// about dead fields without having to silence the lint.
#[derive(Default, Clone, Copy)]
struct Sample {
    rss_kb: u64,
    cpu_user_ticks: u64,
    cpu_sys_ticks: u64,
}

/// Parse `VmRSS` out of `/proc/self/status`. Returns None on non-Linux.
fn read_rss_kb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            // format: "VmRSS:\t  12345 kB"
            return rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok());
        }
    }
    None
}

/// Parse utime / stime out of `/proc/self/stat`. Returns None on non-Linux.
///
/// `/proc/self/stat` has a notoriously annoying format because field 2
/// (`comm`) is in parentheses and may contain spaces. We skip past the
/// closing paren before splitting the rest on whitespace; fields from
/// `state` onward are then position-indexed per `proc(5)`.
fn read_cpu_ticks() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/self/stat").ok()?;
    let close_paren = text.rfind(')')?;
    let rest = &text[close_paren + 1..];
    let fields: Vec<&str> = rest.split_whitespace().collect();
    // After skipping pid and comm, the next field (index 0 here) is
    // `state`. utime is field 14 of the original stat line (index 11 in
    // `rest`), stime is field 15 (index 12). See `man proc` "/proc/[pid]/stat".
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((utime, stime))
}

/// Prepare a minimal config + fixture so the server has something to
/// serve at `/text` and `/file`.
fn prepare_fixtures(port: u16) -> PathBuf {
    let dir = tempdir_leaked();

    let fallback_dir = dir.join("fallback");
    std::fs::create_dir_all(&fallback_dir).expect("mkdir fallback");
    std::fs::write(
        fallback_dir.join("hello.json"),
        "{\"greeting\":\"hello\",\"items\":[1,2,3]}",
    )
    .expect("write hello.json");

    let rule_set_path = dir.join("rules.toml");
    let fallback_abs = fallback_dir.canonicalize().expect("canonicalize fallback");
    std::fs::write(
        &rule_set_path,
        format!(
            "[prefix]\nrespond_dir = \"{}\"\n\n\
             [[rules]]\n\
             when.request.url_path = \"/text\"\n\
             respond = {{ text = \"hello from text rule\" }}\n\
             \n\
             [[rules]]\n\
             when.request.url_path = \"/status\"\n\
             respond = {{ status = 204 }}\n\
             \n\
             [[rules]]\n\
             when.request.url_path = \"/file\"\n\
             respond = {{ file_path = \"hello.json\" }}\n",
            fallback_abs.to_string_lossy(),
        ),
    )
    .expect("write rules.toml");

    let config_path = dir.join("apimock.toml");
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {port}\n\
             \n\
             [log]\n\
             verbose = {{ header = false, body = false }}\n\
             \n\
             [service]\n\
             rule_sets = [\"{}\"]\n\
             fallback_respond_dir = \"{}\"\n",
            rule_set_path.file_name().unwrap().to_string_lossy(),
            fallback_dir.file_name().unwrap().to_string_lossy(),
        ),
    )
    .expect("write apimock.toml");

    config_path
}

/// Make a tempdir that outlives the process — mirroring the bench
/// pattern. The leak is intentional; the OS reclaims on exit.
fn tempdir_leaked() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();
    std::mem::forget(dir);
    path
}

fn pick_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

/// No-op `log::Log` for silencing library log output. See `main` for why.
struct NullLogger;
impl log::Log for NullLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }
    fn log(&self, _: &log::Record) {}
    fn flush(&self) {}
}

#[tokio::main]
async fn main() {
    // Install a no-op `log::Log` as the process-global logger *before*
    // the server starts. `apimock::App::new` internally calls
    // `log::set_logger` too, but that call is idempotent (it returns an
    // error which `App::new` ignores), so whichever logger is installed
    // first wins — which lets us keep the CSV output clean without
    // changing any library code.
    //
    // # Why silence logs here specifically
    //
    // The sampler prints one CSV line per sample on stdout. The server's
    // default stdout logger prints a line per request, also on stdout.
    // Mixed together, the output is unusable for any downstream CSV
    // consumer. Sending logs to /dev/null is the right call for a
    // bench tool; operators debugging the server itself wouldn't be
    // running this binary.
    let _ = log::set_boxed_logger(Box::new(NullLogger));
    log::set_max_level(log::LevelFilter::Off);

    let cli = CliArgs::parse();

    let port = pick_port();
    let config_path = prepare_fixtures(port);

    // Start the server on its own tokio runtime so its worker loop
    // never contends with the load generator on a small host.
    //
    // # Why we leak this runtime
    //
    // Dropping a multi-thread `Runtime` from inside another runtime's
    // async context panics with "Cannot drop a runtime in a context
    // where blocking is not allowed". The cleanest fix for a short-lived
    // bench tool is `Box::leak`: the OS reclaims everything when the
    // process exits, and we never try to tear the runtime down early.
    let server_rt: &'static tokio::runtime::Runtime = Box::leak(Box::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("server runtime"),
    ));

    let config_path_string = config_path.to_string_lossy().into_owned();
    server_rt.spawn(async move {
        let mut env_args = EnvArgs::empty();
        env_args.config_file_path = Some(config_path_string);
        let app = App::new(&env_args, None, false)
            .await
            .expect("App::new for load sampler");
        app.server.start().await;
    });

    // Give the server time to bind. Same pattern as the integration
    // tests, same 400ms heuristic.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let base_url = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(cli.concurrency)
        .timeout(Duration::from_secs(10))
        .build()
        .expect("reqwest client");

    // Smoke-check so we fail loud before the sampling loop if the
    // server isn't actually serving yet.
    if let Err(err) = client
        .get(format!("{}{}", base_url, cli.endpoint))
        .send()
        .await
    {
        eprintln!(
            "bench_load: server not reachable at {} ({}); aborting",
            base_url, err
        );
        std::process::exit(1);
    }

    run_load(&cli, &base_url, &client).await;
}

/// The actual load + sample loop.
async fn run_load(cli: &CliArgs, base_url: &str, client: &reqwest::Client) {
    // Header. Lines starting with '#' or the column header can be fed
    // straight into pandas/awk/etc.
    println!(
        "# apimock bench_load: rps={} duration={}s endpoint={} concurrency={} sample_every_ms={}",
        cli.rps, cli.duration_secs, cli.endpoint, cli.concurrency, cli.sample_every_ms
    );
    if read_rss_kb().is_none() {
        println!("# note: /proc/self/status not available on this platform; rss_kb will be 0");
    }
    println!(
        "t_ms,rss_kb,cpu_user_ticks,cpu_sys_ticks,inflight_requests,completed,errors,avg_latency_us"
    );

    let started = Instant::now();
    let end_at = started + Duration::from_secs(cli.duration_secs);
    let interval = Duration::from_nanos((1_000_000_000u64 / cli.rps.max(1) as u64).max(1));
    let stop = Arc::new(AtomicBool::new(false));
    let inflight = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicU64::new(0));
    let errors = Arc::new(AtomicU64::new(0));
    let latency_ns_sum = Arc::new(AtomicU64::new(0));
    let sem = Arc::new(Semaphore::new(cli.concurrency));

    // --- Sampler task
    let sampler_stop = stop.clone();
    let sampler_inflight = inflight.clone();
    let sampler_completed = completed.clone();
    let sampler_errors = errors.clone();
    let sampler_latency = latency_ns_sum.clone();
    let sample_every = Duration::from_millis(cli.sample_every_ms.max(10));
    let start_instant = started;
    let sample_capacity = (cli.duration_secs * 1000 / cli.sample_every_ms.max(1)).max(1) as usize;
    let sampler = tokio::spawn(async move {
        let mut samples: Vec<Sample> = Vec::with_capacity(sample_capacity);

        let mut last_completed = 0u64;
        let mut last_latency_ns = 0u64;

        while !sampler_stop.load(Ordering::Relaxed) {
            let t_ms = start_instant.elapsed().as_millis() as u64;
            let rss_kb = read_rss_kb().unwrap_or(0);
            let (u, s) = read_cpu_ticks().unwrap_or((0, 0));
            let cur_completed = sampler_completed.load(Ordering::Relaxed);
            let cur_latency_ns = sampler_latency.load(Ordering::Relaxed);
            let delta_completed = cur_completed.saturating_sub(last_completed);
            let delta_latency_ns = cur_latency_ns.saturating_sub(last_latency_ns);
            let avg_latency_us = delta_latency_ns.checked_div(delta_completed).unwrap_or(0) / 1_000;

            println!(
                "{},{},{},{},{},{},{},{}",
                t_ms,
                rss_kb,
                u,
                s,
                sampler_inflight.load(Ordering::Relaxed),
                cur_completed,
                sampler_errors.load(Ordering::Relaxed),
                avg_latency_us,
            );

            samples.push(Sample {
                rss_kb,
                cpu_user_ticks: u,
                cpu_sys_ticks: s,
            });

            last_completed = cur_completed;
            last_latency_ns = cur_latency_ns;
            tokio::time::sleep(sample_every).await;
        }

        samples
    });

    // --- Load generator
    let url = format!("{}{}", base_url, cli.endpoint);
    let mut next_launch = Instant::now();
    while Instant::now() < end_at {
        // Simple rate pacing: wait until the next tick, then launch.
        let now = Instant::now();
        if next_launch > now {
            tokio::time::sleep(next_launch - now).await;
        }
        next_launch += interval;

        // Reserve a slot before spawning. If the server falls behind,
        // this is where we stop piling more work on — better to report
        // inflight=concurrency than to OOM the benchmark host.
        let permit = match sem.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                // Over-budget. Back off until a slot frees. We don't
                // "catch up" by firing a burst, because that would
                // measure the client's catchup, not the server's ceiling.
                errors.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let url = url.clone();
        let client = client.clone();
        let inflight = inflight.clone();
        let completed = completed.clone();
        let errors = errors.clone();
        let latency_ns_sum = latency_ns_sum.clone();
        tokio::spawn(async move {
            let _permit = permit;
            inflight.fetch_add(1, Ordering::Relaxed);
            let start = Instant::now();
            match client.get(&url).send().await {
                Ok(resp) => {
                    // Drain the body so the measurement reflects the
                    // full response, not just headers.
                    let _ = resp.bytes().await;
                }
                Err(_) => {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
            let latency = start.elapsed().as_nanos() as u64;
            latency_ns_sum.fetch_add(latency, Ordering::Relaxed);
            completed.fetch_add(1, Ordering::Relaxed);
            inflight.fetch_sub(1, Ordering::Relaxed);
        });
    }

    // Let inflight work drain briefly. 500ms is generous for loopback.
    tokio::time::sleep(Duration::from_millis(500)).await;
    stop.store(true, Ordering::Relaxed);

    let samples = sampler.await.unwrap_or_default();

    // --- Summary
    let total_done = completed.load(Ordering::Relaxed);
    let total_err = errors.load(Ordering::Relaxed);
    let total_latency_ns = latency_ns_sum.load(Ordering::Relaxed);
    let avg_latency_us = total_latency_ns.checked_div(total_done).unwrap_or(0) / 1_000;
    let peak_rss_kb = samples.iter().map(|s| s.rss_kb).max().unwrap_or(0);
    let final_cpu_user = samples.last().map(|s| s.cpu_user_ticks).unwrap_or(0);
    let final_cpu_sys = samples.last().map(|s| s.cpu_sys_ticks).unwrap_or(0);
    let elapsed_s = started.elapsed().as_secs_f64().max(0.001);
    let achieved_rps = total_done as f64 / elapsed_s;

    println!(
        "# summary duration_s={:.3} target_rps={} achieved_rps={:.1} completed={} errors={} avg_latency_us={} peak_rss_kb={} cpu_user_ticks_total={} cpu_sys_ticks_total={}",
        elapsed_s,
        cli.rps,
        achieved_rps,
        total_done,
        total_err,
        avg_latency_us,
        peak_rss_kb,
        final_cpu_user,
        final_cpu_sys,
    );
}
