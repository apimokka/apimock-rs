use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    // todo: some fields to define condition affecting a single rule set wholly
}

impl Guard {
    /// validate
    pub fn validate(&self) -> bool {
        true
    }
}

impl std::fmt::Display for Guard {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
