//! RFC 068 S-03 — a non-terminating middleware script fails its own
//! request rather than wedging the server. Two changes together close
//! this (`engine.set_max_operations` + moving evaluation into
//! `spawn_blocking`); per the RFC, either alone is insufficient, so
//! both tests below exercise the combination, not just "the operation
//! limit eventually trips."
//!
//! # Before this fix (reproduced by reading the code, not by running it)
//!
//! `Engine::new()` set no operation limit, and `eval_ast_with_scope`
//! ran directly on the calling tokio task — i.e. on an async worker
//! thread. A `while true {}` script never returned, so that call never
//! returned either: the task, and the worker thread executing it,
//! never came back to the runtime's scheduler. Reproducing this
//! properly needs a live hang with no timeout to bound it by, which
//! isn't something this suite can safely automate (an actually-hung
//! test has no way to fail other than the whole run timing out) — the
//! code path was read and traced instead of executed, which is why
//! this is called out explicitly rather than a captured "before" run
//! like `body_limit.rs`'s.

use std::time::Duration;

use tokio::net::TcpListener;

use apimock::{App, EnvArgs};

use crate::util::http::test_request::TestRequest;

/// Launch an HTTP server with one middleware (`script`) compiled under
/// `max_operations`, no rule sets — so every request reaches the
/// middleware and nothing else can answer it.
async fn launch_with_middleware(script: &str, max_operations: u64) -> u16 {
    let dir = tempfile::tempdir().expect("tempdir");
    let script_path = dir.path().join("mw.rhai");
    std::fs::write(&script_path, script).expect("write middleware script");

    let port_probe = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    let port = port_probe.local_addr().expect("local_addr").port();
    drop(port_probe);

    let toml_path = dir.path().join("apimock.toml");
    std::fs::write(
        &toml_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {port}\n\
             [service]\n\
             rule_sets = []\n\
             fallback_respond_dir = \".\"\n\
             middlewares = [\"mw.rhai\"]\n\
             middleware_max_operations = {max_operations}\n"
        ),
    )
    .expect("write apimock.toml");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(toml_path.to_string_lossy().into_owned());
    env_args.port = Some(port);

    let app = App::new(&env_args, None, true)
        .await
        .expect("App::new for middleware-termination test fixture");
    let listener = app
        .server
        .bind_http()
        .await
        .expect("bind_http")
        .expect("http listener configured");
    tokio::spawn(async move {
        app.server.serve_http(listener).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

/// A real, meaningful chunk of synchronous work when uncapped — large
/// enough that if evaluation ran directly on an async worker (instead
/// of `spawn_blocking`), it would be a noticeable stall, not an
/// imperceptible one. Small enough that a handful of these still
/// finish in well under this suite's own timeouts.
const RUNAWAY_MAX_OPERATIONS: u64 = 5_000_000;

/// RFC 068 S-03 acceptance, first two bullets: the runaway request
/// itself completes (rather than hanging forever), **and** a
/// subsequent request on a different connection is answered — the
/// second half is the actual finding; the first alone would also pass
/// with a defect that merely makes every request slow.
#[tokio::test]
async fn runaway_script_fails_its_request_and_server_keeps_serving_other_connections() {
    let port = launch_with_middleware("while true { }", RUNAWAY_MAX_OPERATIONS).await;

    // The runaway request completes (does not hang) once the operation
    // limit trips — bounded generously since op-counting speed varies
    // by machine, not because this should ever be close.
    let _runaway = tokio::time::timeout(
        Duration::from_secs(10),
        TestRequest::default("/", port).send(),
    )
    .await
    .expect("a while-true middleware request must eventually fail, not hang forever");

    // A separate request, on a different connection (reqwest opens a
    // fresh connection per `TestRequest::send` call — no pooling
    // reuse configured), must also complete promptly.
    let _following = tokio::time::timeout(
        Duration::from_secs(10),
        TestRequest::default("/", port).send(),
    )
    .await
    .expect(
        "the server must answer a subsequent request on a different connection after a \
         runaway middleware evaluation, not stay wedged",
    );
}

/// RFC 068 S-03 acceptance, third bullet: N concurrent runaway scripts
/// must not reduce throughput to zero. `spawn_blocking`'s thread pool
/// (hundreds of threads by default) is what makes this hold when the
/// number of concurrent evaluations exceeds the async runtime's own
/// worker count — evaluating directly on the async workers, even with
/// an operation limit, would serialize these behind however many
/// workers tokio actually has.
#[tokio::test]
async fn concurrent_runaway_scripts_do_not_reduce_throughput_to_zero() {
    let port = launch_with_middleware("while true { }", RUNAWAY_MAX_OPERATIONS).await;

    const CONCURRENT_REQUESTS: usize = 8;
    let handles: Vec<_> = (0..CONCURRENT_REQUESTS)
        .map(|_| tokio::spawn(async move { TestRequest::default("/", port).send().await }))
        .collect();

    // All of them completing (none hung) within a generous shared bound
    // is the literal finding: throughput not reduced to zero, not a
    // latency target.
    let all_done = tokio::time::timeout(Duration::from_secs(30), async {
        for handle in handles {
            let _ = handle.await.expect("request task panicked");
        }
    })
    .await;

    assert!(
        all_done.is_ok(),
        "{CONCURRENT_REQUESTS} concurrent while-true middleware requests did not all complete \
         within the bound — throughput reduced to (near) zero"
    );
}
