use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::ci::CiState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub is_draft: bool,
    pub url: String,
    pub ci: CiState,
    #[serde(default)]
    pub review_requests: Vec<ReviewRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReviewRequest {
    User { user: String },
    Team { team: String },
}

impl ReviewRequest {
    pub fn is_for_user(&self, login: &str) -> bool {
        matches!(self, Self::User { user } if user == login)
    }

    pub fn is_for_team(&self, slug: &str) -> bool {
        matches!(self, Self::Team { team } if team == slug)
    }
}
