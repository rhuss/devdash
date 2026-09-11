use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config;
use crate::domain::{OrgRepo, RepoStatus, Repository, sort_pull_requests, sort_repositories};
use crate::source::{RepoPayload, RepositoryData};

use super::state::{self, AppState, FilterMode, OrgRepoState, Pane, Screen, SettingsScreen};

pub fn handle_key(state: &mut AppState, key: KeyEvent) {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        state.should_quit = true;
        return;
    }

    match &state.screen {
        Screen::Loading => handle_loading_key(state, key),
        Screen::LoadFailed { .. } => handle_load_failed_key(state, key),
        Screen::Dashboard => handle_dashboard_key(state, key),
        Screen::Settings(_) => handle_settings_key(state, key),
    }
}

fn handle_loading_key(state: &mut AppState, key: KeyEvent) {
    if key.code == KeyCode::Char('q') {
        state.should_quit = true;
    }
}

fn handle_load_failed_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('r') => state.screen = Screen::Loading,
        _ => {}
    }
}

fn handle_dashboard_key(state: &mut AppState, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => state.should_quit = true,
        KeyCode::Char('s') => {
            state.screen = Screen::Settings(SettingsScreen::Organizations);
        }
        KeyCode::Char('a') => state.filter = FilterMode::All,
        KeyCode::Char('m') => state.filter = FilterMode::Mine,
        KeyCode::Char('v') => state.filter = FilterMode::ReviewRequested,
        KeyCode::Up | KeyCode::Char('k') => move_selection_up(state),
        KeyCode::Down | KeyCode::Char('j') => move_selection_down(state),
        KeyCode::Left | KeyCode::Char('h') => {
            state.selection.focus = Pane::Repositories;
        }
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
            state.selection.focus = Pane::PullRequests;
        }
        KeyCode::Char('r') => {
            if !state.refresh.in_flight {
                state.refresh_requested = true;
            }
        }
        KeyCode::Enter => open_selected_pr(state),
        _ => {}
    }
}

fn open_selected_pr(state: &mut AppState) {
    let Some(url) = state::selected_pull_url(state) else {
        return;
    };

    state.status_message = None;

    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };

    match std::process::Command::new(opener)
        .arg(&url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => {}
        Err(_) => {
            state.status_message = Some(format!("Open: {url}"));
        }
    }
}

fn handle_settings_key(state: &mut AppState, key: KeyEvent) {
    let screen = std::mem::replace(&mut state.screen, Screen::Dashboard);
    let new_screen = match screen {
        Screen::Settings(SettingsScreen::Organizations) => handle_settings_orgs_key(state, key),
        Screen::Settings(SettingsScreen::Repositories {
            org,
            state: org_state,
        }) => handle_settings_repos_key(state, key, org, org_state),
        other => other,
    };
    state.screen = new_screen;
}

fn handle_settings_orgs_key(state: &mut AppState, key: KeyEvent) -> Screen {
    let orgs = state.organizations();
    match key.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
            Screen::Dashboard
        }
        KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => Screen::Dashboard,
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_state.org_index > 0 {
                state.settings_state.org_index -= 1;
            }
            Screen::Settings(SettingsScreen::Organizations)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !orgs.is_empty() && state.settings_state.org_index + 1 < orgs.len() {
                state.settings_state.org_index += 1;
            }
            Screen::Settings(SettingsScreen::Organizations)
        }
        KeyCode::Enter => {
            if let Some(org) = orgs.get(state.settings_state.org_index) {
                state.settings_state.repo_index = 0;
                state.org_fetch_requested = Some(org.clone());
                Screen::Settings(SettingsScreen::Repositories {
                    org: org.clone(),
                    state: OrgRepoState::Loading,
                })
            } else {
                Screen::Settings(SettingsScreen::Organizations)
            }
        }
        _ => Screen::Settings(SettingsScreen::Organizations),
    }
}

fn handle_settings_repos_key(
    state: &mut AppState,
    key: KeyEvent,
    org: String,
    org_state: OrgRepoState,
) -> Screen {
    match key.code {
        KeyCode::Char('q') => {
            state.should_quit = true;
            Screen::Dashboard
        }
        KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
            Screen::Settings(SettingsScreen::Organizations)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if state.settings_state.repo_index > 0 {
                state.settings_state.repo_index -= 1;
            }
            Screen::Settings(SettingsScreen::Repositories {
                org,
                state: org_state,
            })
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let OrgRepoState::Ready { ref repos } = org_state {
                let visible: Vec<&OrgRepo> = repos.iter().filter(|r| !r.is_archived).collect();
                if !visible.is_empty() && state.settings_state.repo_index + 1 < visible.len() {
                    state.settings_state.repo_index += 1;
                }
            }
            Screen::Settings(SettingsScreen::Repositories {
                org,
                state: org_state,
            })
        }
        KeyCode::Char(' ') => {
            if let OrgRepoState::Ready { ref repos } = org_state {
                let visible: Vec<&OrgRepo> = repos.iter().filter(|r| !r.is_archived).collect();
                if let Some(repo) = visible.get(state.settings_state.repo_index) {
                    let repo_clone = (*repo).clone();
                    state.toggle_tracked(&repo_clone);
                    persist_config(state);
                }
            }
            Screen::Settings(SettingsScreen::Repositories {
                org,
                state: org_state,
            })
        }
        KeyCode::Char('r') => {
            if matches!(org_state, OrgRepoState::Failed { .. }) {
                state.org_fetch_requested = Some(org.clone());
                Screen::Settings(SettingsScreen::Repositories {
                    org,
                    state: OrgRepoState::Loading,
                })
            } else {
                Screen::Settings(SettingsScreen::Repositories {
                    org,
                    state: org_state,
                })
            }
        }
        _ => Screen::Settings(SettingsScreen::Repositories {
            org,
            state: org_state,
        }),
    }
}

