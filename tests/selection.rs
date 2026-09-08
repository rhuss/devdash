use time::macros::datetime;

use devdash::app::state::{Pane, Selection};
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

    sel.reconcile(&reordered);

    assert_eq!(
        sel.repo,
        Some(RepoId(2)),
        "selected repo should survive reorder"
    );
    assert_eq!(sel.pull, Some(20), "selected PR should survive reorder");
}

#[test]
fn selection_falls_back_when_repo_removed() {
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

    sel.reconcile(&remaining);

    assert!(sel.repo.is_some(), "should fall back to a surviving repo");
    assert_ne!(
        sel.repo,
        Some(RepoId(2)),
        "removed repo should not remain selected"
    );
}

#[test]
fn selection_becomes_none_when_all_repos_removed() {
    let mut sel = Selection {
        focus: Pane::Repositories,
        repo: Some(RepoId(1)),
        pull: Some(10),
    };

    sel.reconcile(&[]);

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

    sel.reconcile(&repos);

    assert_eq!(sel.repo, Some(RepoId(1)));
    assert_eq!(sel.pull, Some(11), "selected PR should survive reorder");
}

#[test]
fn pull_selection_falls_back_when_pr_removed() {
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

    sel.reconcile(&repos);

    assert_eq!(sel.repo, Some(RepoId(1)));
    assert!(sel.pull.is_some(), "should fall back to a surviving PR");
    assert_ne!(sel.pull, Some(11), "removed PR should not remain selected");
}
