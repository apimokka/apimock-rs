//! Shared test helpers used across the workspace test modules.

use std::fs;
use std::path::{Path, PathBuf};

/// Create a minimal on-disk workspace and return the tempdir guard
/// + absolute path to the root `apimock.toml`.
pub(super) fn make_workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");

    let rule_set_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/users\"\n",
        "respond = { text = \"ok\" }\n",
        "\n",
        "[[rules]]\n",
        "when.request.url_path = \"/api/health\"\n",
        "respond = { status = 204 }\n",
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    fs::write(&rs_path, rule_set_toml).unwrap();

    let fallback = dir.path().join("fallback");
    fs::create_dir_all(&fallback).unwrap();

    let root_toml = format!(
        "[listener]\n\
         ip_address = \"127.0.0.1\"\n\
         port = 3001\n\
         \n\
         [service]\n\
         rule_sets = [\"{}\"]\n\
         fallback_respond_dir = \"{}\"\n",
        rs_path.file_name().unwrap().to_string_lossy(),
        fallback.file_name().unwrap().to_string_lossy(),
    );
    let root_path = dir.path().join("apimock.toml");
    fs::write(&root_path, root_toml).unwrap();

    (dir, root_path)
}

/// Create a workspace whose first rule has both header and body conditions.
pub(super) fn make_workspace_with_headers_and_body() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();

    let rs_toml = concat!(
        "[[rules]]\n",
        "when.request.url_path = \"/api/protected\"\n",
        "when.request.headers.x-api-key = { value = \"shh\" }\n",
        "when.request.body.json.\"action\" = { op = \"equal\", value = \"go\" }\n",
        "respond = { text = \"ok\" }\n",
    );
    let rs_path = dir.path().join("apimock-rule-set.toml");
    std::fs::write(&rs_path, rs_toml).unwrap();

    let root_toml = format!(
        "[listener]\n\
         ip_address = \"127.0.0.1\"\n\
         port = 3001\n\
         \n\
         [service]\n\
         rule_sets = [\"{}\"]\n\
         fallback_respond_dir = \"{}\"\n",
        rs_path.file_name().unwrap().to_string_lossy(),
        fallback.file_name().unwrap().to_string_lossy(),
    );
    let root_path = dir.path().join("apimock.toml");
    std::fs::write(&root_path, root_toml).unwrap();
    (dir, root_path)
}

/// Write an `apimock.toml` to `dir` with the given fallback dir name and
/// optional extra TOML content appended after the `[service]` section.
pub(super) fn write_apimock_toml(dir: &Path, fallback_name: &str, extra: &str) -> PathBuf {
    let content = format!(
        "[listener]\nip_address = \"127.0.0.1\"\nport = 3002\n\
         [service]\nrule_sets = [\"apimock-rule-set.toml\"]\n\
         fallback_respond_dir = \"{}\"\n{}",
        fallback_name, extra
    );
    let p = dir.join("apimock.toml");
    std::fs::write(&p, content).unwrap();
    p
}
