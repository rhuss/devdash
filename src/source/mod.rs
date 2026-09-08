use std::path::PathBuf;

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::pull_request::PullRequest;
use crate::domain::repository::{OrgRepo, RepoId, TrackedRepo};
use crate::domain::viewer::Viewer;

pub mod fixture;
pub mod github;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Live,
    Fixture,
}

pub struct RepositoryData {
    pub id: RepoId,
    pub owner: String,
    pub name: String,
    pub result: Result<RepoPayload, SourceError>,
}

pub struct RepoPayload {
    pub open_count: u32,
    pub pulls: Vec<PullRequest>,
}

#[async_trait]
pub trait DataSource: Send + Sync {
    async fn viewer(&self) -> Result<Viewer, SourceError>;

    async fn dashboard(&self, tracked: &[TrackedRepo]) -> Result<Vec<RepositoryData>, SourceError>;

    async fn org_repositories(&self, org: &str) -> Result<Vec<OrgRepo>, SourceError>;

    fn kind(&self) -> SourceKind;
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("no credential available: {tried}")]
    NoCredential { tried: String },

    #[error("rate limit exhausted, resets at {resets_at}")]
    RateLimited { resets_at: OffsetDateTime },

    #[error("not authorized for {scope}")]
    Unauthorized { scope: String },

    #[error("repository unreadable: {reason}")]
    Repository { id: RepoId, reason: String },

    #[error("network error: {0}")]
    Network(String),

    #[error("unexpected response shape: {0}")]
    Malformed(String),

    #[error("fixture problem at {path}: {reason}")]
    Fixture { path: PathBuf, reason: String },
}
