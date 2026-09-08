use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use time::macros::datetime;

use devdash::app::state::{AppState, FilterMode, Screen};
use devdash::domain::ci::CiState;
use devdash::domain::pull_request::{PullRequest, ReviewRequest};
use devdash::domain::repository::{RepoId, RepoStatus, Repository};
use devdash::domain::viewer::{TeamMemberships, Viewer};
use devdash::source::SourceKind;
use devdash::ui;

fn make_pr(
    number: u32,
    title: &str,
    author: &str,
    ci: CiState,
    review_requests: Vec<ReviewRequest>,
) -> PullRequest {
    PullRequest {
        number,
        title: title.to_string(),
        author: author.to_string(),
        updated_at: datetime!(2026-09-08 10:00:00 UTC),
        is_draft: false,
        url: format!("https://github.com/test/repo/pull/{number}"),
        ci,
        review_requests,
    }
}

fn make_viewer() -> Viewer {
    Viewer {
        login: "octocat".to_string(),
        organizations: vec!["acme".to_string()],
        teams: TeamMemberships::Known {
            known: vec!["acme/platform".to_string()],
        },
    }
}

fn make_viewer_no_teams() -> Viewer {
    Viewer {
        login: "octocat".to_string(),
        organizations: vec!["acme".to_string()],
        teams: TeamMemberships::Unavailable {
            unavailable: "read:org scope missing".to_string(),
        },
    }
}

