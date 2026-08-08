//! Verifies `crates/apimock/examples/secure-with-tls/README.md`.
//!
//! `listener.tls.{cert,key}` are resolved against the process's
//! current directory, not the config file's own directory (unlike
//! `rule_sets`/`fallback_respond_dir`, which resolve relative to the
//! config file - see `Config::current_dir_to_parent_dir_relative_path`
//! vs `TlsConfig::validate`). `TestSetup::launch` never changes the
//! test binary's own working directory (by design - it's shared,
//! parallel-test-unsafe global state), so this example is run as a
//! real child process instead, via `Command::current_dir`, which only
//! affects that child - safe alongside every other test in this
//! binary running concurrently in the same process.
//!
//! This also means the example keeps its hardcoded port (3443, unique
//! among this crate's examples) rather than the dynamic port every
//! other example test gets from `TestSetup`.

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};

use crate::util::http::{test_request::TestRequest, test_response::response_body_str};

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn health_over_https() {
    let example_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/secure-with-tls");

    let child = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .current_dir(&example_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn apimock for the secure-with-tls example");
    let _guard = ChildGuard(child);

    tokio::time::sleep(Duration::from_millis(600)).await;

    let response = TestRequest::default("/health", 3443)
        .with_https()
        .send()
        .await;
    assert_eq!(response_body_str(response).await, "ok, over https");
}
