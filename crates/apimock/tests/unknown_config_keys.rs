//! RFC 069 — an unknown key in a rule, condition, or respond block
//! fails to load, rather than being silently discarded.
//!
//! # Before this fix (reproduced live, not by this suite)
//!
//! ```text
//! $ apimock validate -c ./cfg.toml
//! Validation passed (1 rules across 1 rule set(s)).
//!
//! $ curl http://127.0.0.1:PORT/secret          # no header at all
//! SENSITIVE
//! ```
//! `headerz` instead of `headers` on a rule with a `url_path` — the
//! header condition vanished silently, `validate` endorsed the broken
//! config, and the rule served everything. Exactly RFC 069's own
//! Motivation, reproduced against this branch's own baseline commit
//! before writing the fix below.

use std::process::Command;

use apimock::{App, EnvArgs};
use tokio::net::TcpListener;

/// Run the real `apimock` binary and capture its exit code + stderr —
/// for asserting on the exact text `apimock validate` prints, which
/// only exists in the CLI layer (`crates/apimock/src/cmd/validate.rs`'s
/// near-match hint), not in the library error types themselves.
fn run_validate(config_path: &std::path::Path) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_apimock"))
        .args(["validate", "-c", config_path.to_str().unwrap()])
        .output()
        .expect("failed to run apimock validate");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Write the RFC's own reproduction: a rule gating `/secret` on an
/// `x-token` header, but with `headerz` in place of `headers`. Returns
/// the config file's path.
fn write_typo_config(dir: &std::path::Path, http_port: u16) -> std::path::PathBuf {
    let rules_path = dir.join("rules.toml");
    std::fs::write(
        &rules_path,
        "[[rules]]\n\
         when.request.url_path = \"/secret\"\n\
         [rules.when.request.headerz]\n\
         x-token = { value = \"expected\" }\n\
         respond.text = \"SENSITIVE\"\n",
    )
    .expect("write rule-set file");

    let config_path = dir.join("apimock.toml");
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {http_port}\n\
             [service]\n\
             rule_sets = [\"rules.toml\"]\n\
             fallback_respond_dir = \".\"\n"
        ),
    )
    .expect("write apimock.toml");

    config_path
}

async fn free_tcp_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind ephemeral port");
    listener.local_addr().expect("local_addr").port()
}

/// RFC 069 acceptance: "A mistyped condition key fails validate and
/// fails to load, naming the key" + "the near-match suggestion fires
/// for headerz -> headers".
#[tokio::test]
async fn mistyped_header_key_fails_validate_naming_the_key_with_a_suggestion() {
    let dir = tempfile::tempdir().expect("tempdir");
    let port = free_tcp_port().await;
    let config_path = write_typo_config(dir.path(), port);

    let (code, stderr) = run_validate(&config_path);

    assert_eq!(
        code, 2,
        "a config with an unknown key must fail to load: {stderr}"
    );
    assert!(
        stderr.contains("unknown field `headerz`"),
        "must name the offending key: {stderr}"
    );
    assert!(
        stderr.contains("did you mean `headers`"),
        "must suggest the near-match correction: {stderr}"
    );
}

/// RFC 069 acceptance, the end-to-end scenario: after the fix, the
/// `/secret` rule above must not serve to an unconditioned request —
/// asserted on served behaviour, not only on the loader. Since a
/// `deny_unknown_fields` violation now fails the *whole config* to
/// load, the strongest and most literal version of "does not serve to
/// an unconditioned request" is that **the server never starts at
/// all** — proven the same way tranche 1's RFC 074 S-08 end-to-end test
/// proved a TLS failure leaves no listener bound: `App::new` fails, and
/// the configured port is provably still free afterward, not merely
/// assumed to be.
#[tokio::test]
async fn end_to_end_broken_config_never_starts_a_server_to_leak_the_secret() {
    let dir = tempfile::tempdir().expect("tempdir");
    let http_port = free_tcp_port().await;
    let config_path = write_typo_config(dir.path(), http_port);

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(config_path.to_string_lossy().into_owned());
    env_args.port = Some(http_port);

    let result = App::new(&env_args, None, true).await;
    assert!(
        result.is_err(),
        "App::new must fail on a config with an unknown key, not start up serving it"
    );

    let still_free = TcpListener::bind(("127.0.0.1", http_port)).await;
    assert!(
        still_free.is_ok(),
        "the configured port must still be free after the failed startup — proving no listener \
         is serving the /secret rule to an unconditioned request: {:?}",
        still_free.err()
    );
}

/// RFC 069 acceptance: "A valid config with every documented key still
/// loads." Exercises every rule-facing field this RFC put
/// `deny_unknown_fields` on, in one rule set — `Rule` (weight,
/// priority), `When`/`Request` (url_path detailed form, method,
/// headers with every operator shape, body), `Respond` (every field),
/// plus rule-set-level `[prefix]`/`[default]`/`[guard]`. If any of
/// these was accidentally left off `deny_unknown_fields`'s companion
/// field list (a typo in the struct definition itself, not the config),
/// this fails to load and says so specifically — the regression guard
/// for the fix's own correctness, not just its strictness.
#[tokio::test]
async fn valid_config_using_every_documented_key_still_loads() {
    let dir = tempfile::tempdir().expect("tempdir");
    let http_port = free_tcp_port().await;

    let respond_dir = dir.path().join("responses");
    std::fs::create_dir(&respond_dir).expect("mkdir responses");
    std::fs::write(respond_dir.join("hello.json"), "{}").expect("write hello.json");

    let rules_path = dir.path().join("rules.toml");
    std::fs::write(
        &rules_path,
        "[prefix]\n\
         url_path = \"/api\"\n\
         respond_dir = \"responses\"\n\
         [default]\n\
         delay_response_milliseconds = 5\n\
         [guard]\n\
         \n\
         [[rules]]\n\
         weight = 2\n\
         priority = 1\n\
         when.request.url_path = { op = \"starts_with\", value = \"/widgets\" }\n\
         when.request.method = \"POST\"\n\
         [rules.when.request.headers]\n\
         x-token = { op = \"equal\", value = \"expected\" }\n\
         x-optional = { op = \"exists\" }\n\
         x-missing = { op = \"absent\" }\n\
         [rules.when.request.body.json]\n\
         \"customer.tier\" = { op = \"equal\", value = \"gold\" }\n\
         [rules.respond]\n\
         file_path = \"hello.json\"\n\
         csv_records_key = \"records\"\n\
         delay_response_milliseconds = 1\n\
         [rules.respond.headers]\n\
         x-custom = \"yes\"\n\
         \n\
         [[rules]]\n\
         when.request.url_path = \"/status\"\n\
         [rules.respond]\n\
         json = \"{\\\"ok\\\":true}\"\n\
         status = 200\n",
    )
    .expect("write rule-set file");

    let config_path = dir.path().join("apimock.toml");
    std::fs::write(
        &config_path,
        format!(
            "[listener]\n\
             ip_address = \"127.0.0.1\"\n\
             port = {http_port}\n\
             [service]\n\
             strategy = \"first_match\"\n\
             rule_sets = [\"rules.toml\"]\n\
             fallback_respond_dir = \".\"\n"
        ),
    )
    .expect("write apimock.toml");

    let mut env_args = EnvArgs::empty();
    env_args.config_file_path = Some(config_path.to_string_lossy().into_owned());
    env_args.port = Some(http_port);

    let result = App::new(&env_args, None, true).await;
    assert!(
        result.is_ok(),
        "a valid config using every documented rule-facing key must still load: {:?}",
        result.err()
    );
}