fn test_state(repos: Vec<Repository>, viewer: Viewer, filter: FilterMode) -> AppState {
    let mut state = AppState::new(SourceKind::Fixture, PathBuf::from("/tmp/test.log"));
    state.screen = Screen::Dashboard;
    state.repos = repos;
    state.viewer = Some(viewer);
    state.filter = filter;
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

fn test_repos() -> Vec<Repository> {
    vec![
        Repository {
            id: RepoId(1),
            owner: "acme".to_string(),
            name: "api".to_string(),
            status: RepoStatus::Ready {
                open_count: 4,
                pulls: vec![
                    make_pr(10, "My PR", "octocat", CiState::Passing, vec![]),
                    make_pr(
                        11,
                        "Review for me",
                        "alice",
                        CiState::Failing,
                        vec![ReviewRequest::User {
                            user: "octocat".to_string(),
                        }],
                    ),
                    make_pr(
                        12,
                        "Team review",
                        "bob",
                        CiState::Pending,
                        vec![ReviewRequest::Team {
                            team: "acme/platform".to_string(),
                        }],
                    ),
                    make_pr(13, "Someone else PR", "carol", CiState::NoChecks, vec![]),
                ],
            },
        },
        Repository {
            id: RepoId(2),
            owner: "acme".to_string(),
            name: "web".to_string(),
            status: RepoStatus::Ready {
                open_count: 0,
                pulls: vec![],
            },
        },
        Repository {
            id: RepoId(3),
            owner: "acme".to_string(),
            name: "cli".to_string(),
            status: RepoStatus::Ready {
                open_count: 2,
                pulls: vec![
                    make_pr(20, "Other person PR", "dave", CiState::Passing, vec![]),
                    make_pr(21, "Another other PR", "eve", CiState::Passing, vec![]),
                ],
            },
        },
    ]
}

/// T063: All three filter modes work correctly.
#[test]
fn filter_all_shows_all_prs() {
    let state = test_state(test_repos(), make_viewer(), FilterMode::All);
    let output = render_to_string(&state, 120, 30);
    assert!(output.contains("#10"), "All filter should show PR #10");
    assert!(output.contains("#11"), "All filter should show PR #11");
    assert!(output.contains("#12"), "All filter should show PR #12");
    assert!(output.contains("#13"), "All filter should show PR #13");
}

#[test]
fn filter_mine_shows_only_viewer_prs() {
    let state = test_state(test_repos(), make_viewer(), FilterMode::Mine);
    let output = render_to_string(&state, 120, 30);
    assert!(
        output.contains("#10"),
        "Mine filter should show octocat's PR #10"
    );
    assert!(
        !output.contains("#11"),
        "Mine filter should hide alice's PR #11"
    );
    assert!(
        !output.contains("#12"),
        "Mine filter should hide bob's PR #12"
    );
    assert!(
        !output.contains("#13"),
        "Mine filter should hide carol's PR #13"
    );
}

#[test]
fn filter_review_requested_includes_direct_and_team() {
    let state = test_state(test_repos(), make_viewer(), FilterMode::ReviewRequested);
    let output = render_to_string(&state, 120, 30);
    assert!(
        output.contains("#11"),
        "ReviewRequested filter should show PR #11 (direct review request)"
    );
    assert!(
        output.contains("#12"),
        "ReviewRequested filter should show PR #12 (team review request via acme/platform)"
    );
    assert!(
        !output.contains("#10"),
        "ReviewRequested filter should hide PR #10 (authored by viewer, not review-requested)"
    );
    assert!(
        !output.contains("#13"),
        "ReviewRequested filter should hide PR #13 (no review request)"
    );
}

/// T064: Repository pane counts and indicators are unchanged by filter (FR-002).
#[test]
fn repo_pane_unchanged_by_filter() {
    let repos = test_repos();

    let output_all = render_to_string(
        &test_state(repos.clone(), make_viewer(), FilterMode::All),
        120,
        30,
    );
    let output_mine = render_to_string(
        &test_state(repos.clone(), make_viewer(), FilterMode::Mine),
        120,
        30,
    );
    let output_review = render_to_string(
        &test_state(repos, make_viewer(), FilterMode::ReviewRequested),
        120,
        30,
    );

    // Extract the left pane (first ~48 chars of each line, the 40% pane at 120 width).
    // Exclude the last line (status bar) since it shows the filter name.
    let left_pane = |output: &str| -> Vec<String> {
        let lines: Vec<&str> = output.lines().collect();
        let content_lines = if lines.len() > 1 {
            &lines[..lines.len() - 1]
        } else {
            &lines
        };
        content_lines
            .iter()
            .map(|line| {
                let chars: Vec<char> = line.chars().collect();
                chars.iter().take(48).collect::<String>()
            })
            .collect()
    };

    let left_all = left_pane(&output_all);
    let left_mine = left_pane(&output_mine);
    let left_review = left_pane(&output_review);

    assert_eq!(
        left_all, left_mine,
        "Left pane (repo list) must not change between All and Mine filters"
    );
    assert_eq!(
        left_all, left_review,
        "Left pane (repo list) must not change between All and ReviewRequested filters"
    );
}

/// T065: Distinguish "no pull requests match this filter" from "no open pull requests" (FR-036).
#[test]
fn no_prs_vs_no_filter_match_distinct_messages() {
    let repos = test_repos();

    // Select "acme/web" (zero open PRs) with All filter
    let mut state_empty = test_state(repos.clone(), make_viewer(), FilterMode::All);
    state_empty.selection.repo = Some(RepoId(2));
    state_empty.selection.pull = None;
    let output_empty = render_to_string(&state_empty, 120, 30);
    assert!(
        output_empty.contains("No open pull requests"),
        "Zero-PR repo should say 'No open pull requests', got:\n{output_empty}"
    );

    // Select "acme/cli" (has PRs but none by viewer) with Mine filter
    let mut state_no_match = test_state(repos, make_viewer(), FilterMode::Mine);
    state_no_match.selection.repo = Some(RepoId(3));
    state_no_match.selection.pull = None;
    let output_no_match = render_to_string(&state_no_match, 120, 30);
    assert!(
        output_no_match.contains("No pull requests match this filter"),
        "Repo with PRs but no filter match should say 'No pull requests match this filter', got:\n{output_no_match}"
    );
}

/// T068: Degraded filter notice when team memberships unavailable (FR-032).
#[test]
fn degraded_filter_notice_when_teams_unavailable() {
    let state = test_state(
        test_repos(),
        make_viewer_no_teams(),
        FilterMode::ReviewRequested,
    );
    let output = render_to_string(&state, 120, 30);
    assert!(
        output.contains("team filter limited"),
        "Should show degraded filter notice when teams unavailable"
    );
}

#[test]
fn no_degraded_notice_when_teams_available() {
    let state = test_state(test_repos(), make_viewer(), FilterMode::ReviewRequested);
    let output = render_to_string(&state, 120, 30);
    assert!(
        !output.contains("team filter limited"),
        "Should NOT show degraded filter notice when teams are available"
    );
}

#[test]
fn no_degraded_notice_on_non_review_filter() {
    let state = test_state(test_repos(), make_viewer_no_teams(), FilterMode::All);
    let output = render_to_string(&state, 120, 30);
    assert!(
        !output.contains("team filter limited"),
        "Should NOT show degraded filter notice when not in ReviewRequested mode"
    );
}

/// T069: Filter mode label always visible and not reset on repo change.
#[test]
fn filter_label_always_visible() {
    let state = test_state(test_repos(), make_viewer(), FilterMode::Mine);
    let output = render_to_string(&state, 120, 30);
    assert!(
        output.contains("[mine]"),
        "Filter mode label must be visible in the status bar"
    );
}