fn persist_config(state: &AppState) {
    if state.config_path.as_os_str().is_empty() {
        return;
    }
    let cfg = config::Config {
        refresh_interval_secs: state.refresh_interval_secs,
        tracked: state.tracked.clone(),
    };
    if let Err(e) = config::save(&cfg, &state.config_path, &state.untracked_ids) {
        tracing::warn!("Failed to save config: {e}");
    }
}

#[allow(clippy::collapsible_if)]
fn move_selection_up(state: &mut AppState) {
    match state.selection.focus {
        Pane::Repositories => {
            if let Some(idx) = state.selection.selected_repo_index(&state.repos) {
                if idx > 0 {
                    let new_repo = &state.repos[idx - 1];
                    state.selection.repo = Some(new_repo.id);
                    state.selection.pull = first_pull_number(new_repo);
                }
            }
        }
        Pane::PullRequests => {
            if let Some(repo) = selected_repo(state) {
                if let RepoStatus::Ready { pulls, .. } = &repo.status {
                    if let Some(current) = state.selection.pull {
                        if let Some(idx) = pulls.iter().position(|p| p.number == current) {
                            if idx > 0 {
                                state.selection.pull = Some(pulls[idx - 1].number);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::collapsible_if)]
fn move_selection_down(state: &mut AppState) {
    match state.selection.focus {
        Pane::Repositories => {
            if let Some(idx) = state.selection.selected_repo_index(&state.repos) {
                if idx + 1 < state.repos.len() {
                    let new_repo = &state.repos[idx + 1];
                    state.selection.repo = Some(new_repo.id);
                    state.selection.pull = first_pull_number(new_repo);
                }
            }
        }
        Pane::PullRequests => {
            if let Some(repo) = selected_repo(state) {
                if let RepoStatus::Ready { pulls, .. } = &repo.status {
                    if let Some(current) = state.selection.pull {
                        if let Some(idx) = pulls.iter().position(|p| p.number == current) {
                            if idx + 1 < pulls.len() {
                                state.selection.pull = Some(pulls[idx + 1].number);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn selected_repo(state: &AppState) -> Option<&Repository> {
    state
        .selection
        .repo
        .and_then(|id| state.repos.iter().find(|r| r.id == id))
}

fn first_pull_number(repo: &Repository) -> Option<u32> {
    match &repo.status {
        RepoStatus::Ready { pulls, .. } => pulls.first().map(|p| p.number),
        _ => None,
    }
}

pub fn apply_dashboard_data(state: &mut AppState, data: Vec<RepositoryData>) {
    for item in data {
        let status = match item.result {
            Ok(RepoPayload { open_count, pulls }) => RepoStatus::Ready { open_count, pulls },
            Err(e) => RepoStatus::Unreadable {
                reason: e.to_string(),
            },
        };
        if let Some(repo) = state.repos.iter_mut().find(|r| r.id == item.id) {
            repo.owner = item.owner;
            repo.name = item.name;
            repo.status = status;
        } else {
            state.repos.push(Repository {
                id: item.id,
                owner: item.owner,
                name: item.name,
                status,
            });
        }
    }
    sort_repositories(&mut state.repos);
    for repo in &mut state.repos {
        if let RepoStatus::Ready { pulls, .. } = &mut repo.status {
            sort_pull_requests(pulls);
        }
    }
    state.selection.reconcile(&state.repos);

    // T055: Rename following - update stored tracked entries when API returns different owner/name
    let mut tracked_changed = false;
    for repo in &state.repos {
        if let Some(tracked) = state.tracked.iter_mut().find(|t| t.id == repo.id) {
            if tracked.owner != repo.owner || tracked.name != repo.name {
                tracked.owner.clone_from(&repo.owner);
                tracked.name.clone_from(&repo.name);
                tracked_changed = true;
            }
        }
    }
    if tracked_changed {
        persist_config(state);
    }

    if matches!(state.screen, Screen::Loading) {
        state.screen = Screen::Dashboard;
    }
}
