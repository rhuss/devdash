# Data Model: devdash TUI

**Date**: 2026-09-08
**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Types are given in Rust. Because this project doubles as a teaching example, each
Rust construct is explained the first time it appears.

## Domain types

### CiState

The four states from FR-004, as a closed set.

```rust
/// An `enum` in Rust is a sum type: a value is exactly one of these variants,
/// never several and never none. In Java or TypeScript this would be an enum or
/// a union type, but the compiler here forces every `match` to handle all four,
/// so a fifth state cannot be added without every consumer being updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Passing,
    Failing,
    Pending,
    NoChecks,
}
```

`#[derive(...)]` asks the compiler to generate standard implementations:
`Debug` for printing in tests, `Clone`/`Copy` so the value can be duplicated
freely (it is four bytes at most), and `PartialEq`/`Eq` for comparison.

**Mapping from the API** (R2):

| GraphQL `statusCheckRollup.state` | `CiState` |
|---|---|
| `SUCCESS` | `Passing` |
| `FAILURE`, `ERROR` | `Failing` |
| `PENDING`, `EXPECTED` | `Pending` |
| field is `null` | `NoChecks` |

**Rollup rule** (FR-006, FR-007):

```rust
/// `Option<T>` is Rust's null: either `Some(value)` or `None`. There is no
/// implicit null, so a caller cannot forget the empty case.
/// Returns `None` for a repository with no open pull requests, which FR-007
/// requires to show no indicator at all.
pub fn rollup(states: &[CiState]) -> Option<CiState> { ... }
```

Precedence, in order: any `Failing` wins; else any `Pending`; else any `Passing`;
else `NoChecks`. An empty slice yields `None`.

| Input | Result | Requirement |
|---|---|---|
| `[]` | `None` | FR-007 |
| `[NoChecks]` | `Some(NoChecks)` | FR-006 |
| `[Passing, NoChecks]` | `Some(Passing)` | FR-006 |
| `[Passing, Pending]` | `Some(Pending)` | FR-006 |
| `[Pending, Failing]` | `Some(Failing)` | FR-006 |

### RepoId

```rust
/// A "newtype": a struct wrapping a single value to give it a distinct type.
/// `RepoId(42)` cannot be passed where a pull request number is expected, even
/// though both are integers. This is the cheapest bug class to eliminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoId(pub u64);
```

Sourced from GraphQL's `databaseId` (R3). This is the identity that survives
renames and transfers, and the key for both the tracked set and the config merge
(FR-021, FR-022, FR-023).

### Repository

```rust
pub struct Repository {
    pub id: RepoId,           // stable identity (FR-021)
    pub owner: String,        // display only, may change on rename
    pub name: String,         // display only, may change on rename
    pub status: RepoStatus,
}

pub enum RepoStatus {
    Pending,                                   // not yet resolved (FR-044)
    Ready { open_count: u32, pulls: Vec<PullRequest> },
    Unreadable { reason: String },             // FR-053
}
```

Modelling the three conditions as an enum rather than as an optional list plus an
optional error makes the invalid combinations unrepresentable. A repository cannot
be simultaneously loading and failed, and the renderer must handle all three, which
is what FR-042, FR-044 and FR-053 require.

`open_count` comes from GraphQL's `totalCount` and is exact even when `pulls`
holds only the first page (R11). `pulls` is completed by pagination before the
rollup is computed.

**Field notes**:

| Field | Rule |
|---|---|
| `id` | Immutable for the life of the repository. Never derived from owner/name. |
| `owner`, `name` | Refreshed from the API response on every successful fetch (FR-022). |
| `open_count` | Always all open pull requests, never filtered (FR-002). |
| `status` | `Pending` on first load; `Unreadable` never removes the row (FR-053). |

**Derived**: the repository indicator is `rollup(pulls.map(ci))`, computed over all
open pull requests and never over the filtered subset (FR-002).

### PullRequest

```rust
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub updated_at: OffsetDateTime,
    pub is_draft: bool,
    pub url: String,                       // FR-061
    pub ci: CiState,
    pub review_requests: Vec<ReviewRequest>,
}

pub enum ReviewRequest {
    User { login: String },
    Team { slug: String },
}
```

