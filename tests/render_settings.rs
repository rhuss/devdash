use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use devdash::app::state::*;
use devdash::domain::repository::{OrgRepo, RepoId, TrackedRepo};
use devdash::domain::viewer::{TeamMemberships, Viewer};
use devdash::source::SourceKind;
use devdash::ui;

fn test_state_with_viewer() -> AppState {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Settings(SettingsScreen::Organizations);
    state.viewer = Some(Viewer {
        login: "octocat".into(),
        organizations: vec!["acme".into(), "widgets-inc".into()],
        teams: TeamMemberships::Known {
            known: vec!["acme/platform".into()],
        },
    });
    state
}

fn render_to_string(state: &AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::render(state, frame)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            output.push_str(cell.symbol());
        }
        output.push('\n');
    }
    output
}

/// T047: Settings navigation shows org list, entering an org shows repo list.
#[test]
fn settings_shows_org_list() {
    let state = test_state_with_viewer();
    let output = render_to_string(&state, 80, 20);

    assert!(output.contains("octocat"), "personal account should appear");
    assert!(output.contains("acme"), "org 'acme' should appear");
    assert!(
        output.contains("widgets-inc"),
        "org 'widgets-inc' should appear"
    );
    assert!(
        output.contains("Settings") || output.contains("Organizations"),
        "settings title should appear"
    );
}

/// T047: Entering an org drills into its repository list.
#[test]
fn settings_drill_into_org_shows_repos() {
    let mut state = test_state_with_viewer();
    state.settings_state.org_index = 1; // acme
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready {
            repos: vec![
                OrgRepo {
                    id: RepoId(1001),
                    owner: "acme".into(),
                    name: "api".into(),
                    is_archived: false,
                    is_fork: false,
                },
                OrgRepo {
                    id: RepoId(1002),
                    owner: "acme".into(),
                    name: "web".into(),
                    is_archived: false,
                    is_fork: false,
                },
            ],
        },
    });

    let output = render_to_string(&state, 80, 20);
    assert!(output.contains("api"), "repo 'api' should appear");
    assert!(output.contains("web"), "repo 'web' should appear");
}

/// T047: Toggling updates tracked state visible in the list.
#[test]
fn settings_toggle_tracked_updates_display() {
    let mut state = test_state_with_viewer();
    state.tracked.push(TrackedRepo {
        id: RepoId(1001),
        owner: "acme".into(),
        name: "api".into(),
    });
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready {
            repos: vec![
                OrgRepo {
                    id: RepoId(1001),
                    owner: "acme".into(),
                    name: "api".into(),
                    is_archived: false,
                    is_fork: false,
                },
                OrgRepo {
                    id: RepoId(1002),
                    owner: "acme".into(),
                    name: "web".into(),
                    is_archived: false,
                    is_fork: false,
                },
            ],
        },
    });

    let output = render_to_string(&state, 80, 20);
    // api is tracked, web is not
    assert!(
        output.contains("[✓]"),
        "tracked repo should show checked marker"
    );
    assert!(
        output.contains("[ ]"),
        "untracked repo should show unchecked marker"
    );
}

/// T048: Archived repositories are excluded from the displayed list.
#[test]
fn settings_archived_repos_excluded() {
    let mut state = test_state_with_viewer();
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready {
            repos: vec![
                OrgRepo {
                    id: RepoId(1001),
                    owner: "acme".into(),
                    name: "api".into(),
                    is_archived: false,
                    is_fork: false,
                },
                OrgRepo {
                    id: RepoId(1004),
                    owner: "acme".into(),
                    name: "legacy".into(),
                    is_archived: true,
                    is_fork: false,
                },
            ],
        },
    });

    let output = render_to_string(&state, 80, 20);
    assert!(output.contains("api"), "non-archived repo should appear");
    assert!(
        !output.contains("legacy"),
        "archived repo must NOT appear (FR-027)"
    );
}

/// T048: Loading state renders distinctly from an empty org.
#[test]
fn settings_loading_state_renders_distinctly() {
    let mut state = test_state_with_viewer();

    // Loading state
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Loading,
    });
    let loading_output = render_to_string(&state, 80, 20);
    assert!(
        loading_output.contains("Loading"),
        "loading state should show 'Loading'"
    );

    // Empty ready state
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready { repos: vec![] },
    });
    let empty_output = render_to_string(&state, 80, 20);

    assert_ne!(
        loading_output, empty_output,
        "loading state must look different from empty org"
    );
}

/// T048: Failure state renders with retry hint.
#[test]
fn settings_failure_state_shows_error_and_retry() {
    let mut state = test_state_with_viewer();
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Failed {
            reason: "unauthorized".into(),
        },
    });

    let output = render_to_string(&state, 80, 20);
    assert!(
        output.contains("unauthorized") || output.contains("Failed"),
        "failure reason should appear"
    );
    assert!(
        output.contains("retry") || output.contains("r "),
        "retry hint should appear"
    );
}
