use std::collections::HashSet;
use std::path::PathBuf;

use time::OffsetDateTime;

use crate::domain::repository::TrackedRepo;
use crate::domain::{OrgRepo, RepoId, RepoStatus, Repository, Viewer};
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
    pub fn reconcile(&mut self, repos: &[Repository]) {
        if repos.is_empty() {
            self.repo = None;
            self.pull = None;
            return;
        }

        if let Some(current_id) = self.repo {
            if let Some(repo) = repos.iter().find(|r| r.id == current_id) {
                self.reconcile_pull(repo);
                return;
            }
            // Selected repo disappeared; pick the nearest survivor.
            // We don't have the previous ordering, so just pick the first repo.
            let fallback = &repos[0];
            self.repo = Some(fallback.id);
            self.reconcile_pull(fallback);
        } else {
            let first = &repos[0];
            self.repo = Some(first.id);
            self.reconcile_pull(first);
        }
    }

    fn reconcile_pull(&mut self, repo: &Repository) {
        let RepoStatus::Ready { pulls, .. } = &repo.status else {
            self.pull = None;
            return;
        };

        if pulls.is_empty() {
            self.pull = None;
            return;
        }

        if self
            .pull
            .is_some_and(|pr_num| pulls.iter().any(|p| p.number == pr_num))
        {
            return;
        }
        self.pull = Some(pulls[0].number);
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
    pub refresh_requested: bool,
    pub config_path: PathBuf,
    pub tracked: Vec<TrackedRepo>,
    pub untracked_ids: HashSet<RepoId>,
    pub settings_state: SettingsState,
    pub org_fetch_requested: Option<String>,
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
            refresh_requested: false,
            config_path: PathBuf::new(),
            tracked: Vec::new(),
            untracked_ids: HashSet::new(),
            settings_state: SettingsState::default(),
            org_fetch_requested: None,
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
