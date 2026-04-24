use serde::Deserialize;

/// verbose logs
#[derive(Clone, Default, Deserialize)]
pub struct VerboseConfig {
    pub header: bool,
    pub body: bool,
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
