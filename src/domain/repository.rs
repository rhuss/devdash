use serde::{Deserialize, Serialize};

use super::pull_request::PullRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepoId(pub u64);

#[derive(Debug, Clone)]
pub struct Repository {
    pub id: RepoId,
    pub owner: String,
    pub name: String,
    pub status: RepoStatus,
}

#[derive(Debug, Clone)]
pub enum RepoStatus {
    Pending,
    Ready {
        open_count: u32,
        pulls: Vec<PullRequest>,
    },
    Unreadable {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRepo {
    pub id: RepoId,
    pub owner: String,
    pub name: String,
    pub is_archived: bool,
    #[serde(default)]
    pub is_fork: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedRepo {
    pub id: RepoId,
    pub owner: String,
    pub name: String,
}
