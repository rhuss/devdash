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

/// FR-015: the repository list names the key that leaves settings, not just
/// the one that steps up a level.
#[test]
fn settings_repo_list_hints_the_way_back_to_the_dashboard() {
    let mut state = test_state_with_viewer();
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Ready {
            repos: vec![OrgRepo {
                id: RepoId(1001),
                owner: "acme".into(),
                name: "api".into(),
                is_archived: false,
                is_fork: false,
            }],
        },
    });

    let output = render_to_string(&state, 100, 20);
    assert!(
        output.contains("s:dashboard"),
        "the repository list should name the key that returns to the dashboard"
    );
    assert!(
        output.contains("Esc:orgs"),
        "Esc should be labelled as stepping up to the organization list"
    );
}

/// On the organization list, Esc is itself the way back to the dashboard.
#[test]
fn settings_org_list_labels_esc_as_the_way_back() {
    let state = test_state_with_viewer();

    let output = render_to_string(&state, 100, 20);
    assert!(
        output.contains("Esc:dashboard"),
        "the organization list should label Esc as returning to the dashboard"
    );
}

/// The save confirmation is visible on the settings screen itself.
#[test]
fn settings_shows_the_save_confirmation() {
    let mut state = test_state_with_viewer();
    state.status_message = Some("Saved · 2 repositories tracked".into());

    let output = render_to_string(&state, 100, 20);
    assert!(
        output.contains("Saved"),
        "the save confirmation should render on the settings screen"
    );
}

/// FR-029: when the account and its organizations cannot be read, the settings
/// screen says so instead of looking like a user who belongs to nothing.
#[test]
fn settings_reports_why_the_organization_list_is_unavailable() {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Settings(SettingsScreen::Organizations);
    state.viewer = None;
    state.viewer_error = Some("not authorized for GraphQL API".into());

    let output = render_to_string(&state, 100, 20);

    assert!(
        output.contains("not authorized"),
        "the reason should be on screen:\n{output}"
    );
    assert!(
        output.contains("/tmp/test.log"),
        "FR-064: the log path should be reachable from the error:\n{output}"
    );
    assert!(
        output.contains("r:retry") || output.contains("r to retry"),
        "the retry key should be named:\n{output}"
    );
}

/// An empty list with no error yet is still loading, not a failure.
#[test]
fn settings_without_a_viewer_yet_reads_as_loading() {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Settings(SettingsScreen::Organizations);

    let output = render_to_string(&state, 100, 20);

    assert!(
        output.contains("Loading"),
        "a pending account fetch should read as loading:\n{output}"
    );
}

/// FR-064: a failed organization fetch names the log file too.
#[test]
fn settings_org_failure_names_the_log_file() {
    let mut state = test_state_with_viewer();
    state.screen = Screen::Settings(SettingsScreen::Repositories {
        org: "acme".into(),
        state: OrgRepoState::Failed {
            reason: "SSO enforcement".into(),
        },
    });

    let output = render_to_string(&state, 100, 20);

    assert!(output.contains("SSO enforcement"));
    assert!(
        output.contains("/tmp/test.log"),
        "the log path should accompany the failure:\n{output}"
    );
}
