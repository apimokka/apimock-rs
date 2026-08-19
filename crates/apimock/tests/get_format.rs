//! RFC 055: `apimock get`'s own evidence beyond "agrees with a running
//! server" (that comparison lives in `get_agrees_with_server.rs`) —
//! `--why`'s near-miss explanation, the middleware disclosure, and the
//! `--format json` envelope's shape and provenance.

use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apimock"))
}

fn workspace_with_two_rules() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        r#"[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
[rules.when.request.body.json]
"customer.tier" = { op = "equal", value = "gold" }
[rules.respond]
text = "VIP customer order"

[[rules]]
when.request.method = "POST"
when.request.url_path = "/orders"
respond = { text = "order created", status = 201 }
"#,
    )
    .unwrap();
    dir
}

fn workspace_with_middleware() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("apimock.toml"),
        "[service]\nrule_sets = [\"rules.toml\"]\nmiddlewares = [\"mw.rhai\"]\nfallback_respond_dir = \".\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("rules.toml"),
        "[[rules]]\nwhen.request.url_path = \"/x\"\nrespond.text = \"ok\"\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("mw.rhai"), "fn handle(a, b, c) { () }\n").unwrap();
    dir
}

// ── W1/W2: correct body, status for a rule-set config ──────────────────

#[test]
fn returns_the_matched_rules_body_and_status() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args([
            "get",
            "/orders",
            "-m",
            "POST",
            "-b",
            r#"{"customer":{"tier":"gold"}}"#,
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run apimock get");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}\nstdout:\n{stdout}"));
    assert_eq!(v["result"]["response"]["status"], 200);
    assert_eq!(v["result"]["response"]["body"], "VIP customer order");
    assert_eq!(v["result"]["matched"]["rule_set_index"], 0);
    assert_eq!(v["result"]["matched"]["rule_index"], 0);
}

// ── RFC 057 handoff § 1.2: `matched` carries `rule_set_file` too, so
// its address composes with `set`'s (path, index) addressing without
// the caller needing a second `--why` round trip just to learn the
// path. Additive to RFC 055's shape — an accepted-but-unreleased
// command, so this costs nothing now and a breaking change later. ──

#[test]
fn matched_carries_rule_set_file_alongside_the_index() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args(["get", "/orders", "-m", "POST", "--format", "json"])
        .output()
        .expect("failed to run apimock get");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}\nstdout:\n{stdout}"));
    // The second rule (no body condition) matches when there's no body.
    assert_eq!(v["result"]["matched"]["rule_index"], 1);
    assert_eq!(v["result"]["matched"]["rule_set_file"], "./rules.toml");
}

// ── `--why`: names the deciding rule; a near-miss names the failing condition ──

#[test]
fn why_text_names_the_failing_condition_for_a_near_miss() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args([
            "get",
            "/orders",
            "-m",
            "POST",
            "-b",
            r#"{"customer":{"tier":"silver"}}"#,
            "--why",
        ])
        .output()
        .expect("failed to run apimock get");

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Rule #1 fails on exactly the body condition; rule #2 (no body
    // condition) is the one that actually answers.
    assert!(
        stdout.contains("body.json:customer.tier"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("\"gold\""), "stdout:\n{stdout}");
    assert!(
        stdout.contains("Answered: rule set #1, rule #2"),
        "stdout:\n{stdout}"
    );
}

#[test]
fn why_json_structures_rule_set_rule_and_conditions() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args([
            "get",
            "/orders",
            "-m",
            "POST",
            "-b",
            r#"{"customer":{"tier":"silver"}}"#,
            "--format",
            "json",
        ])
        .output()
        .expect("failed to run apimock get");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}\nstdout:\n{stdout}"));
    // --why defaults on in JSON (RFC 055 § 2 Q3) with no --why flag passed.
    let rule0 = &v["result"]["why"]["rule_sets"][0]["rules"][0];
    assert_eq!(rule0["rule_index"], 0);
    assert_eq!(rule0["matched"], false);
    let failing = rule0["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "body.json:customer.tier")
        .expect("the body condition must be present");
    assert_eq!(failing["matched"], false);
    assert_eq!(failing["actual"], "\"silver\"");
    assert!(failing["expectation"].as_str().unwrap().contains("gold"));
}

#[test]
fn why_defaults_off_in_text_and_on_in_json() {
    let dir = workspace_with_two_rules();
    let text_output = bin()
        .current_dir(dir.path())
        .args(["get", "/orders", "-m", "POST"])
        .output()
        .expect("failed to run apimock get");
    let json_output = bin()
        .current_dir(dir.path())
        .args(["get", "/orders", "-m", "POST", "--format", "json"])
        .output()
        .expect("failed to run apimock get");

    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(
        !text_stdout.contains("-- Why --"),
        "text stdout:\n{text_stdout}"
    );

    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    assert!(v["result"].get("why").is_some(), "v was: {v}");
}

// ── Middleware: not executed, and disclosed ─────────────────────────────

#[test]
fn middleware_configured_is_disclosed_in_text_and_json() {
    let dir = workspace_with_middleware();

    let text_output = bin()
        .current_dir(dir.path())
        .args(["get", "/x"])
        .output()
        .expect("failed to run apimock get");
    assert_eq!(text_output.status.code(), Some(0));
    let text_stdout = String::from_utf8_lossy(&text_output.stdout);
    assert!(
        text_stdout.contains("middleware") && text_stdout.contains("NOT simulated"),
        "stdout:\n{text_stdout}"
    );
    // The answer must still be produced — "incomplete", not withheld.
    assert!(
        text_stdout.contains("Status: 200"),
        "stdout:\n{text_stdout}"
    );

    let json_output = bin()
        .current_dir(dir.path())
        .args(["get", "/x", "--format", "json"])
        .output()
        .expect("failed to run apimock get");
    let json_stdout = String::from_utf8_lossy(&json_output.stdout);
    let v: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    assert_eq!(v["result"]["middleware"]["configured"], 1);
    assert_eq!(v["result"]["middleware"]["simulated"], false);
}

#[test]
fn no_middleware_configured_omits_the_field_entirely() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args(["get", "/orders", "-m", "POST", "--format", "json"])
        .output()
        .expect("failed to run apimock get");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(v["result"].get("middleware").is_none(), "v was: {v}");
}

// ── `--format json`: envelope shape and provenance ──────────────────────

#[test]
fn format_json_emits_a_valid_envelope_with_absolute_provenance() {
    let dir = workspace_with_two_rules();
    let output = bin()
        .current_dir(dir.path())
        .args(["get", "/orders", "-m", "POST", "--format", "json"])
        .output()
        .expect("failed to run apimock get");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("{e}\nstdout:\n{stdout}"));
    assert!(v.is_object());
    assert_eq!(v["schema"], 1);
    assert!(v["apimock"].is_string());
    assert!(v.get("result").is_some());
    assert!(v.get("error").is_none());

    let config_path = v["result"]["source"]["config"].as_str().unwrap();
    assert!(
        std::path::Path::new(config_path).is_absolute(),
        "config path was not absolute: {config_path}"
    );
    let rule_set_path = v["result"]["source"]["rule_sets"][0].as_str().unwrap();
    assert!(
        std::path::Path::new(rule_set_path).is_absolute(),
        "rule set path was not absolute: {rule_set_path}"
    );
}

// ── Usage errors ─────────────────────────────────────────────────────────

#[test]
fn missing_path_is_a_usage_error() {
    let output = bin()
        .args(["get", "-m", "POST"])
        .output()
        .expect("failed to run apimock get");
    assert_eq!(output.status.code(), Some(2));
}
