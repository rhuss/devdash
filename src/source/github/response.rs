use serde_json::Value;
use time::OffsetDateTime;

use crate::domain::ci::CiState;
use crate::domain::pull_request::{PullRequest, ReviewRequest};
use crate::domain::repository::{OrgRepo, RepoId};
use crate::domain::viewer::TeamMemberships;
use crate::source::{RepoPayload, RepositoryData, SourceError};

pub struct DashboardResponse {
    pub repositories: Vec<RepositoryData>,
}

pub struct PageInfo {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

pub fn parse_viewer(body: &Value) -> Result<(String, Vec<String>), SourceError> {
    let data = body
        .get("data")
        .ok_or_else(|| SourceError::Malformed("no data in viewer response".into()))?;

    let viewer = data
        .get("viewer")
        .ok_or_else(|| SourceError::Malformed("no viewer in response".into()))?;

    let login = viewer["login"]
        .as_str()
        .ok_or_else(|| SourceError::Malformed("viewer.login missing".into()))?
        .to_string();

    let orgs: Vec<String> = viewer["organizations"]["nodes"]
        .as_array()
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|n| n["login"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok((login, orgs))
}

pub fn parse_teams(body: &Value, orgs: &[String]) -> TeamMemberships {
    let Some(data) = body.get("data") else {
        return TeamMemberships::Unavailable {
            unavailable: "no data in teams response".into(),
        };
    };

    if body.get("errors").is_some() {
        let msg = body["errors"][0]["message"]
            .as_str()
            .unwrap_or("unknown error");
        if msg.contains("authorize") || msg.contains("scope") || msg.contains("permission") {
            return TeamMemberships::Unavailable {
                unavailable: format!("insufficient scope: {msg}"),
            };
        }
    }

    let mut slugs = Vec::new();
    for (i, org) in orgs.iter().enumerate() {
        let alias = format!("o{i}");
        if let Some(org_data) = data.get(&alias) {
            if let Some(nodes) = org_data["teams"]["nodes"].as_array() {
                for node in nodes {
                    if let Some(slug) = node["slug"].as_str() {
                        slugs.push(format!("{org}/{slug}"));
                    }
                }
            }
        }
    }

    TeamMemberships::Known { known: slugs }
}

pub fn parse_dashboard(
    body: &Value,
    repo_count: usize,
    aliases: &[(RepoId, String, String)],
) -> DashboardResponse {
    let data = body.get("data").unwrap_or(&Value::Null);
    let errors = body.get("errors").and_then(|e| e.as_array());

    let error_aliases: std::collections::HashSet<String> = errors
        .map(|errs| {
            errs.iter()
                .filter_map(|e| e["path"].as_array())
                .filter_map(|path| path.first())
                .filter_map(|p| p.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut repositories = Vec::with_capacity(repo_count);

    for (i, (id, owner, name)) in aliases.iter().enumerate() {
        let alias = format!("r{i}");

        if error_aliases.contains(&alias) {
            let reason = errors
                .and_then(|errs| {
                    errs.iter().find(|e| {
                        e["path"]
                            .as_array()
                            .and_then(|p| p.first())
                            .and_then(|p| p.as_str())
                            == Some(&alias)
                    })
                })
                .and_then(|e| e["message"].as_str())
                .unwrap_or("unknown error")
                .to_string();

            repositories.push(RepositoryData {
                id: *id,
                owner: owner.clone(),
                name: name.clone(),
                result: Err(SourceError::Repository { id: *id, reason }),
            });
            continue;
        }

        let repo_data = match data.get(&alias) {
            Some(Value::Null) | None => {
                repositories.push(RepositoryData {
                    id: *id,
                    owner: owner.clone(),
                    name: name.clone(),
                    result: Err(SourceError::Repository {
                        id: *id,
                        reason: "repository not accessible".into(),
                    }),
                });
                continue;
            }
            Some(d) => d,
        };

        let (actual_owner, actual_name) = parse_name_with_owner(
            repo_data["nameWithOwner"].as_str().unwrap_or(""),
            owner,
            name,
        );

        let prs_data = &repo_data["pullRequests"];
        #[allow(clippy::cast_possible_truncation)]
        let total_count = prs_data["totalCount"].as_u64().unwrap_or(0) as u32;
        let pulls = parse_pull_requests(prs_data);
        let page_info = parse_page_info(prs_data);

        repositories.push(RepositoryData {
            id: RepoId(repo_data["databaseId"].as_u64().unwrap_or(id.0)),
            owner: actual_owner,
            name: actual_name,
            result: Ok(RepoPayload {
                open_count: total_count,
                pulls,
            }),
        });

        let _ = page_info; // pagination handled by caller
    }

    DashboardResponse { repositories }
}

pub fn parse_page_info(prs_data: &Value) -> PageInfo {
    let pi = &prs_data["pageInfo"];
    PageInfo {
        has_next_page: pi["hasNextPage"].as_bool().unwrap_or(false),
        end_cursor: pi["endCursor"].as_str().map(String::from),
    }
}

pub fn parse_pagination_response(
    body: &Value,
) -> Result<(Vec<PullRequest>, PageInfo), SourceError> {
    let data = body
        .get("data")
        .ok_or_else(|| SourceError::Malformed("no data in pagination response".into()))?;

    let repo = data
        .get("repository")
        .ok_or_else(|| SourceError::Malformed("no repository in pagination response".into()))?;

    let prs_data = &repo["pullRequests"];
    let pulls = parse_pull_requests(prs_data);
    let page_info = parse_page_info(prs_data);

    Ok((pulls, page_info))
}

pub fn parse_pull_requests(prs_data: &Value) -> Vec<PullRequest> {
    let Some(nodes) = prs_data["nodes"].as_array() else {
        return Vec::new();
    };

    nodes.iter().filter_map(parse_pr_node).collect()
}

fn parse_pr_node(node: &Value) -> Option<PullRequest> {
    #[allow(clippy::cast_possible_truncation)]
    let number = node["number"].as_u64()? as u32;
    let title = node["title"].as_str().unwrap_or("").to_string();
    let author = node["author"]["login"]
        .as_str()
        .unwrap_or("ghost")
        .to_string();
    let updated_at_str = node["updatedAt"].as_str()?;
    let updated_at = OffsetDateTime::parse(
        updated_at_str,
        &time::format_description::well_known::Rfc3339,
    )
    .ok()?;
    let is_draft = node["isDraft"].as_bool().unwrap_or(false);
    let url = node["url"].as_str().unwrap_or("").to_string();

    let ci = parse_ci_state(node);
    let review_requests = parse_review_requests(node);

    Some(PullRequest {
        number,
        title,
        author,
        updated_at,
        is_draft,
        url,
        ci,
        review_requests,
    })
}

fn parse_ci_state(node: &Value) -> CiState {
    let commits = &node["commits"]["nodes"];
    let commit_nodes = match commits.as_array() {
        Some(n) if !n.is_empty() => n,
        _ => return CiState::NoChecks,
    };

    let state = commit_nodes[0]["commit"]["statusCheckRollup"]["state"].as_str();
    CiState::from_graphql_state(state)
}

fn parse_review_requests(node: &Value) -> Vec<ReviewRequest> {
    let Some(nodes) = node["reviewRequests"]["nodes"].as_array() else {
        return Vec::new();
    };

    nodes
        .iter()
        .filter_map(|rr| {
            let reviewer = &rr["requestedReviewer"];
            if reviewer.is_null() {
                return None;
            }
            let typename = reviewer["__typename"].as_str()?;
            match typename {
                "User" => {
                    let login = reviewer["login"].as_str()?.to_string();
                    Some(ReviewRequest::User { user: login })
                }
                "Team" => {
                    let slug = reviewer["slug"].as_str()?.to_string();
                    Some(ReviewRequest::Team { team: slug })
                }
                _ => None,
            }
        })
        .collect()
}

fn parse_name_with_owner(
    name_with_owner: &str,
    fallback_owner: &str,
    fallback_name: &str,
) -> (String, String) {
    if let Some((owner, name)) = name_with_owner.split_once('/') {
        (owner.to_string(), name.to_string())
    } else {
        (fallback_owner.to_string(), fallback_name.to_string())
    }
}

pub fn parse_org_repos(body: &Value) -> Result<(Vec<OrgRepo>, PageInfo), SourceError> {
    let data = body
        .get("data")
        .ok_or_else(|| SourceError::Malformed("no data in org repos response".into()))?;

    let owner = data
        .get("repositoryOwner")
        .ok_or_else(|| SourceError::Malformed("no repositoryOwner in response".into()))?;

    let repos_data = &owner["repositories"];
    let page_info = parse_page_info_generic(repos_data);

    let nodes = repos_data["nodes"]
        .as_array()
        .ok_or_else(|| SourceError::Malformed("no nodes in repositories".into()))?;

    let repos: Vec<OrgRepo> = nodes
        .iter()
        .filter_map(|node| {
            let db_id = node["databaseId"].as_u64()?;
            let (owner, name) =
                parse_name_with_owner(node["nameWithOwner"].as_str().unwrap_or(""), "", "");
            Some(OrgRepo {
                id: RepoId(db_id),
                owner,
                name,
                is_archived: node["isArchived"].as_bool().unwrap_or(false),
                is_fork: node["isFork"].as_bool().unwrap_or(false),
            })
        })
        .collect();

    Ok((repos, page_info))
}

fn parse_page_info_generic(data: &Value) -> PageInfo {
    let pi = &data["pageInfo"];
    PageInfo {
        has_next_page: pi["hasNextPage"].as_bool().unwrap_or(false),
        end_cursor: pi["endCursor"].as_str().map(String::from),
    }
}

pub fn check_rate_limit(body: &Value) -> Option<OffsetDateTime> {
    let rl = body.get("data")?.get("rateLimit")?;
    let remaining = rl["remaining"].as_u64()?;
    if remaining == 0 {
        let reset_str = rl["resetAt"].as_str()?;
        OffsetDateTime::parse(reset_str, &time::format_description::well_known::Rfc3339).ok()
    } else {
        None
    }
}
