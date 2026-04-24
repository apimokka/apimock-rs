use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub enum Strategy {
    FirstMatch,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::FirstMatch
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstMatch => write!(f, "first_match"),
        }
    }
}
