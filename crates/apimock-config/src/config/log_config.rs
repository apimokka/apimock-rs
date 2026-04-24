use serde::Deserialize;

pub mod verbose_config;

use verbose_config::VerboseConfig;

/// log
#[derive(Clone, Default, Deserialize)]
pub struct LogConfig {
    pub verbose: VerboseConfig,
}

impl std::fmt::Display for LogConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = write!(f, "{}", self.verbose);
        Ok(())
    }
}
