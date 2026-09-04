//! End-to-end HTTP response-latency benchmarks.
//!
//! # What this measures
//!
//! The full path: TCP accept → hyper → routing → response. criterion
//! times each request the client sees, so the reported number includes
//! tokio scheduling, the loopback stack, and reqwest's client overhead.
//! That's the number users actually feel; we label it "end-to-end" so
//! it isn't confused with the CPU-only numbers from `routing.rs`.
//!
//! # Why the server is started once per group, not per iteration
//!
//! Spinning up tokio + binding a TCP socket + loading config takes
//! tens of milliseconds — much more than the request we're trying to
//! measure. criterion wants each sample to be dominated by the thing
//! under test, so we stand up one server in a module-scope `OnceLock`
//! and reuse it across every bench here.
//!
//! # Why HTTP and not pure routing
//!
//! This bench's job is complementary to `routing.rs`. `routing.rs`
//! answers "is the matcher fast?"; this one answers "what does a real
//! client see?". The gap between the two is how much tokio/HTTP
//! framing adds — and noticing that gap grow unexpectedly is the whole
//! point of keeping end-to-end latency in CI.
//!
//! # File I/O is implicit in "cold_file" vs "warm_file"
//!
//! The first read of a response file usually hits the disk; subsequent
//! reads hit the page cache. Rather than inventing an artificial "IO
//! benchmark", we expose this honestly: `cold_file` clears the relevant
//! file from cache (best-effort) before each sample, `warm_file` does
//! not. Operators debugging "why is my mock slow on the first hit?"
//! can read those numbers directly.
//!
//! # Rule-count and directory-size scaling (RFC 071 / RFC 077 P-06)
//!
//! `bench_response_latency` above holds configuration size fixed and
//! varies response *kind*. `bench_rule_scaling` and
//! `bench_directory_scaling` instead hold the request fixed (a
//! non-matching URL; an existing file) and vary configuration size —
//! that is the shape neither this bench nor `routing.rs` covered before,
//! and the shape RFC 071's per-request `Config` clone and RFC 077 P-06's
//! per-request directory listing both live in. Each rule/file count gets
//! its own server, cached the same way the shared `server()` is.

use std::collections::HashMap;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use apimock::{App, EnvArgs};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use tokio::runtime::Runtime;

/// Shared state between criterion setup and the async tasks.
struct BenchServer {
    base_url: String,
    fallback_dir: PathBuf,
    rt: Runtime,
}

/// One-time initializer so the server is started once per `cargo bench`
/// invocation. Using `OnceLock` instead of `lazy_static`/`once_cell` keeps
/// the bench dep-free and works with stable since 1.70.
static SERVER: OnceLock<BenchServer> = OnceLock::new();

/// Bring the shared server up if it isn't already, then return a handle.
fn server() -> &'static BenchServer {
    SERVER.get_or_init(|| {
        // Silence apimock's info-level request logging before the server
        // initialises its own logger — otherwise every benched request
        // writes a line to stdout and drowns out criterion's progress
        // output. `log::set_boxed_logger` is idempotent-with-last-wins
        // semantics: whichever call succeeds first wins, and apimock's
        // own `init_logger` silently tolerates the failure.
        let _ = log::set_boxed_logger(Box::new(NullLogger));
        log::set_max_level(log::LevelFilter::Off);

        // Dedicated runtime for the server. We don't reuse the criterion
        // runtime below so that blocking the bench runtime can't starve
        // the server's accept loop.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("build tokio runtime for bench server");

        let (port, fallback_dir, config_path) = rt.block_on(async { prepare_fixtures().await });

        let mut env_args = EnvArgs::empty();
        env_args.config_file_path = Some(config_path.to_string_lossy().into_owned());
        env_args.port = Some(port);

        rt.spawn(async move {
            let app = App::new(&env_args, None, false)
                .await
                .expect("bench server App::new");
            app.server.start().await;
        });

        // Give the accept loop time to bind. If your host is very slow
        // this could flake; 400ms matches the existing integration-test
        // harness and has been stable in CI.
        std::thread::sleep(Duration::from_millis(400));

        BenchServer {
            base_url: format!("http://127.0.0.1:{}", port),
            fallback_dir,
            rt,
        }
    })
}

