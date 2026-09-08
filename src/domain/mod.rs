pub mod ci;
pub mod pull_request;
pub mod repository;
pub mod viewer;

pub use ci::{CiState, rollup};
pub use pull_request::{PullRequest, ReviewRequest};
pub use repository::{OrgRepo, RepoId, RepoStatus, Repository, TrackedRepo};
pub use viewer::{TeamMemberships, Viewer};

/// Sort repositories by owner then name, case-insensitive (FR-014).
pub fn sort_repositories(repos: &mut [Repository]) {
    repos.sort_by(|a, b| {
        a.owner
            .to_lowercase()
            .cmp(&b.owner.to_lowercase())
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

/// Sort pull requests by `updated_at` descending, most recent first (FR-014).
pub fn sort_pull_requests(pulls: &mut [PullRequest]) {
    pulls.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
}
