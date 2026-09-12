//! Settings screen navigation, save feedback, and tracked-set reconciliation.
//!
//! Covers FR-015 (return to the dashboard), FR-019 (toggle reflected
//! immediately), FR-021 (persistence) and FR-026 (the dashboard shows the
//! updated tracked set without a restart).

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use devdash::app::state::{AppState, OrgRepoState, Screen, SettingsScreen};
use devdash::app::update::handle_key;
use devdash::domain::repository::{OrgRepo, RepoId, RepoStatus, Repository, TrackedRepo};
use devdash::source::SourceKind;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn org_repo(id: u64, name: &str) -> OrgRepo {
    OrgRepo {
        id: RepoId(id),
        owner: "acme".into(),
        name: name.into(),
        is_archived: false,
        is_fork: false,
    }
}

fn tracked(id: u64, name: &str) -> TrackedRepo {
    TrackedRepo {
        id: RepoId(id),
        owner: "acme".into(),
        name: name.into(),
    }
}

fn ready_repo(id: u64, name: &str) -> Repository {
    Repository {
        id: RepoId(id),
        owner: "acme".into(),
        name: name.into(),
        status: RepoStatus::Ready {
            open_count: 0,
            pulls: Vec::new(),
        },
    }
}

fn base_state() -> AppState {
    AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/devdash-test.log"))
}

/// A repository list screen with `api` and `web` available to toggle.
fn repo_list_state() -> AppState {
    let mut state = base_state();
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready {
            repos: vec![org_repo(1001, "api"), org_repo(1002, "web")],
        },
    });
    state
}

// --- Save feedback -------------------------------------------------------

/// FR-021: a successful write tells the user the choice reached disk.
#[test]
fn toggling_a_repository_confirms_the_save() {
    let dir = TempDir::new().unwrap();
    let mut state = repo_list_state();
    state.config_path = dir.path().join("config.toml");

    handle_key(&mut state, key(KeyCode::Char(' ')));

    let msg = state
        .status_message
        .as_deref()
        .expect("toggling should report the save");
    assert!(
        msg.contains("Saved"),
        "save confirmation should say it saved, got: {msg}"
    );
    assert!(
        msg.contains('1'),
        "save confirmation should report the tracked count, got: {msg}"
    );
}

/// The count in the confirmation follows the tracked set, not the toggle count.
#[test]
fn untracking_a_repository_reports_the_lower_count() {
    let dir = TempDir::new().unwrap();
    let mut state = repo_list_state();
    state.config_path = dir.path().join("config.toml");
    state.tracked = vec![tracked(1001, "api"), tracked(1002, "web")];

    // Cursor sits on `api`, which is already tracked, so this untracks it.
    handle_key(&mut state, key(KeyCode::Char(' ')));

    let msg = state.status_message.as_deref().unwrap();
    assert!(
        msg.contains('1'),
        "untracking should report the remaining count, got: {msg}"
    );
}

/// A failed write must not look identical to a successful one.
#[test]
fn a_failed_save_is_reported_on_screen() {
    let mut state = repo_list_state();
    // `/dev/null` is not a directory, so creating the parent fails.
    state.config_path = PathBuf::from("/dev/null/nope/config.toml");

    handle_key(&mut state, key(KeyCode::Char(' ')));

    let msg = state
        .status_message
        .as_deref()
        .expect("a failed save must be surfaced, not just logged");
    assert!(
        msg.contains("Save failed"),
        "failed save should say so, got: {msg}"
    );
}

// --- Returning to the dashboard ------------------------------------------

/// FR-015: one key leaves settings from the repository list.
#[test]
fn s_returns_to_the_dashboard_from_the_repository_list() {
    let mut state = repo_list_state();

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert!(
        matches!(state.screen, Screen::Dashboard),
        "s should close settings from the repository list"
    );
}

/// FR-015: the same key works from the organization list.
#[test]
fn s_returns_to_the_dashboard_from_the_organization_list() {
    let mut state = base_state();
    state.screen = Screen::Settings(SettingsScreen::Organizations);

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert!(
        matches!(state.screen, Screen::Dashboard),
        "s should close settings from the organization list"
    );
}

/// FR-020: Esc keeps its up-one-level meaning.
#[test]
fn esc_from_the_repository_list_still_goes_to_the_organization_list() {
    let mut state = repo_list_state();

    handle_key(&mut state, key(KeyCode::Esc));

    assert!(
        matches!(
            state.screen,
            Screen::Settings(SettingsScreen::Organizations)
        ),
        "Esc should still step up to the organization list"
    );
}

/// The save confirmation belongs to settings, not to the dashboard.
#[test]
fn leaving_settings_clears_the_save_confirmation() {
    let mut state = repo_list_state();
    state.status_message = Some("Saved · 1 repository tracked".into());

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert!(
        state.status_message.is_none(),
        "the save confirmation should not follow the user to the dashboard"
    );
}

// --- FR-026: the dashboard reflects the new tracked set ------------------

/// FR-026: repositories toggled off are gone from the pane on return.
#[test]
fn returning_to_the_dashboard_drops_untracked_repositories() {
    let mut state = repo_list_state();
    state.repos = vec![ready_repo(1001, "api"), ready_repo(1002, "web")];
    state.tracked = vec![tracked(1002, "web")];
    state.tracked_dirty = true;

    handle_key(&mut state, key(KeyCode::Char('s')));

    let ids: Vec<RepoId> = state.repos.iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec![RepoId(1002)],
        "an untracked repository must leave the pane"
    );
}

