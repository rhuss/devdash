use std::collections::HashSet;
use std::path::PathBuf;

use time::OffsetDateTime;

use crate::domain::repository::TrackedRepo;
use crate::domain::{OrgRepo, RepoId, RepoStatus, Repository, Viewer, sort_repositories};
use crate::source::SourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Mine,
    ReviewRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Repositories,
    PullRequests,
}

pub struct Selection {
    pub focus: Pane,
    pub repo: Option<RepoId>,
    pub pull: Option<u32>,
}

/// The ordering the selection was last seen in, captured before the lists are
/// rebuilt. Without it a vanished selection can only fall to the top of the
/// list, which FR-045 forbids.
#[derive(Debug, Clone, Default)]
pub struct PrevOrdering {
    /// Repository ids in the order they were displayed.
    pub repos: Vec<RepoId>,
    /// Pull request numbers of the selected repository, in display order.
    pub pulls: Vec<u32>,
}

/// The surviving item closest to where `current` used to sit, preferring the
/// item that now occupies its row over the one above it (FR-045).
fn nearest_surviving<T, F>(prev: &[T], current: T, survives: F) -> Option<T>
where
    T: Copy + PartialEq,
    F: Fn(T) -> bool,
{
    let idx = prev.iter().position(|item| *item == current)?;
    for distance in 1..=prev.len() {
        if let Some(after) = prev.get(idx + distance) {
            if survives(*after) {
                return Some(*after);
            }
        }
        if distance <= idx {
            let before = prev[idx - distance];
            if survives(before) {
                return Some(before);
            }
        }
    }
    None
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            focus: Pane::Repositories,
            repo: None,
            pull: None,
        }
    }
}

impl Selection {
    /// Keep the selection pointing at the same items across a rebuild of the
    /// lists. When a selected item is gone, move to the nearest survivor in
    /// `prev` rather than to the top of the list (FR-045).
    pub fn reconcile(&mut self, repos: &[Repository], prev: &PrevOrdering) {
        if repos.is_empty() {
            self.repo = None;
            self.pull = None;
            return;
        }

        let previous_repo = self.repo;
        let survives = |id: RepoId| repos.iter().any(|r| r.id == id);

        let target = match previous_repo {
            Some(id) if survives(id) => id,
            Some(id) => nearest_surviving(&prev.repos, id, survives).unwrap_or(repos[0].id),
            None => repos[0].id,
        };

        self.repo = Some(target);
        let repo = repos
            .iter()
            .find(|r| r.id == target)
            .expect("target was chosen from repos");

        // The previous pull ordering only describes the repository it was
        // captured from, so it is useless once the selection moves.
        let prev_pulls: &[u32] = if previous_repo == Some(target) {
            &prev.pulls
        } else {
            &[]
        };
        self.reconcile_pull(repo, prev_pulls);
    }

    fn reconcile_pull(&mut self, repo: &Repository, prev_pulls: &[u32]) {
        let RepoStatus::Ready { pulls, .. } = &repo.status else {
            self.pull = None;
            return;
        };

        if pulls.is_empty() {
            self.pull = None;
            return;
        }

        let Some(current) = self.pull else {
            self.pull = Some(pulls[0].number);
            return;
        };

        if pulls.iter().any(|p| p.number == current) {
            return;
        }

        self.pull = Some(
            nearest_surviving(prev_pulls, current, |number| {
                pulls.iter().any(|p| p.number == number)
            })
            .unwrap_or(pulls[0].number),
        );
    }

    pub fn selected_repo_index(&self, repos: &[Repository]) -> Option<usize> {
        self.repo
            .and_then(|id| repos.iter().position(|r| r.id == id))
    }
}

pub enum Screen {
    Loading,
    LoadFailed { reason: String },
    Dashboard,
    Settings(SettingsScreen),
}

pub enum SettingsScreen {
    Organizations,
    Repositories { org: String, state: OrgRepoState },
}

pub enum OrgRepoState {
    Loading,
    Failed { reason: String },
    Ready { repos: Vec<OrgRepo> },
}

#[derive(Default)]
pub struct RefreshState {
    pub in_flight: bool,
    pub last_success: Option<OffsetDateTime>,
    pub last_error: Option<String>,
    pub rate_limited_until: Option<OffsetDateTime>,
}

#[derive(Default)]
pub struct SettingsState {
    pub org_index: usize,
    pub repo_index: usize,
}

