# Contract: GitHub GraphQL queries

**Feature**: [../spec.md](../spec.md) | **Research**: [../research.md](../research.md)

Endpoint: `POST https://api.github.com/graphql`
Headers: `Authorization: bearer <token>`, `User-Agent: devdash/<version>`

Every query below was executed against the live API on 2026-09-08 and the shapes
here are what it returned, not what the documentation implies.

## Q1: Startup, viewer identity and teams

Satisfies FR-040 (identity and team memberships) and FR-016 (organization list).

```graphql
query Viewer {
  viewer {
    login
    organizations(first: 100) { nodes { login } }
  }
  rateLimit { limit cost remaining resetAt }
}
```

Measured cost: **1 point**.

Team memberships require a second query, aliased across the organizations returned
above. `userLogins` filters to the viewer's own memberships:

```graphql
query Teams {
  o0: organization(login: "openshift") {
    teams(first: 100, userLogins: ["rhuss"]) { nodes { slug } }
  }
  o1: organization(login: "fabric8io") {
    teams(first: 100, userLogins: ["rhuss"]) { nodes { slug } }
  }
  rateLimit { cost remaining }
}
```

Measured: returns real memberships (3, 5 and 1 team across three organizations),
cost 1 point per organization, aliasable into one request.

**Failure handling**: if this query fails for lack of `read:org`, the viewer's
teams become `TeamMemberships::Unavailable` and FR-032's degraded filter applies.
The failure must not prevent the dashboard from loading.

## Q2: Refresh, all tracked repositories in one request

The hot path. Satisfies FR-002, FR-003, FR-004, FR-006 and FR-031.

One alias per tracked repository:

```graphql
query Dashboard {
  r0: repository(owner: "ratatui", name: "ratatui") {
    databaseId
    nameWithOwner
    pullRequests(states: OPEN, first: 100,
                 orderBy: {field: UPDATED_AT, direction: DESC}) {
      totalCount
      pageInfo { hasNextPage endCursor }
      nodes {
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
      }
    }
  }
  r1: repository(owner: "rust-lang", name: "cargo") { ...same shape... }
  rateLimit { cost remaining resetAt }
}
```

**Measured**: 3 repositories, 60 pull requests with full CI rollup, **1 HTTP
request, cost 1 point** of a 5000/hour budget. This is the finding that makes
SC-004 comfortable and the refresh interval a matter of taste rather than necessity.

### Field mapping

| GraphQL path | Domain field | Requirement |
|---|---|---|
| `databaseId` | `RepoId` | FR-021, FR-022 |
| `nameWithOwner` | `owner`, `name`, split on `/` | FR-022 |
| `pullRequests.totalCount` | `open_count` | FR-002, R11 |
| `nodes[].number` | `PullRequest.number` | FR-003 |
| `nodes[].title` | `PullRequest.title` | FR-003 |
| `nodes[].author.login` | `PullRequest.author` | FR-003 |
| `nodes[].updatedAt` | `PullRequest.updated_at` | FR-014 |
| `nodes[].url` | `PullRequest.url` | FR-061 |
| `nodes[].reviewRequests` | `Vec<ReviewRequest>` | FR-031 |
| `nodes[].commits.nodes[0].commit.statusCheckRollup.state` | `CiState` | FR-004 |

### Response quirks, observed in live data

These are not hypothetical. Each was seen while researching this contract.

| Quirk | Handling |
|---|---|
| `statusCheckRollup` is `null` when the head commit has no checks | Map to `CiState::NoChecks` (FR-004's fourth state). Do not treat as an error. |
| `requestedReviewer` can be `null` inside a non-empty `reviewRequests.nodes` | Skip that entry. Do not fail the repository. |
| `author` can be `null` for a deleted account | Render as `ghost`, matching GitHub's own convention. |
| `commits.nodes` can be empty | Map to `CiState::NoChecks`. |
| A renamed repository resolves via redirect and returns its **current** `nameWithOwner` | Match on `databaseId`, rewrite stored owner and name (FR-022). |
| A repository the token cannot read yields a `null` alias plus an entry in top-level `errors` | Map to that repository's inner `Err`, leaving the others intact (FR-053). |

The last one matters most: GraphQL returns HTTP 200 with a partial `data` object
and a sibling `errors` array. Treating a non-empty `errors` array as total failure
would violate FR-053. Each error's `path` identifies the alias, and therefore the
repository, that failed.

## Q3: Pagination for large repositories

Issued only when `pageInfo.hasNextPage` is true, which R11 showed is reachable for
ordinary repositories (`rust-lang/cargo` had 90 open pull requests, `ratatui/ratatui`
had 76, against a page size of 100).

```graphql
query DashboardPage($owner: String!, $name: String!, $after: String!) {
  repository(owner: $owner, name: $name) {
    databaseId
    pullRequests(states: OPEN, first: 100, after: $after,
                 orderBy: {field: UPDATED_AT, direction: DESC}) {
      pageInfo { hasNextPage endCursor }
      nodes { ...same node shape as Q2... }
    }
  }
  rateLimit { cost remaining }
}
```

Required for correctness of FR-006, not for the count: `totalCount` from Q2 is
already exact. Without this, a repository whose only failing pull request sits
beyond the first page would show a passing indicator, which is precisely the lie
SC-002 depends on not telling.

## Q4: Settings, repositories within an organization

Satisfies FR-017 and FR-027.

```graphql
query OrgRepos($login: String!, $after: String) {
  organization(login: $login) {
    repositories(first: 100, after: $after,
                 orderBy: {field: NAME, direction: ASC}) {
      pageInfo { hasNextPage endCursor }
      nodes { databaseId nameWithOwner isArchived isFork }
    }
  }
  rateLimit { cost remaining }
}
```

`isArchived` filters per FR-027. `isFork` is carried but not filtered, since forks
are included per the spec's Assumptions.

The viewer's personal account uses `user(login:)` with the same `repositories`
shape, because FR-016 presents it alongside the organizations.

Pagination here is user-visible: FR-018 requires a loading state while it runs, and
FR-044's progressive population applies, so pages should be rendered as they arrive
rather than buffered until the last one.

## Rate limiting

Every query requests `rateLimit { cost remaining resetAt }`, which costs nothing
extra and gives FR-051 the reset time it must display.

| Condition | Action | Requirement |
|---|---|---|
| `remaining` is 0, or HTTP 403 with a rate-limit body | Surface `SourceError::RateLimited { resets_at }`, suspend automatic refresh until `resetAt` | FR-051 |
| `remaining` low but non-zero | Log a warning, continue | FR-063 |

Measured budget: 5000 points per hour, and a full refresh costs 1. At the default
five-minute interval that is 12 points per hour, roughly 0.2 percent of the budget.
The specification's concern about rate-limit headroom at 20 repositories does not
materialise under this design.

## Security

The token appears only in the `Authorization` header. It is never logged, never
placed in a tracing field, and never included in an error message (FR-038, SC-008).
Request bodies may be logged at `debug` level; headers must not be.
