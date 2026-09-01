use std::time::Duration;

use hyper::StatusCode;

use apimock::{App, EnvArgs};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    constant::root_config_dir::listener::TLS,
    util::{
        http::{test_request::TestRequest, test_response::response_body_str},
        test_setup::TestSetup,
        tls::{cert_file_path, generate_tls_credentials, key_file_path, tls_credentials_are_ready},
    },
};

#[tokio::test]
async fn tls_server_tls_client() {
    let port = tls_setup().await;
    let response = TestRequest::default("/", port).with_https().send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
}

#[tokio::test]
#[should_panic = "hyper_util::client::legacy::Error(Connect, Custom { kind: Other, error: Custom { kind: InvalidData, error: InvalidMessage(InvalidContentType) } })"]
async fn nontls_server_tls_client() {
    let port = default_setup().await;
    let _ = TestRequest::default("/", port).with_https().send().await;
}

#[tokio::test]
#[should_panic = "hyper::Error(Parse(Version)"]
async fn tls_server_nontls_client() {
    let port = tls_setup().await;
    let _ = TestRequest::default("/", port).send().await;
}

#[tokio::test]
async fn nontls_server_nontls_client() {
    let port = default_setup().await;
    let response = TestRequest::default("/", port).send().await;

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body_str = response_body_str(response).await;
    assert_eq!(body_str.as_str(), "{\"hello\":\"index\"}");
}

/// internal setup fn on https support config
async fn tls_setup() -> u16 {
    let test_setup = TestSetup::default_with_root_config_dir(TLS);
    test_setup.launch().await
}

/// internal setup fn on default config
async fn default_setup() -> u16 {
    let test_setup = TestSetup::default();
    test_setup.launch().await
}

/// RFC 074 S-08 — a TLS setup failure must be fatal at startup, and
/// must not leave any listener bound, HTTP included.
///
/// Reproduces the defect first: **before** this fix, `Server::new`
/// never touched the cert/key files at all — `bind_https` loaded and
/// parsed them lazily, the first time `.start()` (or this test's own
/// direct call, pre-fix) reached it, and a parse failure there was
/// swallowed by `https_start`'s `log::error!` + return, while a
/// separately-configured HTTP listener kept serving regardless. That
/// is the silent HTTP-only degradation this RFC exists to close: an
/// operator who configured HTTPS and got HTTP instead, with no
/// indication anything was wrong.
#[tokio::test]
async fn malformed_tls_material_fails_startup_and_binds_no_listener() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Deliberately not a PEM at all — `TlsConfig::validate` (existence
    // only) passes; `apimock_server::tls::load_certs` must be the one
    // to reject it.
    let cert_path = dir.path().join("malformed_cert.pem");
    let key_path = dir.path().join("malformed_key.pem");
    std::fs::write(&cert_path, b"not a certificate\n").expect("write malformed cert");
    std::fs::write(&key_path, b"not a private key\n").expect("write malformed key");

    // Separate HTTP and HTTPS ports (`[listener.tls].port` distinct from
    // `[listener].port`) — the scenario RFC 074's Motivation describes:
    // an HTTP listener that would otherwise start and keep serving
    // while HTTPS silently fails to.
    let http_port = free_tcp_port().await;
    let https_port = free_tcp_port().await;

    let cert_path_toml = toml_safe_path(&cert_path);
    let key_path_toml = toml_safe_path(&key_path);

    let toml_path = dir.path().join("apimock.toml");
    std::fs::write(
        &toml_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {http_port}\n\
             [listener.tls]\n\
             cert = \"{cert_path_toml}\"\n\
             key = \"{key_path_toml}\"\n\
             port = {https_port}\n\
             [service]\n\
             rule_sets = []\n\
             fallback_respond_dir = \".\"\n"
        ),
    )
    .expect("write apimock.toml");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(toml_path.to_string_lossy().into_owned());
    env_args.port = Some(http_port);

    let result = App::new(&env_args, None, true).await;

    let err = match result {
        Ok(_) => {
            panic!("App::new must fail on malformed TLS material, not start up serving HTTP only")
        }
        Err(err) => err,
    };
    let message = err.to_string();
    assert!(
        message.contains(&cert_path_toml),
        "error must name the malformed file: {message}"
    );

    // The HTTP port must still be free: `App::new` failed before
    // `main`'s `app.server.start().await` — the only place `bind_http`
    // is ever called outside a test harness — could run.
    let http_still_free = TcpListener::bind(("127.0.0.1", http_port)).await;
    assert!(
        http_still_free.is_ok(),
        "HTTP port {http_port} must still be free after the failed startup, \
         proving no HTTP listener was bound: {:?}",
        http_still_free.err()
    );
}

/// A path string safe to embed inside a TOML double-quoted string.
///
/// `path.to_string_lossy()` alone is **not** safe on Windows: a native
/// path there is backslash-separated, and TOML's basic-string escape
/// rules treat `\` as the start of an escape sequence — `\A`, `\U`
/// (followed by non-hex), etc. are invalid escapes, so an
/// absolute Windows path written in verbatim fails to parse (confirmed
/// on CI: `windows-latest` failed exactly this way before this helper
/// existed). Forward slashes work as a path separator on Windows too
/// (both `std::path` and this project's own TLS/config file loading
/// just hand the string to the OS), so normalising to `/` sidesteps
/// TOML escaping entirely rather than escaping every backslash.
fn toml_safe_path(path: &std::path::Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Bind an ephemeral TCP listener, read back its port, then drop it —
/// yields a port that was free at the moment of the call, without
/// hardcoding a literal that could collide with another test running
/// concurrently in this same binary.
async fn free_tcp_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    listener.local_addr().expect("local_addr").port()
}

