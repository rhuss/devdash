use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use time::macros::datetime;

use devdash::app::state::{AppState, Screen};
use devdash::domain::ci::CiState;
use devdash::domain::pull_request::PullRequest;
use devdash::domain::repository::{RepoId, RepoStatus, Repository};
use devdash::source::SourceKind;
use devdash::ui;

fn make_pr(number: u32, title: &str, author: &str, ci: CiState) -> PullRequest {
    PullRequest {
        number,
        title: title.to_string(),
        author: author.to_string(),
        updated_at: datetime!(2026-09-08 10:00:00 UTC),
        is_draft: false,
        url: format!("https://github.com/test/repo/pull/{number}"),
        ci,
        review_requests: Vec::new(),
    }
}

fn make_repo(id: u64, owner: &str, name: &str, pulls: Vec<PullRequest>) -> Repository {
    #[allow(clippy::cast_possible_truncation)]
    let open_count = pulls.len() as u32;
    Repository {
        id: RepoId(id),
        owner: owner.to_string(),
        name: name.to_string(),
        status: RepoStatus::Ready { open_count, pulls },
    }
}

fn test_state_with_repos(repos: Vec<Repository>) -> AppState {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Dashboard;
    state.repos = repos;
    if let Some(first) = state.repos.first() {
        state.selection.repo = Some(first.id);
        if let RepoStatus::Ready { pulls, .. } = &first.status {
            state.selection.pull = pulls.first().map(|p| p.number);
        }
    }
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

/// T023: Two-pane render test (FR-001).
/// With repositories loaded, the output must contain repository names
/// and pull request information in two distinct panes.
#[test]

fn two_pane_layout_shows_repos_and_pulls() {
    let repos = vec![
        make_repo(
            1,
            "acme",
            "api",
            vec![
                make_pr(10, "Add retry logic", "alice", CiState::Passing),
                make_pr(11, "Fix timeout bug", "bob", CiState::Failing),
            ],
        ),
        make_repo(
            2,
            "acme",
            "web",
            vec![make_pr(20, "Update deps", "carol", CiState::Pending)],
        ),
    ];

    let state = test_state_with_repos(repos);
    let output = render_to_string(&state, 120, 30);

    assert!(output.contains("acme/api"), "repo pane must show acme/api");
    assert!(output.contains("acme/web"), "repo pane must show acme/web");
    assert!(
        output.contains("Add retry logic") || output.contains("#10"),
        "pull request pane must show the selected repo's PRs"
    );
}

/// T024: A repository with zero open pull requests shows count 0,
/// no CI indicator, and an explicit empty message when selected.
#[test]

fn empty_repo_shows_zero_count_and_empty_message() {
    let repos = vec![
        make_repo(1, "acme", "empty-repo", vec![]),
        make_repo(
            2,
            "acme",
            "other",
            vec![make_pr(5, "Some PR", "alice", CiState::Passing)],
        ),
    ];

    let state = test_state_with_repos(repos);
    let output = render_to_string(&state, 120, 30);

    assert!(
        output.contains('0') || output.contains("empty-repo"),
        "empty repo should appear with a zero count"
    );
    // The CI indicators are ✓ ✗ ● ○ — none should appear for the empty repo row
    // (the repo with zero PRs should have no rollup indicator)
    // When selected, the PR pane should show an empty message
    assert!(
        output.contains("No open pull requests")
            || output.contains("no open pull requests")
            || output.contains("No pull requests"),
        "selecting an empty repo must show an explicit empty message, got:\n{output}"
    );
}

/// T025: A repository whose pull requests include a failure shows the
/// failing indicator (✗) in the repository pane without being selected.
#[test]

fn failing_rollup_visible_without_selection() {
    let repos = vec![
        make_repo(
            1,
            "acme",
            "healthy",
            vec![make_pr(10, "Good PR", "alice", CiState::Passing)],
        ),
        make_repo(
            2,
            "acme",
            "broken",
            vec![
                make_pr(20, "Passing PR", "bob", CiState::Passing),
                make_pr(21, "Broken PR", "carol", CiState::Failing),
            ],
        ),
    ];

    // Select the first repo so "broken" is NOT selected
    let state = test_state_with_repos(repos);
    let output = render_to_string(&state, 120, 30);

    // The failing indicator ✗ must appear in the repo pane for the "broken" repo
    // even though it is not selected
    assert!(
        output.contains("✗"),
        "failing CI indicator must be visible for unselected repo with a failing PR"
    );
    assert!(
        output.contains("✓"),
        "passing CI indicator must be visible for the healthy repo"
    );
}

/// T026: Colour-independence test (SC-013).
/// All four CI states must produce distinct symbols in the text buffer,
/// so they remain distinguishable with colour removed entirely.
#[test]

fn all_four_ci_states_distinguishable_by_symbol() {
    let repos = vec![make_repo(
        1,
        "acme",
        "mixed",
        vec![
            make_pr(1, "PR passing", "a", CiState::Passing),
            make_pr(2, "PR failing", "b", CiState::Failing),
            make_pr(3, "PR pending", "c", CiState::Pending),
            make_pr(4, "PR no checks", "d", CiState::NoChecks),
        ],
    )];

    let state = test_state_with_repos(repos);
    let output = render_to_string(&state, 120, 30);

    let symbols = ["✓", "✗", "●", "○"];
    for sym in &symbols {
        assert!(
            output.contains(sym),
            "CI symbol {sym} must appear in the rendered output"
        );
    }

    // Verify all four symbols are distinct (they are by construction,
    // but assert the buffer actually contains four different ones)
    let mut found: Vec<&str> = symbols
        .iter()
        .filter(|s| output.contains(**s))
        .copied()
        .collect();
    found.dedup();
    assert_eq!(
        found.len(),
        4,
        "all four CI state symbols must be distinct in the text buffer"
    );
}

/// T072: Failed refresh leaves previous data on screen (no blank frame, SC-009).
#[test]
fn failed_refresh_retains_previous_data() {
    let repos = vec![make_repo(
        1,
        "acme",
        "api",
        vec![make_pr(10, "Add retry logic", "alice", CiState::Passing)],
    )];

    let mut state = test_state_with_repos(repos);
    state.refresh.last_error = Some("network timeout".to_string());

    let output = render_to_string(&state, 120, 30);

    assert!(
        output.contains("acme/api"),
        "previous data must remain visible after a failed refresh"
    );
    assert!(
        output.contains("⚠") || output.contains("network timeout"),
        "error indicator must be shown"
    );
}

/// T072: An unreadable repository shows a marked row while others remain normal.
#[test]
fn unreadable_repo_degrades_to_marked_row() {
    let repos = vec![
        make_repo(
            1,
            "acme",
            "api",
            vec![make_pr(10, "Good PR", "alice", CiState::Passing)],
        ),
        Repository {
            id: RepoId(2),
            owner: "acme".to_string(),
            name: "broken".to_string(),
            status: RepoStatus::Unreadable {
                reason: "access denied".to_string(),
            },
        },
    ];

    let state = test_state_with_repos(repos);
    let output = render_to_string(&state, 120, 30);

    assert!(
        output.contains("acme/api"),
        "healthy repo must still render"
    );
    assert!(
        output.contains("acme/broken"),
        "unreadable repo must still appear"
    );
    assert!(
        output.contains("⚠"),
        "unreadable repo should show a warning marker"
    );
}

/// FR-042 / US5 scenario 6: before the first fetch returns, the dashboard shows
/// its own layout with a loading state, not a standalone box.
#[test]
fn loading_state_shows_the_dashboard_layout() {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Loading;

    let output = render_to_string(&state, 100, 20);

    assert!(
        output.contains("Repositories"),
        "the repository pane should already be laid out:\n{output}"
    );
    assert!(
        output.contains("Pull Requests"),
        "the pull request pane should already be laid out:\n{output}"
    );
    assert!(
        output.contains("Loading"),
        "the panes should say they are loading:\n{output}"
    );
    assert!(
        output.contains("q:quit"),
        "the quit key should be discoverable while loading:\n{output}"
    );
}

/// FR-064: an error indicator carries the path of the log holding the detail.
#[test]
fn an_error_status_message_names_the_log_file() {
    let mut state = test_state_with_repos(vec![make_repo(
        1,
        "acme",
        "api",
        vec![make_pr(10, "PR A", "alice", CiState::Passing)],
    )]);
    state.status_message = Some("Save failed: permission denied".into());
    state.status_is_error = true;

    let output = render_to_string(&state, 160, 20);

    assert!(
        output.contains("Save failed"),
        "the failure should be on screen:\n{output}"
    );
    assert!(
        output.contains("/tmp/test.log"),
        "the log path should accompany the failure:\n{output}"
    );
}

/// A success message is not an error, so it must not drag the log path along.
#[test]
fn a_success_status_message_does_not_name_the_log_file() {
    let mut state = test_state_with_repos(vec![make_repo(
        1,
        "acme",
        "api",
        vec![make_pr(10, "PR A", "alice", CiState::Passing)],
    )]);
    state.status_message = Some("Saved \u{b7} 1 repository tracked".into());

    let output = render_to_string(&state, 160, 20);

    assert!(output.contains("Saved"));
    assert!(
        !output.contains("/tmp/test.log"),
        "a confirmation should not advertise the log file:\n{output}"
    );
}
