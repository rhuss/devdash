pub mod query;
pub mod response;

use async_trait::async_trait;
use reqwest::Client;

use crate::domain::repository::{OrgRepo, RepoId, TrackedRepo};
use crate::domain::viewer::{TeamMemberships, Viewer};
use crate::source::{DataSource, RepositoryData, SourceError, SourceKind};

pub struct GithubDataSource {
    client: Client,
    token: String,
}

impl GithubDataSource {
    pub fn new(token: String) -> Self {
        let client = Client::builder()
            .user_agent("devdash/0.1.0")
            .build()
            .expect("failed to build HTTP client");
        Self { client, token }
    }

    async fn execute_query(&self, query_str: &str) -> Result<serde_json::Value, SourceError> {
        let resp = self
            .client
            .post("https://api.github.com/graphql")
            .header("Authorization", format!("bearer {}", self.token))
            .json(&serde_json::json!({"query": query_str}))
            .send()
            .await
            .map_err(|e| SourceError::Network(e.to_string()))?;

        let status = resp.status();

        if status.as_u16() == 401 {
            return Err(SourceError::Unauthorized {
                scope: "GraphQL API".into(),
            });
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SourceError::Malformed(e.to_string()))?;

        if status.as_u16() == 403 {
            if let Some(reset_at) = response::check_rate_limit(&body) {
                return Err(SourceError::RateLimited {
                    resets_at: reset_at,
                });
            }
            return Err(SourceError::Unauthorized {
                scope: "GraphQL API (403)".into(),
            });
        }

        if let Some(reset_at) = response::check_rate_limit(&body) {
            return Err(SourceError::RateLimited {
                resets_at: reset_at,
            });
        }

        Ok(body)
    }
}

#[async_trait]
impl DataSource for GithubDataSource {
    async fn viewer(&self) -> Result<Viewer, SourceError> {
        let q = query::viewer_query();
        let body = self.execute_query(&q).await?;
        let (login, orgs) = response::parse_viewer(&body)?;

        let teams = if orgs.is_empty() {
            TeamMemberships::Known { known: Vec::new() }
        } else {
            let teams_q = query::teams_query(&orgs, &login);
            match self.execute_query(&teams_q).await {
                Ok(teams_body) => response::parse_teams(&teams_body, &orgs),
                Err(_) => TeamMemberships::Unavailable {
                    unavailable: "could not read team memberships (may need read:org scope)".into(),
                },
            }
        };

        Ok(Viewer {
            login,
            organizations: orgs,
            teams,
        })
    }

    async fn dashboard(&self, tracked: &[TrackedRepo]) -> Result<Vec<RepositoryData>, SourceError> {
        if tracked.is_empty() {
            return Ok(Vec::new());
        }

        let q = query::dashboard_query(tracked);
        let body = self.execute_query(&q).await?;

        let aliases: Vec<(RepoId, String, String)> = tracked
            .iter()
            .map(|t| (t.id, t.owner.clone(), t.name.clone()))
            .collect();

        let mut dash_response = response::parse_dashboard(&body, tracked.len(), &aliases);

        // T043: Paginate repos that have more PRs than the first page.
        for (i, repo_data) in dash_response.repositories.iter_mut().enumerate() {
            if repo_data.result.is_err() {
                continue;
            }

            let alias = format!("r{i}");
            let data = body.get("data").and_then(|d| d.get(&alias));
            if let Some(repo_json) = data {
                let prs_data = &repo_json["pullRequests"];
                let mut page_info = response::parse_page_info(prs_data);

                while page_info.has_next_page {
                    let cursor = match &page_info.end_cursor {
                        Some(c) => c.clone(),
                        None => break,
                    };

                    let (owner, name) = match &repo_data.result {
                        Ok(_) => (repo_data.owner.clone(), repo_data.name.clone()),
                        Err(_) => break,
                    };

                    let page_q = query::pagination_query(&owner, &name, &cursor);
                    let page_body = match self.execute_query(&page_q).await {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::warn!("Pagination failed for {owner}/{name}: {e}");
                            break;
                        }
                    };

                    match response::parse_pagination_response(&page_body) {
                        Ok((mut more_pulls, next_page)) => {
                            if let Ok(ref mut payload) = repo_data.result {
                                payload.pulls.append(&mut more_pulls);
                            }
                            page_info = next_page;
                        }
                        Err(e) => {
                            tracing::warn!("Pagination parse failed for {owner}/{name}: {e}");
                            break;
                        }
                    }
                }
            }
        }

        Ok(dash_response.repositories)
    }

    async fn org_repositories(&self, org: &str) -> Result<Vec<OrgRepo>, SourceError> {
        let mut all_repos = Vec::new();
        let mut after: Option<String> = None;

        loop {
            let q = query::org_repos_query(org, after.as_deref());
            let body = self.execute_query(&q).await?;
            let (repos, page_info) = response::parse_org_repos(&body)?;

            all_repos.extend(repos);

            if page_info.has_next_page && page_info.end_cursor.is_some() {
                after = page_info.end_cursor;
            } else {
                break;
            }
        }

        Ok(all_repos)
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Live
    }
}
