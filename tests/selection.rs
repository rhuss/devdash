use time::macros::datetime;

use devdash::app::state::{Pane, PrevOrdering, Selection};
use devdash::domain::ci::CiState;
use devdash::domain::pull_request::PullRequest;
use devdash::domain::repository::{RepoId, RepoStatus, Repository};

fn make_pr(number: u32, title: &str, ci: CiState) -> PullRequest {
    PullRequest {
        number,
        title: title.to_string(),
        author: "alice".to_string(),
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

/// The ordering a reconcile is measured against: repository ids as displayed,
/// and the pull numbers of the selected repository.
fn prev(repos: &[u64], pulls: &[u32]) -> PrevOrdering {
    PrevOrdering {
        repos: repos.iter().copied().map(RepoId).collect(),
        pulls: pulls.to_vec(),
    }
}

#[test]
fn selection_survives_refresh_reorder() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(2)),
        pull: Some(20),
    };

    let reordered = vec![
        make_repo(
            3,
            "acme",
            "cli",
            vec![make_pr(30, "PR C", CiState::Pending)],
        ),
        make_repo(
            1,
            "acme",
            "api",
            vec![make_pr(10, "PR A", CiState::Passing)],
        ),
        make_repo(
            2,
            "acme",
            "web",
            vec![make_pr(20, "PR B", CiState::Failing)],
        ),
    ];

    sel.reconcile(&reordered, &prev(&[1, 2, 3], &[20]));

    assert_eq!(
        sel.repo,
        Some(RepoId(2)),
        "selected repo should survive reorder"
    );
    assert_eq!(sel.pull, Some(20), "selected PR should survive reorder");
}

/// FR-045: a removed repository hands the selection to its nearest neighbour
/// in the previous ordering, not to the top of the list.
#[test]
fn selection_moves_to_the_nearest_repo_when_the_selected_one_is_removed() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(2)),
        pull: Some(20),
    };

    let remaining = vec![
        make_repo(
            1,
            "acme",
            "api",
            vec![make_pr(10, "PR A", CiState::Passing)],
        ),
        make_repo(
            3,
            "acme",
            "cli",
            vec![make_pr(30, "PR C", CiState::Pending)],
        ),
    ];

    sel.reconcile(&remaining, &prev(&[1, 2, 3], &[20]));

    assert_eq!(
        sel.repo,
        Some(RepoId(3)),
        "the row that took the removed repository's place should be selected, not the top of the list"
    );
    assert_eq!(
        sel.pull,
        Some(30),
        "the pull request pane should follow the new repository"
    );
}

/// When the selection was on the last row, the nearest survivor is above it.
#[test]
fn selection_moves_up_when_the_last_repo_is_removed() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(3)),
        pull: None,
    };

    let remaining = vec![
        make_repo(
            1,
            "acme",
            "api",
            vec![make_pr(10, "PR A", CiState::Passing)],
        ),
        make_repo(
            2,
            "acme",
            "web",
            vec![make_pr(20, "PR B", CiState::Failing)],
        ),
    ];

    sel.reconcile(&remaining, &prev(&[1, 2, 3], &[]));

    assert_eq!(
        sel.repo,
        Some(RepoId(2)),
        "with nothing below it the selection should step up one row"
    );
}

/// Without a previous ordering there is no neighbour to find, so the first
/// surviving repository is the only sensible answer.
#[test]
fn selection_falls_back_to_the_first_repo_without_a_previous_ordering() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(2)),
        pull: Some(20),
    };

    let remaining = vec![make_repo(
        1,
        "acme",
        "api",
        vec![make_pr(10, "PR A", CiState::Passing)],
    )];

    sel.reconcile(&remaining, &PrevOrdering::default());

    assert_eq!(sel.repo, Some(RepoId(1)));
}

#[test]
fn selection_becomes_none_when_all_repos_removed() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(1)),
        pull: Some(10),
    };

    sel.reconcile(&[], &prev(&[1], &[10]));

    assert_eq!(sel.repo, None);
    assert_eq!(sel.pull, None);
}

#[test]
fn pull_selection_survives_pr_reorder() {
    let mut sel = Selection {
        focus: Pane::PullRequests,
        repo: Some(RepoId(1)),
        pull: Some(11),
    };

    let repos = vec![make_repo(
        1,
        "acme",
        "api",
        vec![
            make_pr(12, "PR C", CiState::Passing),
            make_pr(11, "PR B", CiState::Failing),
            make_pr(10, "PR A", CiState::Pending),
        ],
    )];

    sel.reconcile(&repos, &prev(&[1], &[11, 12, 10]));

    assert_eq!(sel.repo, Some(RepoId(1)));
    assert_eq!(sel.pull, Some(11), "selected PR should survive reorder");
}

/// FR-045 / US5 scenario 10: a closed pull request hands the selection to the
/// row that takes its place, not to the top of the list.
#[test]
fn pull_selection_moves_to_the_nearest_pr_when_the_selected_one_closes() {
    let mut sel = Selection {
        focus: Pane::PullRequests,
        repo: Some(RepoId(1)),
        pull: Some(11),
    };

    let repos = vec![make_repo(
        1,
        "acme",
        "api",
        vec![
            make_pr(10, "PR A", CiState::Passing),
            make_pr(12, "PR C", CiState::Pending),
        ],
    )];

    // #11 sat between #10 and #12 before it closed.
    sel.reconcile(&repos, &prev(&[1], &[10, 11, 12]));

    assert_eq!(sel.repo, Some(RepoId(1)));
    assert_eq!(
        sel.pull,
        Some(12),
        "the selection should land on the neighbour below, not jump to the top"
    );
}

/// The pull request above is the nearest survivor when nothing follows.
#[test]
fn pull_selection_moves_up_when_the_last_pr_closes() {
    let mut sel = Selection {
        focus: Pane::PullRequests,
        repo: Some(RepoId(1)),
        pull: Some(12),
    };

    let repos = vec![make_repo(
        1,
        "acme",
        "api",
        vec![
            make_pr(10, "PR A", CiState::Passing),
            make_pr(11, "PR B", CiState::Failing),
        ],
    )];

    sel.reconcile(&repos, &prev(&[1], &[10, 11, 12]));

    assert_eq!(sel.pull, Some(11));
}
