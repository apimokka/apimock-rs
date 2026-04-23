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

use std::hint::black_box;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use apimock::core::{app::App, args::EnvArgs};
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

        let env_args = EnvArgs {
            config_file_path: Some(config_path.to_string_lossy().into_owned()),
            port: Some(port),
            fallback_respond_dir_path: None,
        };

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
            rule_set_path
                .file_name()
                .unwrap()
                .to_string_lossy(),
            fallback_dir
                .file_name()
                .unwrap()
                .to_string_lossy(),
        ),
    )
    .expect("write apimock.toml");

    // Patch the rule set to include a prefix pointing at the absolute
    // fallback dir so `file_path = "hello.json"` resolves correctly
    // regardless of where cargo-bench sets the CWD.
    let fallback_abs = fallback_dir
        .canonicalize()
        .expect("canonicalize fallback");
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
    let port = listener
        .local_addr()
        .expect("local_addr")
        .port();
    drop(listener);
    port
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

criterion_group!(benches, bench_response_latency);
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