/// Start an HTTPS listener with the given `[listener.tls]`
/// `handshake_timeout_seconds` / `max_connections`, using the shared
/// generated test cert/key (same fixture every other TLS test in this
/// binary uses). Returns the bound port; the server runs for the rest
/// of the test process (this binary's own convention — see
/// `TestSetup::launch`, which never tears a server down either).
async fn launch_https_with_s07_settings(
    handshake_timeout_seconds: u64,
    max_connections: usize,
) -> u16 {
    if !tls_credentials_are_ready() {
        generate_tls_credentials();
    }

    let port = free_tcp_port().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let toml_path = dir.path().join("apimock.toml");
    std::fs::write(
        &toml_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {port}\n\
             [listener.tls]\n\
             cert = \"{cert}\"\n\
             key = \"{key}\"\n\
             handshake_timeout_seconds = {handshake_timeout_seconds}\n\
             max_connections = {max_connections}\n\
             [service]\n\
             rule_sets = []\n\
             fallback_respond_dir = \".\"\n",
            cert = toml_safe_path(&cert_file_path()),
            key = toml_safe_path(&key_file_path()),
        ),
    )
    .expect("write apimock.toml");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(toml_path.to_string_lossy().into_owned());

    let app = App::new(&env_args, None, true)
        .await
        .expect("App::new for S-07 test fixture");
    let (listener, acceptor) = app
        .server
        .bind_https()
        .await
        .expect("bind_https")
        .expect("https listener configured");
    tokio::spawn(async move {
        app.server.serve_https(listener, acceptor).await;
    });

    // The listener is already bound synchronously above; this just
    // gives the spawned `serve_https` task a moment to reach its first
    // `accept().await` before the test starts dialing it.
    tokio::time::sleep(Duration::from_millis(50)).await;

    port
}

/// RFC 074 S-07 — an incomplete TLS handshake is dropped after the
/// configured timeout, **and the server serves other connections
/// throughout** — the second half is the actual finding; the first
/// alone would also pass with a per-connection deadlock, since nothing
/// would be there to time out a different way.
#[tokio::test]
async fn incomplete_handshake_is_dropped_after_timeout_and_server_keeps_serving() {
    let port = launch_https_with_s07_settings(1, 256).await;

    // A client that opens the connection and sends nothing at all —
    // never completes (or even starts) the TLS handshake.
    let stalled = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("open stalled connection");

    // While that connection sits open, a normal request against the
    // same listener must still complete the TLS handshake and get a
    // real HTTP response — proving the stalled handshake doesn't block
    // anything else (each connection already gets its own task; this
    // is the regression guard for that staying true). The response's
    // own status doesn't matter here — this fixture has no rule sets —
    // only that the round trip completes at all rather than hanging.
    let _response = TestRequest::default("/", port).with_https().send().await;

    // Past the configured 1s timeout, the server must have dropped the
    // stalled connection — proven by reading from it and seeing the
    // peer close (0 bytes) rather than it hanging forever.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let mut buf = [0u8; 1];
    let read = tokio::time::timeout(Duration::from_secs(2), async {
        use tokio::io::AsyncReadExt;
        let mut stalled = stalled;
        stalled.read(&mut buf).await
    })
    .await
    .expect("server never closed the timed-out connection");
    assert_eq!(
        read.expect("read from closed connection should not error"),
        0,
        "server should have closed the stalled connection after the handshake timeout"
    );
}

/// RFC 074 S-07 — the connection cap is reached and the server
/// recovers once a connection closes, rather than needing a restart.
#[tokio::test]
async fn connection_cap_delays_new_connections_until_a_slot_frees() {
    // A long handshake timeout so this test controls exactly when the
    // stalled connection releases its slot (by closing it), rather
    // than racing the timeout.
    let port = launch_https_with_s07_settings(30, 1).await;

    // Take the sole slot with a connection that never completes its
    // handshake.
    let stalled = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("open stalled connection");

    // A second, well-behaved request must NOT complete while the sole
    // slot is held — proving new connections wait rather than being
    // served immediately regardless of the cap.
    let blocked = tokio::time::timeout(
        Duration::from_millis(500),
        TestRequest::default("/", port).with_https().send(),
    )
    .await;
    assert!(
        blocked.is_err(),
        "a second connection must wait while the sole connection slot is held, not be served \
         immediately"
    );

    // Free the slot: closing the stalled connection makes the
    // server's `acceptor.accept()` on it return (EOF), which drops its
    // permit.
    drop(stalled);

    // The server must recover: the same request now completes, well
    // within a generous bound — no restart needed.
    let _recovered = tokio::time::timeout(
        Duration::from_secs(5),
        TestRequest::default("/", port).with_https().send(),
    )
    .await
    .expect("server did not recover after the connection slot freed");
}
