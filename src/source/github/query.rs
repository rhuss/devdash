#![allow(clippy::needless_raw_string_hashes)]

use crate::domain::TrackedRepo;

fn gql_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

const PR_NODE_FIELDS: &str = r#"
        number
        title
        updatedAt
        isDraft
        url
        author { login }
        reviewRequests(first: 20) {
          nodes {
            requestedReviewer {
              __typename
              ... on User { login }
              ... on Team { slug }
            }
          }
        }
        commits(last: 1) {
          nodes { commit { statusCheckRollup { state } } }
        }
"#;

pub fn viewer_query() -> String {
    r#"query Viewer {
  viewer {
    login
    organizations(first: 100) { nodes { login } }
  }
  rateLimit { limit cost remaining resetAt }
}"#
    .to_string()
}

pub fn teams_query(orgs: &[String], login: &str) -> String {
    if orgs.is_empty() {
        return r#"query Teams { rateLimit { cost remaining } }"#.to_string();
    }

    let login = gql_escape(login);
    let aliases: Vec<String> = orgs
        .iter()
        .enumerate()
        .map(|(i, org)| {
            let org = gql_escape(org);
            format!(
                r#"  o{i}: organization(login: "{org}") {{
    teams(first: 100, userLogins: ["{login}"]) {{ nodes {{ slug }} }}
  }}"#
            )
        })
        .collect();

    format!(
        "query Teams {{\n{}\n  rateLimit {{ cost remaining }}\n}}",
        aliases.join("\n")
    )
}

pub fn dashboard_query(tracked: &[TrackedRepo]) -> String {
    if tracked.is_empty() {
        return r#"query Dashboard { rateLimit { cost remaining resetAt } }"#.to_string();
    }

    let aliases: Vec<String> = tracked
        .iter()
        .enumerate()
        .map(|(i, repo)| {
            let owner = gql_escape(&repo.owner);
            let name = gql_escape(&repo.name);
            format!(
                r#"  r{i}: repository(owner: "{owner}", name: "{name}") {{
    databaseId
    nameWithOwner
    pullRequests(states: OPEN, first: 100, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
      totalCount
      pageInfo {{ hasNextPage endCursor }}
      nodes {{{PR_NODE_FIELDS}      }}
    }}
  }}"#
            )
        })
        .collect();

    format!(
        "query Dashboard {{\n{}\n  rateLimit {{ cost remaining resetAt }}\n}}",
        aliases.join("\n")
    )
}

pub fn pagination_query(owner: &str, name: &str, after: &str) -> String {
    let owner = gql_escape(owner);
    let name = gql_escape(name);
    let after = gql_escape(after);
    format!(
        r#"query DashboardPage {{
  repository(owner: "{owner}", name: "{name}") {{
    databaseId
    pullRequests(states: OPEN, first: 100, after: "{after}", orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
      pageInfo {{ hasNextPage endCursor }}
      nodes {{{PR_NODE_FIELDS}      }}
    }}
  }}
  rateLimit {{ cost remaining }}
}}"#
    )
}

pub fn org_repos_query(org: &str, after: Option<&str>) -> String {
    let org = gql_escape(org);
    let after_arg = match after {
        Some(cursor) => format!(r#", after: "{}""#, gql_escape(cursor)),
        None => String::new(),
    };

    format!(
        r#"query OrgRepos {{
  repositoryOwner(login: "{org}") {{
    repositories(first: 100{after_arg}, orderBy: {{field: NAME, direction: ASC}}) {{
      pageInfo {{ hasNextPage endCursor }}
      nodes {{ databaseId nameWithOwner isArchived isFork }}
    }}
  }}
  rateLimit {{ cost remaining }}
}}"#
    )
}
