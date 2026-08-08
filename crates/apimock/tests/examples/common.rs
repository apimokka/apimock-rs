//! Shared setup for the RFC 036 example-verification tests.

use std::path::Path;

use crate::util::test_setup::TestSetup;

/// Absolute path to `crates/apimock/examples/<set_name>/apimock.toml`.
pub fn example_config_path(set_name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(set_name)
        .join("apimock.toml")
        .to_str()
        .expect("example config path is valid UTF-8")
        .to_owned()
}

/// A `TestSetup` pointed at an example set's own `apimock.toml`.
///
/// Unlike `TestSetup::default_with_root_config_dir`, this isn't
/// scoped under `examples/config/tests/` - example sets are real,
/// documented directories under `examples/` directly. The chosen port
/// still overrides whatever the example's `apimock.toml` hardcodes for
/// its human-facing `curl` walkthrough (`EnvArgs.port` always wins -
/// see `crates/apimock/src/app.rs`), so parallel test runs don't
/// collide on a fixed port.
pub fn example_test_setup(set_name: &str) -> TestSetup {
    TestSetup {
        root_config_file_path: Some(example_config_path(set_name)),
        ..Default::default()
    }
}
