use serde::Deserialize;

/// verbose logs
#[derive(Clone, Default, Deserialize)]
#[non_exhaustive]
pub struct VerboseConfig {
    pub header: bool,
    pub body: bool,
}

impl VerboseConfig {
    /// Construct explicitly. `#[non_exhaustive]` blocks struct-literal
    /// syntax from another crate — `apimock-server` needs one non-default
    /// instance (a `const` test fixture with `header: true`), and this
    /// is a `const fn` for that reason: allowed in a `const` initializer,
    /// where a runtime-only builder would not be.
    pub const fn new(header: bool, body: bool) -> Self {
        Self { header, body }
    }
}

impl std::fmt::Display for VerboseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = writeln!(
            f,
            "[log.verbose] header = {}, body = {}",
            if self.header { "Yes" } else { "No" },
            if self.body { "Yes" } else { "No" }
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_both_fields() {
        let v = VerboseConfig::new(true, false);
        assert!(v.header);
        assert!(!v.body);
    }

    // `new` being usable in a `const` position (the reason it's a
    // `const fn` at all) is exercised for real by
    // `apimock-server::parsed_request::VERBOSE_HEADERS_ONLY` — a
    // second, standalone const-context test here would only restate
    // that construction without observing anything a plain call
    // doesn't already cover.
}
