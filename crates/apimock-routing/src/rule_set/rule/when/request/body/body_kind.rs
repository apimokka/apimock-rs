use serde::Deserialize;

#[derive(Clone, Deserialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BodyKind {
    Json,
}

impl std::fmt::Display for BodyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "JSON"),
        }
    }
}