/// FR-026: newly tracked repositories appear straight away, as pending.
#[test]
fn returning_to_the_dashboard_adds_newly_tracked_repositories_as_pending() {
    let mut state = repo_list_state();
    state.repos = vec![ready_repo(1002, "web")];
    state.tracked = vec![tracked(1002, "web"), tracked(1003, "cli")];
    state.tracked_dirty = true;

    handle_key(&mut state, key(KeyCode::Char('s')));

    let cli = state
        .repos
        .iter()
        .find(|r| r.id == RepoId(1003))
        .expect("a newly tracked repository must appear immediately");
    assert!(
        matches!(cli.status, RepoStatus::Pending),
        "it has no data yet, so it must read as pending"
    );
    assert_eq!(cli.name, "cli", "the stored name should be displayed");
}

/// The pending rows need a fetch behind them.
#[test]
fn returning_to_the_dashboard_after_a_change_requests_a_refresh() {
    let mut state = repo_list_state();
    state.tracked = vec![tracked(1003, "cli")];
    state.tracked_dirty = true;

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert!(
        state.refresh_requested,
        "a changed tracked set must trigger one refresh"
    );
    assert!(
        !state.tracked_dirty,
        "the dirty flag should be cleared once acted on"
    );
}

/// Leaving settings without touching anything must not spend rate limit.
#[test]
fn returning_to_the_dashboard_unchanged_does_not_refresh() {
    let mut state = repo_list_state();
    state.repos = vec![ready_repo(1002, "web")];
    state.tracked = vec![tracked(1002, "web")];

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert!(
        !state.refresh_requested,
        "an unchanged tracked set must not trigger a refresh (FR-051)"
    );
}

/// The selection must not point at a repository that just left the pane.
#[test]
fn dropping_the_selected_repository_moves_the_selection() {
    let mut state = repo_list_state();
    state.repos = vec![ready_repo(1001, "api"), ready_repo(1002, "web")];
    state.selection.repo = Some(RepoId(1001));
    state.tracked = vec![tracked(1002, "web")];
    state.tracked_dirty = true;

    handle_key(&mut state, key(KeyCode::Char('s')));

    assert_eq!(
        state.selection.repo,
        Some(RepoId(1002)),
        "the selection should land on a surviving repository"
    );
}

/// Toggling is what marks the set dirty; nothing else should.
#[test]
fn toggling_marks_the_tracked_set_dirty() {
    let dir = TempDir::new().unwrap();
    let mut state = repo_list_state();
    state.config_path = dir.path().join("config.toml");

    assert!(!state.tracked_dirty, "a fresh state is not dirty");
    handle_key(&mut state, key(KeyCode::Char(' ')));

    assert!(state.tracked_dirty, "toggling should mark the set dirty");
}

// --- Refresh keeps the pane honest ---------------------------------------

/// A refresh must not resurrect repositories that are no longer tracked.
#[test]
fn a_refresh_drops_repositories_that_left_the_tracked_set() {
    use devdash::app::update::apply_dashboard_data;
    use devdash::source::{RepoPayload, RepositoryData};

    let mut state = base_state();
    state.repos = vec![ready_repo(1001, "api"), ready_repo(1002, "web")];
    state.tracked = vec![tracked(1002, "web")];

    apply_dashboard_data(
        &mut state,
        vec![RepositoryData {
            id: RepoId(1002),
            owner: "acme".into(),
            name: "web".into(),
            result: Ok(RepoPayload {
                open_count: 0,
                pulls: Vec::new(),
            }),
        }],
    );

    let ids: Vec<RepoId> = state.repos.iter().map(|r| r.id).collect();
    assert_eq!(
        ids,
        vec![RepoId(1002)],
        "a stale repository must not survive a refresh"
    );
}

// --- The queued refresh must survive an in-flight fetch -------------------

/// Leaving settings during the first fetch is the most likely moment to
/// change the tracked set. The queued refresh must not be thrown away.
#[test]
fn a_refresh_queued_during_a_fetch_is_kept_until_it_can_run() {
    use devdash::app::update::take_refresh_request;

    let mut state = base_state();
    state.refresh.in_flight = true;
    state.refresh_requested = true;

    assert!(
        !take_refresh_request(&mut state),
        "no second fetch may start while one is in flight"
    );
    assert!(
        state.refresh_requested,
        "the request must stay queued rather than being dropped"
    );

    state.refresh.in_flight = false;
    assert!(
        take_refresh_request(&mut state),
        "the queued request should run once the fetch finishes"
    );
    assert!(!state.refresh_requested, "a dispatched request is consumed");
}

// --- Acceptance: US2 scenario 4 ------------------------------------------

/// "Given repositories have been toggled, when the user returns to the
/// dashboard, then newly tracked repositories appear in the repository pane
/// and untracked ones are gone." Driven end to end through the key handler
/// and the real renderer.
#[test]
fn toggled_repositories_show_on_the_dashboard_without_a_restart() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let dir = TempDir::new().unwrap();
    let mut state = repo_list_state();
    state.config_path = dir.path().join("config.toml");
    // `web` is already tracked and on the dashboard; `api` is not.
    state.tracked = vec![tracked(1002, "web")];
    state.repos = vec![ready_repo(1002, "web")];

    // Cursor starts on `api`: track it, then leave settings.
    handle_key(&mut state, key(KeyCode::Char(' ')));
    handle_key(&mut state, key(KeyCode::Char('s')));

    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| devdash::ui::render(&state, frame))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut screen = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            screen.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        screen.push('\n');
    }

    assert!(
        screen.contains("api"),
        "the repository just tracked should be on the dashboard:\n{screen}"
    );
    assert!(
        screen.contains("web"),
        "the already tracked repository should still be there:\n{screen}"
    );
}
