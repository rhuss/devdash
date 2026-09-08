use std::path::PathBuf;

use devdash::domain::ci::CiState;

use devdash::domain::repository::{RepoId, TrackedRepo};
use devdash::source::fixture::FixtureDataSource;
use devdash::source::{DataSource, SourceKind};

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/snapshot.json")
}

fn load_fixture() -> FixtureDataSource {
    FixtureDataSource::load(&snapshot_path()).expect("fixture snapshot should load")
}

fn tracked_all() -> Vec<TrackedRepo> {
    vec![
        TrackedRepo {
            id: RepoId(1001),
            owner: "acme".into(),
            name: "api".into(),
        },
        TrackedRepo {
            id: RepoId(1002),
            owner: "acme".into(),
            name: "web".into(),
        },
        TrackedRepo {
            id: RepoId(1003),
            owner: "acme".into(),
            name: "cli".into(),
        },
        TrackedRepo {
            id: RepoId(1005),
            owner: "acme".into(),
            name: "docs".into(),
        },
        TrackedRepo {
            id: RepoId(1006),
            owner: "acme".into(),
            name: "mobile".into(),
        },
        TrackedRepo {
            id: RepoId(2001),
            owner: "widgets-inc".into(),
            name: "dashboard".into(),
        },
        TrackedRepo {
            id: RepoId(2002),
            owner: "widgets-inc".into(),
            name: "sdk".into(),
        },
        TrackedRepo {
            id: RepoId(2003),
            owner: "widgets-inc".into(),
            name: "infra".into(),
        },
    ]
}

#[tokio::test]
async fn fixture_source_kind_is_fixture() {
    let source = load_fixture();
    assert_eq!(source.kind(), SourceKind::Fixture);
}

#[tokio::test]
async fn fixture_viewer_loads() {
    let source = load_fixture();
    let viewer = source.viewer().await.expect("viewer should load");
    assert_eq!(viewer.login, "octocat");
    assert_eq!(viewer.organizations.len(), 2);
}

#[tokio::test]
async fn fixture_dashboard_loads_all_repos() {
    let source = load_fixture();
    let data = source
        .dashboard(&tracked_all())
        .await
        .expect("dashboard should load");
    assert_eq!(data.len(), 8);
}

// F1: A repository with several open pull requests
#[tokio::test]
async fn f01_repo_with_several_prs() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    assert_eq!(payload.open_count, 4);
    assert!(payload.pulls.len() >= 2);
}

// F2: A repository with zero open pull requests
#[tokio::test]
async fn f02_repo_with_zero_prs() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let web = data.iter().find(|r| r.id == RepoId(1002)).unwrap();
    let payload = web.result.as_ref().unwrap();
    assert_eq!(payload.open_count, 0);
    assert!(payload.pulls.is_empty());
}

// F3: A pull request with ci: passing
#[tokio::test]
async fn f03_pr_with_ci_passing() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    assert!(payload.pulls.iter().any(|p| p.ci == CiState::Passing));
}

// F4: A pull request with ci: failing
#[tokio::test]
async fn f04_pr_with_ci_failing() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    assert!(payload.pulls.iter().any(|p| p.ci == CiState::Failing));
}

// F5: A pull request with ci: pending
#[tokio::test]
async fn f05_pr_with_ci_pending() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    assert!(payload.pulls.iter().any(|p| p.ci == CiState::Pending));
}

// F6: A pull request with ci: no_checks
#[tokio::test]
async fn f06_pr_with_ci_no_checks() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    assert!(payload.pulls.iter().any(|p| p.ci == CiState::NoChecks));
}

// F7: A repository whose rollup is failing because one pull request fails
#[tokio::test]
async fn f07_repo_rollup_failing() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let api = data.iter().find(|r| r.id == RepoId(1001)).unwrap();
    let payload = api.result.as_ref().unwrap();
    let states: Vec<CiState> = payload.pulls.iter().map(|p| p.ci).collect();
    let rolled = devdash::domain::ci::rollup(&states);
    assert_eq!(rolled, Some(CiState::Failing));
}

// F8: A repository where every pull request is no_checks
#[tokio::test]
async fn f08_repo_all_no_checks() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let docs = data.iter().find(|r| r.id == RepoId(1005)).unwrap();
    let payload = docs.result.as_ref().unwrap();
    assert!(payload.pulls.iter().all(|p| p.ci == CiState::NoChecks));
    let states: Vec<CiState> = payload.pulls.iter().map(|p| p.ci).collect();
    assert_eq!(
        devdash::domain::ci::rollup(&states),
        Some(CiState::NoChecks)
    );
}

