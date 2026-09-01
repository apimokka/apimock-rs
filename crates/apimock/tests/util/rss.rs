//! `VmRSS` reading for RFC 068 S-02's memory assertion — "the status can
//! be right while the body was still buffered first," so the test needs
//! actual process memory, not just the response status.
//!
//! Mirrors `crates/apimock/examples/bench_load.rs`'s own `read_rss_kb`
//! (not reused directly — examples and integration tests are separate
//! compilation targets with no way to share code between them without a
//! new library-only module, which is more machinery than one ten-line
//! function warrants).

/// Parse `VmRSS` out of `/proc/self/status`, in kB. `None` on any
/// non-Linux platform — callers must treat that as "skip the RSS
/// assertion," not as a test failure, since CI also runs macOS and
/// Windows jobs.
pub fn read_rss_kb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse::<u64>().ok());
        }
    }
    None
}
