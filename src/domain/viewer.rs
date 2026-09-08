use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewer {
    pub login: String,
    pub organizations: Vec<String>,
    pub teams: TeamMemberships,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TeamMemberships {
    Known { known: Vec<String> },
    Unavailable { unavailable: String },
}

impl TeamMemberships {
    pub fn slugs(&self) -> &[String] {
        match self {
            Self::Known { known } => known,
            Self::Unavailable { .. } => &[],
        }
    }

    pub const fn is_available(&self) -> bool {
        matches!(self, Self::Known { .. })
    }
}
