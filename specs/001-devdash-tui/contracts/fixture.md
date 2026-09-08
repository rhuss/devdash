# Contract: Fixture snapshot

**Feature**: [../spec.md](../spec.md) | **Data source**: [./data-source.md](./data-source.md)

The fixture snapshot is what makes the interface testable and demonstrable without
a network or a credential (FR-055 through FR-060, SC-006, SC-007).

**Location**: `fixtures/snapshot.json`, committed to the repository (FR-055).
Overridable with `--fixtures PATH`.

**Authoring**: hand-written, not recorded from a live response, per the spec's
Assumptions. A new interface state is covered by editing this file, with no traffic
capture and no re-recording when the query shape changes.

## Schema

The snapshot holds domain values, not GraphQL responses. It deserializes into the
same types the live source produces, so the fixture cannot drift from the interface
even if the GraphQL query changes.

```json
{
  "viewer": {
    "login": "octocat",
    "organizations": ["acme", "widgets-inc"],
    "teams": { "known": ["acme/platform", "acme/reviewers"] }
  },
  "repositories": [
    {
      "id": 1001,
      "owner": "acme",
      "name": "api",
      "open_count": 4,
      "pulls": [
        {
          "number": 212,
          "title": "Add retry backoff to the fetch loop",
          "author": "octocat",
          "updated_at": "2026-09-08T09:14:00Z",
          "is_draft": false,
          "url": "https://github.com/acme/api/pull/212",
          "ci": "passing",
          "review_requests": []
        }
      ]
    }
  ],
  "org_repositories": {
    "acme": [
      { "id": 1001, "owner": "acme", "name": "api", "is_archived": false },
      { "id": 1004, "owner": "acme", "name": "legacy", "is_archived": true }
    ]
  }
}
```

### Fields

| Field | Type | Notes |
|---|---|---|
| `viewer.login` | string | Drives the "authored by me" filter (FR-030) |
| `viewer.organizations` | array of string | The settings organization list (FR-016) |
| `viewer.teams` | `{"known": [...]}` or `{"unavailable": "reason"}` | Exercises both sides of FR-032 |
| `repositories[].id` | integer | `RepoId`, the tracked-set key (FR-021) |
| `repositories[].open_count` | integer | May exceed `pulls.length` to exercise R11 |
| `pulls[].ci` | `passing`\|`failing`\|`pending`\|`no_checks` | The four states of FR-004 |
| `pulls[].review_requests[]` | `{"user": "login"}` or `{"team": "org/slug"}` | Both shapes of FR-031 |
| `org_repositories` | map of organization to repository list | Settings drilldown (FR-017) |
| `org_repositories[][].is_archived` | boolean | Must be filtered out (FR-027) |

## Required coverage

FR-059 requires the snapshot to cover the states the interface has to render. The
committed fixture must contain at least the following, and a test asserts each is
present so the fixture cannot silently lose coverage.

| # | Case | Requirement covered |
|---|---|---|
| F1 | A repository with several open pull requests | FR-001, FR-002 |
| F2 | A repository with zero open pull requests | FR-007, US1 scenario 3 |
| F3 | A pull request with `ci: passing` | FR-004 |
| F4 | A pull request with `ci: failing` | FR-004 |
| F5 | A pull request with `ci: pending` | FR-004 |
| F6 | A pull request with `ci: no_checks` | FR-004, US1 scenario 5 |
| F7 | A repository whose rollup is failing because one pull request fails | FR-006, US1 scenario 4 |
| F8 | A repository where every pull request is `no_checks` | FR-006 |
| F9 | A pull request authored by `viewer.login` | FR-030 |
| F10 | A pull request with a direct review request for the viewer | FR-031 |
| F11 | A pull request with a review request for a team the viewer belongs to | FR-031, US4 scenario 6 |
| F12 | A repository with open pull requests but none matching the "mine" filter | FR-036 |
| F13 | A draft pull request | Assumptions |
| F14 | An archived repository in `org_repositories` | FR-027 |
| F15 | Two organizations, so the drilldown has something to choose between | FR-016, FR-017 |
| F16 | A repository whose `open_count` exceeds its `pulls` length | R11 |
| F17 | Enough repositories and pull requests to require scrolling | FR-010 |

## Behaviour

| Condition | Result | Requirement |
|---|---|---|
| Snapshot loads | Every `DataSource` method serves from it | FR-055 |
| Any method called | Zero network requests | FR-057 |
| File missing | `SourceError::Fixture` naming the path, exit code 2 | FR-060 |
| File malformed | `SourceError::Fixture` naming the path and the parse error, exit code 2 | FR-060 |
| Source active | `kind()` returns `Fixture`, interface says so | FR-058 |

FR-057's "no network requests" is asserted, not assumed: the fixture source is
constructed without an HTTP client at all, so a network call is a compile error
rather than a runtime surprise.

## Fixture-driven test matrix

These tests run against the fixture and require neither network nor credentials,
which is SC-007.

| Test file | Asserts | Fixture cases |
|---|---|---|
| `ci_rollup.rs` | Precedence and the empty-repository case | F2, F7, F8 |
| `filtering.rs` | Three filter modes, team requests, empty-filter message | F9, F10, F11, F12 |
| `selection.rs` | Selection survives reorder and removal | F1, F17 |
| `render_dashboard.rs` | Frame snapshots, all four symbols present without colour | F1..F8 |
| `render_settings.rs` | Organization list, drilldown, archived filtered out | F14, F15 |
| `fixture_source.rs` | Full run, no network, missing and malformed files | all |