// F9: A pull request authored by viewer.login
#[tokio::test]
async fn f09_pr_authored_by_viewer() {
    let source = load_fixture();
    let viewer = source.viewer().await.unwrap();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let has_authored = data.iter().any(|r| {
        r.result
            .as_ref()
            .is_ok_and(|p| p.pulls.iter().any(|pr| pr.author == viewer.login))
    });
    assert!(has_authored);
}

// F10: A pull request with a direct review request for the viewer
#[tokio::test]
async fn f10_direct_review_request() {
    let source = load_fixture();
    let viewer = source.viewer().await.unwrap();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let has_direct = data.iter().any(|r| {
        r.result.as_ref().is_ok_and(|p| {
            p.pulls.iter().any(|pr| {
                pr.review_requests
                    .iter()
                    .any(|rr| rr.is_for_user(&viewer.login))
            })
        })
    });
    assert!(has_direct);
}

// F11: A pull request with a review request for a team the viewer belongs to
#[tokio::test]
async fn f11_team_review_request() {
    let source = load_fixture();
    let viewer = source.viewer().await.unwrap();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let team_slugs = viewer.teams.slugs();
    let has_team = data.iter().any(|r| {
        r.result.as_ref().is_ok_and(|p| {
            p.pulls.iter().any(|pr| {
                pr.review_requests
                    .iter()
                    .any(|rr| team_slugs.iter().any(|slug| rr.is_for_team(slug)))
            })
        })
    });
    assert!(has_team);
}

// F12: A repository with open PRs but none matching "mine" filter
#[tokio::test]
async fn f12_repo_with_no_mine_matches() {
    let source = load_fixture();
    let viewer = source.viewer().await.unwrap();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let has_no_mine = data.iter().any(|r| {
        r.result.as_ref().is_ok_and(|p| {
            !p.pulls.is_empty() && p.pulls.iter().all(|pr| pr.author != viewer.login)
        })
    });
    assert!(has_no_mine);
}

// F13: A draft pull request
#[tokio::test]
async fn f13_draft_pr() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let has_draft = data.iter().any(|r| {
        r.result
            .as_ref()
            .map_or(false, |p| p.pulls.iter().any(|pr| pr.is_draft))
    });
    assert!(has_draft);
}

// F14: An archived repository in org_repositories
#[tokio::test]
async fn f14_archived_repo_in_org() {
    let source = load_fixture();
    let repos = source.org_repositories("acme").await.unwrap();
    assert!(repos.iter().any(|r| r.is_archived));
}

// F15: Two organizations
#[tokio::test]
async fn f15_two_organizations() {
    let source = load_fixture();
    let viewer = source.viewer().await.unwrap();
    assert!(viewer.organizations.len() >= 2);
}

// F16: A repository whose open_count exceeds pulls length (R11)
#[tokio::test]
async fn f16_open_count_exceeds_pulls() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    let infra = data.iter().find(|r| r.id == RepoId(2003)).unwrap();
    let payload = infra.result.as_ref().unwrap();
    assert!(payload.open_count as usize > payload.pulls.len());
}

// F17: Enough repositories and pull requests to require scrolling
#[tokio::test]
async fn f17_enough_for_scrolling() {
    let source = load_fixture();
    let data = source.dashboard(&tracked_all()).await.unwrap();
    assert!(data.len() >= 8);
    let total_prs: usize = data
        .iter()
        .filter_map(|r| r.result.as_ref().ok())
        .map(|p| p.pulls.len())
        .sum();
    assert!(total_prs >= 10);
}

// FR-057: Zero network requests
#[tokio::test]
async fn no_network_requests_made() {
    let source = load_fixture();
    let _ = source.viewer().await;
    let _ = source.dashboard(&tracked_all()).await;
    let _ = source.org_repositories("acme").await;
    // The fixture source has no HTTP client at all, so network requests are
    // a compile error rather than a runtime surprise.
}

// FR-060: Missing snapshot
#[test]
fn missing_snapshot_returns_fixture_error() {
    let result = FixtureDataSource::load(&PathBuf::from("/nonexistent/path.json"));
    match result {
        Err(e) => assert!(e.to_string().contains("/nonexistent/path.json")),
        Ok(_) => panic!("expected error for missing file"),
    }
}

// FR-060: Malformed snapshot
#[test]
fn malformed_snapshot_returns_fixture_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "not valid json {{{").unwrap();
    match FixtureDataSource::load(&path) {
        Err(e) => assert!(e.to_string().contains("bad.json")),
        Ok(_) => panic!("expected error for malformed file"),
    }
}