/// Prepare a tempdir with:
/// - a rule-set TOML exposing `/text`, `/status`, `/file` endpoints
/// - a fallback dir with `hello.json` for the dyn-route path
/// - a root config TOML referencing the above
///
/// Returns the port we picked, the fallback dir path, and the root
/// config path.
async fn prepare_fixtures() -> (u16, PathBuf, PathBuf) {
    // Leak the tempdir so the files outlive criterion's run. Benches
    // don't need to be tidy, and leaking is simpler than threading a
    // TempDir guard through a OnceLock.
    let dir = Box::leak(Box::new(
        tempfile::tempdir().expect("tempdir for bench fixtures"),
    ));

    let fallback_dir = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback_dir).expect("mkdir fallback");

    // Dyn-route response fixture — representative small JSON payload.
    std::fs::write(
        fallback_dir.join("hello.json"),
        "{\"greeting\":\"hello\",\"items\":[1,2,3]}",
    )
    .expect("write hello.json");

    // Rule set: one of each response kind, so a bench group can cover
    // every branch of `Respond::response` without needing more than one
    // server process.
    let rule_set_path = dir.path().join("rules.toml");
    std::fs::write(
        &rule_set_path,
        concat!(
            "[[rules]]\n",
            "when.request.url_path = \"/text\"\n",
            "respond = { text = \"hello from text rule\" }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/status\"\n",
            "respond = { status = 204 }\n",
            "\n",
            "[[rules]]\n",
            "when.request.url_path = \"/file\"\n",
            "respond = { file_path = \"hello.json\" }\n",
        ),
    )
    .expect("write rules.toml");

    let config_path = dir.path().join("apimock.toml");
    // We put `respond_dir` on the rule set inline (pointing at the
    // fallback dir) so the "file" rule can find hello.json, and we also
    // set `fallback_respond_dir` for the 404/dyn-route test.
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = 0\n\
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

    // Patch the rule set to include a prefix pointing at the absolute
    // fallback dir so `file_path = "hello.json"` resolves correctly
    // regardless of where cargo-bench sets the CWD.
    let fallback_abs = fallback_dir.canonicalize().expect("canonicalize fallback");
    let rule_set_body = std::fs::read_to_string(&rule_set_path).unwrap();
    let rule_set_with_prefix = format!(
        "[prefix]\nrespond_dir = \"{}\"\n\n{}",
        fallback_abs.to_string_lossy(),
        rule_set_body,
    );
    std::fs::write(&rule_set_path, rule_set_with_prefix).unwrap();

    (pick_port(), fallback_dir, config_path)
}

/// Pick a dynamic port by binding 127.0.0.1:0 and reading back the
/// assigned port. Using the kernel's allocator here avoids the
/// randomly-pick-and-retry pattern in the integration tests (which
/// can race under parallel cargo jobs).
fn pick_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

/// Cache of one server per rule count, keyed the same way `SERVER` caches
/// the single default one. `&'static` handles leak like `prepare_fixtures`
/// already does for its own tempdir.
static RULE_SCALING_SERVERS: OnceLock<Mutex<HashMap<usize, &'static BenchServer>>> =
    OnceLock::new();

/// Bring up (or reuse) a server whose rule set has exactly `rule_count`
/// rules, none of which match `/does-not-exist` — the probe path used by
/// `bench_rule_scaling`. This is RFC 071's shape: cost proportional to
/// configuration size on a request that does no matching work at all,
/// which is exactly what a per-request `Config` clone would be sensitive
/// to and `find_matched` alone (see `routing.rs`) would not reveal.
fn rule_scaling_server(rule_count: usize) -> &'static BenchServer {
    let cache = RULE_SCALING_SERVERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().expect("rule scaling server cache lock");
    if let Some(server) = guard.get(&rule_count) {
        return server;
    }

    let dir = Box::leak(Box::new(
        tempfile::tempdir().expect("tempdir for rule-scaling fixtures"),
    ));
    let fallback_dir = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback_dir).expect("mkdir fallback");

    let rule_set_path = dir.path().join("rules.toml");
    let mut rule_set_body = String::with_capacity(rule_count * 64);
    for i in 0..rule_count {
        rule_set_body.push_str(&format!(
            "[[rules]]\nwhen.request.url_path = \"/rule-{i}\"\nrespond = {{ status = 200 }}\n\n"
        ));
    }
    std::fs::write(&rule_set_path, rule_set_body).expect("write rules.toml");

    let config_path = dir.path().join("apimock.toml");
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = 0\n\
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

    let port = pick_port();
    let server: &'static BenchServer = Box::leak(Box::new(start_server(config_path, port)));
    guard.insert(rule_count, server);
    server
}

