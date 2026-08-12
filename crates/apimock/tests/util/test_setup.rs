use std::{env, net::SocketAddr, path::Path, time::Duration};

use apimock::{App, EnvArgs};
use tokio::net::TcpStream;

use super::{
    constant::{CONFIG_FILE_NAME, CONFIG_TESTS_ROOT_DIR_PATH},
    tls::{generate_tls_credentials, tls_credentials_are_ready},
};

/// Bound on how long `launch` waits for the server to accept connections
/// before giving up. Generous on purpose - a loaded CI runner is the
/// case this exists for, not the common case.
const READINESS_TIMEOUT: Duration = Duration::from_secs(5);
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone)]
pub struct TestSetup {
    pub root_config_file_path: Option<String>,
    pub port: Option<u16>,
    pub fallback_respond_dir_path: Option<String>,
    /// bound to set_current_dir(). **caution:** affects globally
    pub current_dir_path: Option<String>,
}

impl TestSetup {
    /// generate setup args with specific dir where root config file is located
    pub fn default_with_root_config_dir(root_config_dir_path: &str) -> Self {
        Self {
            root_config_file_path: Some(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(CONFIG_TESTS_ROOT_DIR_PATH)
                    .join(root_config_dir_path)
                    .join(CONFIG_FILE_NAME)
                    .to_str()
                    .expect("failed to generate root config file path")
                    .to_string(),
            ),
            ..Default::default()
        }
    }

    /// test initial setup with dynamic port selected
    ///
    /// # No probe-then-drop
    ///
    /// `self.port` is honoured verbatim when set (some tests assert on a
    /// specific literal port). Otherwise the requested port is `0`: the
    /// OS assigns a free one at bind time, inside `launch_impl`, and
    /// that exact listener - never released, never rebound - is what the
    /// server then serves on. There is no separate "pick a port" step
    /// whose result could go stale before the server binds, which is
    /// what made the old `dynamic_port()` racy (RFC 046).
    pub async fn launch(&self) -> u16 {
        if !tls_credentials_are_ready() {
            generate_tls_credentials();
        }

        let requested_port = self.port.unwrap_or(0);
        self.launch_impl(requested_port).await
    }

    /// test initial setup: start up mock server. Returns the port
    /// actually bound (which equals `port` unless `port` was `0`).
    async fn launch_impl(&self, port: u16) -> u16 {
        if let Some(current_dir_path) = self.current_dir_path.as_ref() {
            let current_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(CONFIG_TESTS_ROOT_DIR_PATH)
                .join(current_dir_path.as_str());

            match env::set_current_dir(current_dir.clone()) {
                Ok(_) => (),
                Err(err) => {
                    panic!(
                        "failed to set current dir: {} ({})",
                        current_dir.to_string_lossy(),
                        err
                    );
                }
            };
        }

        let mut app_env_args = env_args(port);

        if let Some(root_config_file_path) = self.root_config_file_path.as_ref() {
            app_env_args.config_file_path = Some(root_config_file_path.to_owned());
        }

        if let Some(fallback_respond_dir_path) = self.fallback_respond_dir_path.as_ref() {
            app_env_args.fallback_respond_dir_path = Some(fallback_respond_dir_path.to_owned());
        }

        let app = App::new(&app_env_args, None, true)
            .await
            .expect("App::new failed in test setup");

        // Bind before spawning. A bind failure (or, for HTTPS, a failure
        // anywhere in the TLS setup `bind_https` does before it binds)
        // surfaces right here, synchronously, in the test's own task -
        // not inside a spawned task whose result nothing awaited, which
        // is how a bind failure used to reach the first request as an
        // unexplained connection error instead of a message naming the
        // cause (RFC 046 Defect 2).
        if let Some(listener) = app
            .server
            .bind_http()
            .await
            .expect("failed to bind HTTP listener in test setup")
        {
            let bound_addr = listener
                .local_addr()
                .expect("bound HTTP listener has no local_addr");

            tokio::spawn(async move {
                app.server.serve_http(listener).await;
            });

            wait_until_accepting(bound_addr).await;
            return bound_addr.port();
        }

        if let Some((listener, acceptor)) = app
            .server
            .bind_https()
            .await
            .expect("failed to bind HTTPS listener in test setup")
        {
            let bound_addr = listener
                .local_addr()
                .expect("bound HTTPS listener has no local_addr");

            tokio::spawn(async move {
                app.server.serve_https(listener, acceptor).await;
            });

            wait_until_accepting(bound_addr).await;
            return bound_addr.port();
        }

        panic!(
            "test config at {:?} configures neither an HTTP nor an HTTPS listener",
            self.root_config_file_path
        );
    }
}

impl Default for TestSetup {
    fn default() -> Self {
        Self {
            root_config_file_path: Some(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(CONFIG_TESTS_ROOT_DIR_PATH)
                    .join(CONFIG_FILE_NAME)
                    .to_str()
                    .expect("failed to generate root config file path")
                    .to_string(),
            ),
            port: None,
            fallback_respond_dir_path: None,
            current_dir_path: None,
        }
    }
}

/// Poll `addr` until a TCP connection succeeds, or panic naming the
/// address once `READINESS_TIMEOUT` elapses.
///
/// # Why not connect to `addr` for a wildcard bind
///
/// `[listener] ip_address = "0.0.0.0"` / `"[::]"` are bind targets, not
/// connect targets - a socket cannot dial the unspecified address. Where
/// `addr.ip()` is unspecified, the loopback equivalent of the same
/// family is used instead, since loopback is always one of the
/// interfaces such a listener accepts on. Every other address - notably
/// a specific IPv6 address, whether loopback or a real interface - is
/// connected to exactly as bound. A previous attempt at this fix
/// hardcoded `127.0.0.1` here unconditionally and regressed every IPv6
/// bound-address test; this is the reason that hardcoding is gone.
///
/// # Expected log noise on HTTPS fixtures
///
/// This probe only needs the TCP handshake to succeed, so it connects
/// and drops the stream immediately - it never sends a TLS ClientHello.
/// For an HTTPS listener, the server's per-connection task then sees the
/// dropped connection mid-handshake and logs `TLS handshake failed: ...
/// eof`. That is this probe, not a defect; it happens once per HTTPS
/// `TestSetup::launch()` call and does not affect the accept loop or the
/// test that follows.
async fn wait_until_accepting(addr: SocketAddr) {
    let connect_addr = if addr.ip().is_unspecified() {
        let loopback = if addr.is_ipv6() {
            std::net::Ipv6Addr::LOCALHOST.into()
        } else {
            std::net::Ipv4Addr::LOCALHOST.into()
        };
        SocketAddr::new(loopback, addr.port())
    } else {
        addr
    };

    let deadline = tokio::time::Instant::now() + READINESS_TIMEOUT;
    loop {
        if TcpStream::connect(connect_addr).await.is_ok() {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!(
                "server did not start accepting connections on {} within {:?}",
                connect_addr, READINESS_TIMEOUT
            );
        }
        tokio::time::sleep(READINESS_POLL_INTERVAL).await;
    }
}

/// env args for testing
fn env_args(port: u16) -> EnvArgs {
    let mut ret = EnvArgs::default()
        .expect("failed to parse env args")
        .expect("no env args returned (unexpected --init short-circuit in tests)");

    ret.port = Some(port);

    ret.validate().expect("env args validation failed");
    ret
}