// The flags below are one-shot requests and display hints that the event loop
// drains each pass. Grouping them into sub-structs would only add indirection.
#[allow(clippy::struct_excessive_bools)]
pub struct AppState {
    pub screen: Screen,
    pub repos: Vec<Repository>,
    pub viewer: Option<Viewer>,
    pub filter: FilterMode,
    pub selection: Selection,
    pub refresh: RefreshState,
    pub source_kind: SourceKind,
    pub log_path: PathBuf,
    pub should_quit: bool,
    pub status_message: Option<String>,
    /// Whether `status_message` reports a failure. An error indicator has to
    /// carry the log file's path with it (FR-064).
    pub status_is_error: bool,
    pub refresh_requested: bool,
    pub config_path: PathBuf,
    pub tracked: Vec<TrackedRepo>,
    pub untracked_ids: HashSet<RepoId>,
    /// Set when the tracked set changes, so returning to the dashboard knows
    /// it must reconcile the repository pane and refetch (FR-026).
    pub tracked_dirty: bool,
    pub settings_state: SettingsState,
    pub org_fetch_requested: Option<String>,
    /// Why the authenticated user's identity and organization list could not be
    /// retrieved. The settings screen has to say this rather than looking like
    /// a user who belongs to nothing (FR-029).
    pub viewer_error: Option<String>,
    pub viewer_fetch_requested: bool,
    pub refresh_interval_secs: u64,
}

impl AppState {
    pub fn new(source_kind: SourceKind, log_path: PathBuf) -> Self {
        Self {
            screen: Screen::Loading,
            repos: Vec::new(),
            viewer: None,
            filter: FilterMode::All,
            selection: Selection::default(),
            refresh: RefreshState::default(),
            source_kind,
            log_path,
            should_quit: false,
            status_message: None,
            status_is_error: false,
            refresh_requested: false,
            config_path: PathBuf::new(),
            tracked: Vec::new(),
            untracked_ids: HashSet::new(),
            tracked_dirty: false,
            settings_state: SettingsState::default(),
            org_fetch_requested: None,
            viewer_error: None,
            viewer_fetch_requested: false,
            refresh_interval_secs: 300,
        }
    }

    pub fn organizations(&self) -> Vec<String> {
        let mut orgs = Vec::new();
        if let Some(viewer) = &self.viewer {
            orgs.push(viewer.login.clone());
            orgs.extend(viewer.organizations.iter().cloned());
        }
        orgs
    }

    pub fn is_tracked(&self, id: RepoId) -> bool {
        self.tracked.iter().any(|t| t.id == id)
    }

    pub fn toggle_tracked(&mut self, repo: &OrgRepo) {
        self.tracked_dirty = true;
        if let Some(pos) = self.tracked.iter().position(|t| t.id == repo.id) {
            self.tracked.remove(pos);
            self.untracked_ids.insert(repo.id);
        } else {
            self.tracked.push(TrackedRepo {
                id: repo.id,
                owner: repo.owner.clone(),
                name: repo.name.clone(),
            });
            self.untracked_ids.remove(&repo.id);
        }
    }

    /// Capture the ordering the selection is currently sitting in. Take this
    /// before rebuilding the lists so a vanished selection can fall to its
    /// nearest neighbour instead of the top (FR-045).
    pub fn ordering_snapshot(&self) -> PrevOrdering {
        let pulls = self
            .selection
            .repo
            .and_then(|id| self.repos.iter().find(|r| r.id == id))
            .map_or_else(Vec::new, |repo| match &repo.status {
                RepoStatus::Ready { pulls, .. } => pulls.iter().map(|p| p.number).collect(),
                _ => Vec::new(),
            });

        PrevOrdering {
            repos: self.repos.iter().map(|r| r.id).collect(),
            pulls,
        }
    }

    /// Bring the repository pane in line with the tracked set: drop
    /// repositories that are no longer tracked, add pending rows for newly
    /// tracked ones, and leave the selection on something that still exists.
    ///
    /// This is what lets a changed tracked set show on the dashboard without
    /// waiting for a refresh to land, per FR-026.
    pub fn reconcile_repos_to_tracked(&mut self) {
        let prev = self.ordering_snapshot();
        self.reconcile_repos_to_tracked_from(&prev);
    }

    /// As [`Self::reconcile_repos_to_tracked`], but against an ordering
    /// captured earlier. Callers that have already overwritten the repository
    /// list must snapshot before they mutate it.
    pub fn reconcile_repos_to_tracked_from(&mut self, prev: &PrevOrdering) {
        let tracked_ids: HashSet<RepoId> = self.tracked.iter().map(|t| t.id).collect();
        self.repos.retain(|repo| tracked_ids.contains(&repo.id));

        let newly_tracked: Vec<Repository> = self
            .tracked
            .iter()
            .filter(|t| !self.repos.iter().any(|r| r.id == t.id))
            .map(|t| Repository {
                id: t.id,
                owner: t.owner.clone(),
                name: t.name.clone(),
                status: RepoStatus::Pending,
            })
            .collect();
        self.repos.extend(newly_tracked);

        sort_repositories(&mut self.repos);
        self.selection.reconcile(&self.repos, prev);
    }
}

pub fn selected_pull_url(state: &AppState) -> Option<String> {
    let repo_id = state.selection.repo?;
    let pr_num = state.selection.pull?;
    let repo = state.repos.iter().find(|r| r.id == repo_id)?;
    if let RepoStatus::Ready { pulls, .. } = &repo.status {
        pulls
            .iter()
            .find(|p| p.number == pr_num)
            .map(|p| p.url.clone())
    } else {
        None
    }
}