Draft pull requests are included, per the spec's Assumptions.

`review_requests` carries both shapes because FR-031 needs both. GraphQL can also
return a `requestedReviewer` of `null` (observed in live data), which is skipped
rather than treated as an error.

### OrgRepo

A repository as it appears in the settings screen, before it is tracked. Distinct
from `Repository`, which carries pull requests and CI state that settings never
fetches.

```rust
pub struct OrgRepo {
    pub id: RepoId,          // same identity as Repository (FR-021)
    pub owner: String,
    pub name: String,
    pub is_archived: bool,   // filtered out before display (FR-027)
    pub is_fork: bool,       // carried but not filtered, per Assumptions
}
```

Sourced from GraphQL query Q4. `is_archived` is carried rather than filtered at the
source so the filtering rule lives in one place and is testable against fixture data
that deliberately includes an archived repository (case F14).

The settings screen renders an `OrgRepo` as tracked when its `id` is present in the
tracked set, which is why identity must be the same `RepoId` used everywhere else.

### Viewer

```rust
pub struct Viewer {
    pub login: String,
    pub organizations: Vec<String>,
    pub teams: TeamMemberships,
}

pub enum TeamMemberships {
    Known(Vec<String>),   // team slugs, "org/team" form
    Unavailable { reason: String },   // FR-032
}
```

`TeamMemberships` is an enum rather than a plain vector so that "no teams" and
"could not read teams" cannot be confused. FR-032 requires the second case to be
stated on screen, which an empty vector could not express.

### Ordering rules (FR-014)

- Repositories: by `owner`, then `name`, case-insensitive.
- Pull requests: by `updated_at` descending, most recent first.

## Application state

### AppState

```rust
pub struct AppState {
    pub screen: Screen,
    pub repos: Vec<Repository>,
    pub viewer: Option<Viewer>,
    pub filter: FilterMode,
    pub selection: Selection,
    pub refresh: RefreshState,
    pub source_kind: SourceKind,   // Live or Fixture (FR-058)
    pub log_path: PathBuf,         // FR-064
}

pub enum Screen {
    Loading,                                    // FR-042
    LoadFailed { reason: String },              // FR-043
    Dashboard,
    Settings(SettingsScreen),
}

pub enum SettingsScreen {
    Organizations,
    Repositories { org: String, state: OrgRepoState },
}

pub enum OrgRepoState {
    Loading,                        // FR-018
    Failed { reason: String },      // FR-018, FR-028
    Ready { repos: Vec<OrgRepo> },
}

pub enum FilterMode { All, Mine, ReviewRequested }   // FR-030
```

`Screen` being an enum is what makes FR-042 and FR-043 testable: the loading state
and the first-fetch-failure state are distinct values, not an empty repository list
that happens to render differently.

### Selection

```rust
pub enum Pane { Repositories, PullRequests }

pub struct Selection {
    pub focus: Pane,                  // which pane has focus (FR-008)
    pub repo: Option<RepoId>,         // identity, not index
    pub pull: Option<u32>,            // pull request number, not index
}
```

`Pane` has exactly two variants because FR-001 fixes the dashboard at two panes. The
settings screen tracks its own position through `SettingsScreen` rather than through
`Pane`, since its two levels are a drilldown rather than side-by-side panes.

Selection is stored as **identity, not index**. This is what implements FR-045:
after a refresh reorders or removes items, the selected repository and pull request
are re-located by id and number. Storing indices would silently move the cursor to
a different item whenever the sort order changed, and pull requests are sorted by
last update (FR-014), so the order changes on almost every refresh.

**Reconciliation after a refresh** (FR-045):

1. If the selected id still exists, keep it.
2. If it does not, select the nearest surviving item using the *previous* ordering,
   preferring the item that followed it.
3. If the list is now empty, the selection becomes `None`.

### RefreshState

```rust
pub struct RefreshState {
    pub in_flight: bool,                      // FR-049, FR-052
    pub last_success: Option<OffsetDateTime>, // FR-049
    pub last_error: Option<String>,           // FR-050
    pub rate_limited_until: Option<OffsetDateTime>, // FR-051
}
```

`in_flight` is what FR-052 checks to refuse a second concurrent refresh.
`rate_limited_until` suspends the interval timer without stopping the event loop.

