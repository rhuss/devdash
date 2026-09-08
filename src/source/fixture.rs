use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::Deserialize;

use crate::domain::pull_request::PullRequest;
use crate::domain::repository::{OrgRepo, RepoId, TrackedRepo};
use crate::domain::viewer::Viewer;

use super::{DataSource, RepoPayload, RepositoryData, SourceError, SourceKind};

#[derive(Debug, Deserialize)]
struct FixtureRepository {
    id: u64,
    owner: String,
    name: String,
    open_count: u32,
    pulls: Vec<PullRequest>,
}

#[derive(Debug, Deserialize)]
struct FixtureSnapshot {
    viewer: Viewer,
    repositories: Vec<FixtureRepository>,
    #[serde(default)]
    org_repositories: HashMap<String, Vec<OrgRepo>>,
}

pub struct FixtureDataSource {
    snapshot: FixtureSnapshot,
    path: PathBuf,
}

impl FixtureDataSource {
    pub fn load(path: &Path) -> Result<Self, SourceError> {
        let content = std::fs::read_to_string(path).map_err(|e| SourceError::Fixture {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;

        let snapshot: FixtureSnapshot =
            serde_json::from_str(&content).map_err(|e| SourceError::Fixture {
                path: path.to_path_buf(),
                reason: e.to_string(),
            })?;

        Ok(Self {
            snapshot,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[async_trait]
impl DataSource for FixtureDataSource {
    async fn viewer(&self) -> Result<Viewer, SourceError> {
        Ok(self.snapshot.viewer.clone())
    }

    async fn dashboard(&self, tracked: &[TrackedRepo]) -> Result<Vec<RepositoryData>, SourceError> {
        let results = tracked
            .iter()
            .map(|t| {
                let found = self.snapshot.repositories.iter().find(|r| r.id == t.id.0);

                match found {
                    Some(repo) => RepositoryData {
                        id: RepoId(repo.id),
                        owner: repo.owner.clone(),
                        name: repo.name.clone(),
                        result: Ok(RepoPayload {
                            open_count: repo.open_count,
                            pulls: repo.pulls.clone(),
                        }),
                    },
                    None => RepositoryData {
                        id: t.id,
                        owner: t.owner.clone(),
                        name: t.name.clone(),
                        result: Err(SourceError::Repository {
                            id: t.id,
                            reason: format!(
                                "repository {}/{} not found in fixture at {}",
                                t.owner,
                                t.name,
                                self.path.display()
                            ),
                        }),
                    },
                }
            })
            .collect();

        Ok(results)
    }

    async fn org_repositories(&self, org: &str) -> Result<Vec<OrgRepo>, SourceError> {
        Ok(self
            .snapshot
            .org_repositories
            .get(org)
            .cloned()
            .unwrap_or_default())
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Fixture
    }
}
