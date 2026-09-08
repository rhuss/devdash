# Contract: The DataSource abstraction

**Feature**: [../spec.md](../spec.md) | **Data model**: [../data-model.md](../data-model.md)

FR-054 requires all repository, pull request and check data to reach the interface
through one abstraction rather than through calls made from view code. This is that
abstraction.

## The trait

```rust
/// A `trait` is Rust's interface: a set of methods a type promises to provide.
/// It is like an interface in Java or a protocol in Swift, except that a type can
/// implement one without the trait's author knowing about the type.
///
/// `async_trait` is needed because Rust's built-in async functions in traits do not
/// yet support the dynamic dispatch this design uses (holding the source behind a
/// `Box<dyn DataSource>` so the live and fixture implementations are interchangeable
/// at runtime, which is what FR-056 requires).
#[async_trait]
pub trait DataSource: Send + Sync {
    /// The authenticated identity, organizations, and team memberships (FR-040).
    async fn viewer(&self) -> Result<Viewer, SourceError>;

    /// Repositories, their open pull request counts, and CI state, for the
    /// tracked set. One call per refresh (R1).
    async fn dashboard(&self, tracked: &[TrackedRepo])
        -> Result<Vec<RepositoryData>, SourceError>;

    /// Repositories within one organization, for the settings screen (FR-017).
    async fn org_repositories(&self, org: &str)
        -> Result<Vec<OrgRepo>, SourceError>;

    /// Which source is active, so the interface can say so (FR-058).
    fn kind(&self) -> SourceKind;
}

pub enum SourceKind { Live, Fixture }
```

`Send + Sync` are marker traits meaning the type is safe to move between threads
and to share across them. The async runtime requires both, since the source is
called from spawned tasks.

`Result<T, E>` is Rust's error type: either `Ok(value)` or `Err(error)`. There are
no exceptions, so every caller must deal with the failure case, which is what makes
FR-050 and FR-053 hard to forget.

## Error taxonomy

```rust
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("no credential available: {tried}")]
    NoCredential { tried: String },                    // FR-039

    #[error("rate limit exhausted, resets at {resets_at}")]
    RateLimited { resets_at: OffsetDateTime },         // FR-051

    #[error("not authorized for {scope}")]
    Unauthorized { scope: String },                    // FR-028, FR-029, FR-032

    #[error("repository unreadable: {reason}")]
    Repository { id: RepoId, reason: String },         // FR-053

    #[error("network error: {0}")]
    Network(String),                                   // FR-050

    #[error("unexpected response shape: {0}")]
    Malformed(String),                                 // R11 risk mitigation

    #[error("fixture problem at {path}: {reason}")]
    Fixture { path: PathBuf, reason: String },         // FR-060
}
```

The variants exist because different requirements demand different handling.
`RateLimited` must suspend the interval timer (FR-051), `Repository` must degrade
one row while the others refresh (FR-053), and `Unauthorized` must leave other
organizations usable (FR-028). Collapsing them into one error string would make
those behaviours impossible to implement correctly.

## Partial results

`dashboard` returns per-repository outcomes, not an all-or-nothing list, because
FR-053 requires one unreadable repository to leave the others working.

```rust
pub struct RepositoryData {
    pub id: RepoId,
    pub owner: String,     // current values, may differ from stored (FR-022)
    pub name: String,
    pub result: Result<RepoPayload, SourceError>,
}

pub struct RepoPayload {
    pub open_count: u32,             // totalCount, exact even if paginated (R11)
    pub pulls: Vec<PullRequest>,
}
```

The outer `Result` on the method covers whole-request failures such as no
credential or a dead network. The inner `Result` per repository covers one
repository being deleted, renamed beyond recovery, or access-revoked.

## Behavioural contract

Both implementations must satisfy these, and the same test suite runs against both.

| # | Guarantee | Requirement |
|---|---|---|
| C1 | `open_count` is the true total of open pull requests, even when `pulls` is a partial page | FR-002, R11 |
| C2 | `pulls` is complete before the caller computes a rollup | FR-006, R11 |
| C3 | Draft pull requests are included | Assumptions |
| C4 | A repository with no open pull requests returns `open_count: 0` and an empty `pulls` | FR-007 |
| C5 | `review_requests` carries both user and team entries | FR-031 |
| C6 | `viewer().teams` distinguishes "no teams" from "could not read teams" | FR-032 |
| C7 | Archived repositories are absent from `org_repositories` | FR-027 |
| C8 | No method ever returns a credential in a value or an error message | FR-038, SC-008 |
| C9 | `kind()` is constant for the life of the source | FR-058 |

## Implementations

### GithubDataSource

Backed by the GraphQL endpoint. See [github-graphql.md](./github-graphql.md).

- `dashboard` issues one aliased query for all tracked repositories (R1), then
  follow-up paginated queries only for repositories whose `totalCount` exceeds the
  page already fetched (R11, satisfying C2).
- Owner and name in the response are authoritative and may differ from the stored
  values after a rename (FR-022).

### FixtureDataSource

Backed by a JSON file. See [fixture.md](./fixture.md).

- Makes **no network request of any kind** (FR-057). This is asserted in tests, not
  merely intended.
- Returns `SourceError::Fixture` for a missing or malformed snapshot (FR-060).
- Ignores the `tracked` argument's owner and name and matches on `id`, so a fixture
  run behaves identically whether or not the config file exists.

## Why this shape

The trait exists to make the interface testable, which is the stated reason in the
brainstorm and the basis of SC-007. Two properties deliver that:

1. **Dynamic dispatch.** The application holds `Box<dyn DataSource>`, so the source
   is chosen at startup from a command-line flag with no recompilation (FR-056).
2. **Domain types at the boundary.** The trait returns `PullRequest` and `CiState`,
   not GraphQL JSON. The interface never sees an API shape, so a change to the query
   cannot reach the renderer, and a render test never needs to construct a
   GraphQL response.
