use std::path::PathBuf;

use time::macros::datetime;

use devdash::app::state::{self, AppState, Pane, Screen};
use devdash::domain::ci::CiState;
use devdash::domain::pull_request::PullRequest;
use devdash::domain::repository::{RepoId, RepoStatus, Repository};
use devdash::source::SourceKind;

fn make_pr(number: u32, url: &str, ci: CiState) -> PullRequest {
    PullRequest {
        number,
        title: format!("PR #{number}"),
        author: "alice".to_string(),
        updated_at: datetime!(2026-09-08 10:00:00 UTC),
        is_draft: false,
        url: url.to_string(),
        ci,
        review_requests: Vec::new(),
    }
}

fn test_state() -> (AppState, &'static str) {
    let expected_url = "https://github.com/acme/api/pull/42";
    let repos = vec![Repository {
        id: RepoId(1),
        owner: "acme".to_string(),
        name: "api".to_string(),
        status: RepoStatus::Ready {
            open_count: 2,
            pulls: vec![
                make_pr(42, expected_url, CiState::Passing),
                make_pr(43, "https://github.com/acme/api/pull/43", CiState::Failing),
            ],
        },
    }];

    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Dashboard;
    state.repos = repos;
    state.selection.repo = Some(RepoId(1));
    state.selection.pull = Some(42);
    state.selection.focus = Pane::PullRequests;

    (state, expected_url)
}

#[test]
fn selected_pr_url_matches_expected() {
    let (state, expected_url) = test_state();
    let url = state::selected_pull_url(&state);
    assert_eq!(url.as_deref(), Some(expected_url));
}

#[test]
fn no_url_when_no_pr_selected() {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Dashboard;
    assert!(state::selected_pull_url(&state).is_none());
}

#[test]
fn no_url_when_repo_pending() {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Dashboard;
    state.repos = vec![Repository {
        id: RepoId(1),
        owner: "acme".to_string(),
        name: "api".to_string(),
        status: RepoStatus::Pending,
    }];
    state.selection.repo = Some(RepoId(1));
    state.selection.pull = Some(42);
    assert!(state::selected_pull_url(&state).is_none());
}

#[test]
fn url_changes_when_selection_changes() {
    let (mut state, _) = test_state();
    state.selection.pull = Some(43);
    let url = state::selected_pull_url(&state);
    assert_eq!(url.as_deref(), Some("https://github.com/acme/api/pull/43"));
}