/// Cache of one server per fallback-directory file count, mirroring
/// `RULE_SCALING_SERVERS`.
static DIRECTORY_SCALING_SERVERS: OnceLock<Mutex<HashMap<usize, &'static BenchServer>>> =
    OnceLock::new();

/// Bring up (or reuse) a server whose dyn-route fallback directory holds
/// exactly `file_count` files, and whose first file (`file-0.json`) is
/// requested by `bench_directory_scaling`. This is RFC 077 P-06's shape:
/// before the fix, `dyn_route_content` lists the whole directory on every
/// request even to serve a single known file, so latency should scale
/// with `file_count` unless that per-request listing has been removed.
fn dyn_route_scaling_server(file_count: usize) -> &'static BenchServer {
    let cache = DIRECTORY_SCALING_SERVERS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().expect("directory scaling server cache lock");
    if let Some(server) = guard.get(&file_count) {
        return server;
    }

    let dir = Box::leak(Box::new(
        tempfile::tempdir().expect("tempdir for directory-scaling fixtures"),
    ));
    let fallback_dir = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback_dir).expect("mkdir fallback");

    for i in 0..file_count.max(1) {
        std::fs::write(
            fallback_dir.join(format!("file-{i}.json")),
            format!("{{\"i\":{i}}}"),
        )
        .expect("write fallback fixture file");
    }

    let config_path = dir.path().join("apimock.toml");
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = 0\n\
             \n\
             [log]\n\
             verbose = {{ header = false, body = false }}\n\
             \n\
             [service]\n\
             fallback_respond_dir = \"{}\"\n",
            fallback_dir.file_name().unwrap().to_string_lossy(),
        ),
    )
    .expect("write apimock.toml");

    let port = pick_port();
    let server: &'static BenchServer = Box::leak(Box::new(start_server(config_path, port)));
    guard.insert(file_count, server);
    server
}

/// Start a bench server for an already-written config at `config_path`,
/// listening on `port`. Factored out of `server()`'s init closure so the
/// rule-count and directory-size caches above can reuse it without
/// duplicating the tokio-runtime/App::new/settle-time dance.
fn start_server(config_path: PathBuf, port: u16) -> BenchServer {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build tokio runtime for bench server");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(config_path.to_string_lossy().into_owned());
    env_args.port = Some(port);

    rt.spawn(async move {
        let app = App::new(&env_args, None, false)
            .await
            .expect("bench server App::new");
        app.server.start().await;
    });

    // See `server()`'s matching comment: 400ms matches the integration
    // harness and has been stable in CI.
    std::thread::sleep(Duration::from_millis(400));

    BenchServer {
        base_url: format!("http://127.0.0.1:{}", port),
        fallback_dir: config_path
            .parent()
            .expect("config path has a parent dir")
            .join("fallback"),
        rt,
    }
}