## Persisted model

The configuration file. Full schema in [contracts/config.md](./contracts/config.md).

`TrackedRepo` lives in `domain`, not in `config`, even though the configuration file
is its only persistence. `DataSource::dashboard` takes `&[TrackedRepo]`, and putting
it in `config` would make `source` depend on `config`, breaking the dependency
direction stated in plan.md. `Config` in `config` owns a `Vec<TrackedRepo>`; the type
itself belongs to the domain.

```rust
// domain/repository.rs
pub struct TrackedRepo {
    pub id: RepoId,     // the merge key (FR-023), same newtype used everywhere else
    pub owner: String,  // refreshed on rename (FR-022)
    pub name: String,
}

// config.rs
pub struct Config {
    pub refresh_interval_secs: u64,      // default 300 (FR-046)
    pub tracked: Vec<TrackedRepo>,
}
```

`id` is a `RepoId`, not a bare `u64`. Using the raw integer here would defeat the
newtype for the one type that crosses the config-to-source boundary, and would force
a conversion at every `dashboard()` call and every merge comparison. `RepoId` derives
`Serialize`/`Deserialize` as a transparent newtype, so the TOML on disk still holds a
plain integer (see contracts/config.md).

**Merge rule** (FR-023): the union of the on-disk set and the in-memory set, keyed
on `id`. An entry present in memory but absent on disk was added by this instance
and is kept. An entry present on disk but absent in memory was added by another
instance and is kept. An entry the user explicitly untracked in this instance is
removed, which requires tracking untracked ids for the session rather than
inferring removal from absence.

**Validation**:

| Rule | Behaviour on violation |
|---|---|
| File must parse as TOML | FR-025: report path and problem, start empty, do not overwrite |
| `id` must be unique within `tracked` | Later duplicate discarded, warning logged |
| `refresh_interval_secs` must be at least 30 | Clamped to 30, warning logged |

## State transitions

**Repository status**, per refresh cycle:

```
        ┌──────────────────────────── refresh fails for this repo ──────┐
        │                                                               v
    Pending ──── fetch resolves ────> Ready ──── refresh fails ────> Unreadable
        │                              ^                                │
        └──── fetch fails ─────────────┴──── later refresh succeeds ────┘
```

A repository never leaves the list once tracked (FR-053). `Unreadable` is
recoverable: a later successful refresh returns it to `Ready`.

**Screen**, per run:

```
    Loading ──── first fetch succeeds ────> Dashboard <────> Settings
       │                                        ^
       └──── first fetch fails ───> LoadFailed ─┘  (retry key)
```

FR-042 requires `Loading` to accept the quit key, and FR-043 requires `LoadFailed`
to name the retry key. Neither may render as an empty dashboard.

## Requirement coverage

| Requirement | Where modelled |
|---|---|
| FR-002 count and indicator unfiltered | `Repository.open_count`, rollup over all `pulls` |
| FR-004, FR-006 four states and precedence | `CiState`, `rollup` |
| FR-007 empty repository shows nothing | `rollup` returns `Option`, `None` for empty |
| FR-014 ordering | Ordering rules |
| FR-017, FR-027 settings repository list, archived excluded | `OrgRepo` |
| FR-018, FR-028 settings loading and failure | `OrgRepoState` |
| FR-021, FR-022 identity across rename | `RepoId` from `databaseId`, owner/name refreshed |
| FR-023 merge on write | `TrackedRepo.id` as merge key |
| FR-025 unparseable config | Validation table |
| FR-030 three filter modes | `FilterMode` |
| FR-031, FR-032 team requests and fallback | `ReviewRequest::Team`, `TeamMemberships` |
| FR-042, FR-043 loading and first-fetch failure | `Screen::Loading`, `Screen::LoadFailed` |
| FR-044 batched refresh, uncovered repository | `RepoStatus::Pending` |
| FR-045 selection survives refresh | `Selection` by identity, reconciliation rule |
| FR-049..FR-052 freshness and concurrency | `RefreshState` |
| FR-053 unreadable repository stays | `RepoStatus::Unreadable` |
| FR-058 active source on screen | `AppState.source_kind` |
| FR-064 log path discoverable | `AppState.log_path` |