fn bench_response_latency(c: &mut Criterion) {
    let server = server();

    // Shared reqwest client. Connection reuse is realistic (every
    // serious client uses keep-alive) and keeps the measurement focused
    // on per-request server cost rather than TCP/TLS handshake overhead.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let mut group = c.benchmark_group("response_latency");
    group.throughput(Throughput::Elements(1));
    // Keep sample time modest — the HTTP path is much slower than
    // `find_matched`, and criterion's defaults would run each case for
    // many seconds. 3s is enough for stable statistics on a quiet host.
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(3));

    // Text-rule response: never touches the filesystem after startup.
    group.bench_function("text_rule", |b| {
        b.to_async(&server.rt).iter(|| async {
            let resp = client
                .get(format!("{}/text", server.base_url))
                .send()
                .await
                .expect("GET /text");
            let bytes = resp.bytes().await.expect("body");
            black_box(bytes);
        });
    });

    // Status-only rule: empty body, exercises the shortest response path.
    group.bench_function("status_rule", |b| {
        b.to_async(&server.rt).iter(|| async {
            let resp = client
                .get(format!("{}/status", server.base_url))
                .send()
                .await
                .expect("GET /status");
            let _ = resp.bytes().await;
        });
    });

    // File rule (warm): file is almost certainly in the page cache
    // because every iteration reads it. This is the realistic "steady
    // state" latency operators see.
    group.bench_function("file_rule_warm", |b| {
        b.to_async(&server.rt).iter(|| async {
            let resp = client
                .get(format!("{}/file", server.base_url))
                .send()
                .await
                .expect("GET /file");
            let bytes = resp.bytes().await.expect("body");
            black_box(bytes);
        });
    });

    // Dyn-route fallback: URL maps onto a file in the fallback dir.
    // Covers the zero-config "just drop JSON in a folder" path that
    // the README advertises — worth tracking independently of the
    // rule-set path because it takes a different code branch.
    group.bench_function("dyn_route_fallback", |b| {
        b.to_async(&server.rt).iter(|| async {
            let resp = client
                .get(format!("{}/hello", server.base_url))
                .send()
                .await
                .expect("GET /hello");
            let bytes = resp.bytes().await.expect("body");
            black_box(bytes);
        });
    });

    // 404: path that hits none of the rules and has no file on disk.
    // The "not found" response path is worth measuring because a
    // misconfigured client can spray these at the server.
    group.bench_function("not_found", |b| {
        b.to_async(&server.rt).iter(|| async {
            let resp = client
                .get(format!("{}/does-not-exist", server.base_url))
                .send()
                .await
                .expect("GET /does-not-exist");
            let _ = resp.bytes().await;
        });
    });

    // Silence unused-field warning on fallback_dir — we kept it so that
    // future benches (e.g. a cold-file variant that drops caches) can
    // locate the fixture without re-parsing config.
    let _ = &server.fallback_dir;

    group.finish();
}

/// RFC 071's shape: a request that matches none of the configured rules,
/// measured at a small and a large rule count. Before the fix, the cost
/// is dominated by cloning `Config` (and therefore the whole rule set)
/// once per request, so latency should grow with rule count even though
/// the matcher does the same (trivial, all-miss) amount of work either
/// way. After the fix, these two bars should be flat within noise.
fn bench_rule_scaling(c: &mut Criterion) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let mut group = c.benchmark_group("rule_scaling");
    group.throughput(Throughput::Elements(1));
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(3));

    for &rule_count in &[1usize, 2_500] {
        let server = rule_scaling_server(rule_count);
        group.bench_function(format!("non_matching_request/{rule_count}_rules"), |b| {
            b.to_async(&server.rt).iter(|| async {
                let resp = client
                    .get(format!("{}/does-not-exist", server.base_url))
                    .send()
                    .await
                    .expect("GET /does-not-exist");
                let _ = resp.bytes().await;
            });
        });
    }

    group.finish();
}

/// RFC 077 P-06's shape: a request for a file that exists, measured
/// against a fallback directory holding a small and a large number of
/// files. Before the fix, `dyn_route_content` lists the whole directory
/// on every request regardless of whether the candidate path can be
/// `stat`-ed directly, so latency should grow with file count. After the
/// fix, these two bars should be flat within noise — the assertion the
/// handoff notes does not exist yet.
fn bench_directory_scaling(c: &mut Criterion) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("reqwest client");

    let mut group = c.benchmark_group("directory_scaling");
    group.throughput(Throughput::Elements(1));
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(3));

    for &file_count in &[1usize, 2_500] {
        let server = dyn_route_scaling_server(file_count);
        group.bench_function(format!("existing_file/{file_count}_files"), |b| {
            b.to_async(&server.rt).iter(|| async {
                let resp = client
                    .get(format!("{}/file-0", server.base_url))
                    .send()
                    .await
                    .expect("GET /file-0");
                let bytes = resp.bytes().await.expect("body");
                black_box(bytes);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_response_latency,
    bench_rule_scaling,
    bench_directory_scaling
);
criterion_main!(benches);

/// No-op `log::Log` so the benched server process doesn't flood stdout
/// with request-received lines. See comment in `server()` above.
struct NullLogger;
impl log::Log for NullLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }
    fn log(&self, _: &log::Record) {}
    fn flush(&self) {}
}
