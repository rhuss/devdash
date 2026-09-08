# Research: devdash TUI

**Date**: 2026-09-08
**Feature**: [spec.md](./spec.md)

All findings below were verified against the live GitHub API using the authenticated
`gh` CLI on 2026-09-08, not taken from memory. Crate versions were read from
crates.io via `cargo search` on the same date.

## R1: Fetch strategy, GraphQL over REST

**Decision**: Use the GitHub GraphQL API with one aliased query covering every
tracked repository per refresh. Do not use the REST API.

**Rationale**: The specification's eager check fetching assumption (Assumptions section, SC-004)
worried that 20 repositories implies "well over a hundred requests". That is true of
REST, where listing pull requests costs one request per repository and the check
rollup costs one more per pull request. It is not true of GraphQL.

Measured directly:

```
3 repositories, 60 open pull requests, full CI rollup for each
-> 1 HTTP request, rateLimit cost = 1 point (of 5000/hour)
```

The same query shape scales to 20 repositories in a single round trip. This makes
SC-004 (drawn within 1 second, fully resolved within 15 seconds) comfortable rather
than tight, and makes the rate-limit budget a non-issue at the specified scale.

**Alternatives considered**:

- *REST via octocrab*: typed and convenient, but 20 repositories with 5 open pull
  requests each costs roughly 120 requests against a 5000/hour budget, and the
  latency is dominated by round trips even when issued concurrently.
- *REST with concurrency*: reduces wall-clock time but not request count, and makes
  the rate-limit story materially worse for a five-minute refresh interval.

**Consequence for the spec**: the Eager check fetching assumption is more pessimistic
than reality. It is not wrong as a requirement, and its conclusion (a refresh
interval measured in minutes) still holds, but the concurrency it calls for is
largely unnecessary. FR-044 (progressive population) remains worth implementing for
the settings screen and for the multi-page case in R11.

## R2: CI state maps natively to statusCheckRollup

**Decision**: Derive the four CI states from `commits(last:1).nodes[].commit.statusCheckRollup.state`.

**Rationale**: FR-004 requires exactly four distinguishable states. GraphQL's
`statusCheckRollup` provides them without any client-side aggregation over
individual check runs:

| `statusCheckRollup` | devdash CI state |
|---|---|
| `SUCCESS` | passing |
| `FAILURE`, `ERROR` | failing |
| `PENDING`, `EXPECTED` | pending |
| `null` (field absent) | no checks configured |

The `null` case is the important one: it is returned when the head commit has no
checks or statuses at all, which is precisely FR-004's fourth state. Verified in
live data, where `ratatui/ratatui` returned a mix of `SUCCESS`, `FAILURE`, and
`null` across its open pull requests.

**Alternatives considered**: Fetching `checkSuites`/`checkRuns` and rolling up
manually. Rejected: more nodes, more code, and it would have to re-implement
GitHub's own precedence rules, which is exactly the kind of subtle mismatch that
makes a dashboard untrustworthy.

## R3: Stable repository identity

**Decision**: Use GraphQL's `repository.databaseId` as the tracked-set key.

**Rationale**: FR-021 and FR-022 require the tracked set to survive renames and
transfers. `databaseId` is the numeric id that GitHub keeps stable across both.
Verified: `ratatui/ratatui` returns `databaseId: 600886023`.

The refresh query addresses repositories by owner and name, so a renamed repository
must be re-resolved. GitHub's API redirects the old name to the new one, and the
response carries the current `nameWithOwner`. Matching the response's `databaseId`
back to the stored entry lets devdash detect the rename and rewrite the stored
owner and name, satisfying FR-022.

**Alternatives considered**: Storing only owner and name and treating a failed
lookup as unreadable. Rejected during clarification because it silently loses
tracked repositories.

## R4: Team-based review requests

**Decision**: Fetch the viewer's team memberships once per run via
`organization(login:).teams(first:, userLogins:[viewer])`, aliased across the
viewer's organizations into the startup query. Compute the "awaiting my review"
filter client-side by matching each pull request's `reviewRequests` nodes against
the viewer's login and team slugs.

**Rationale**: FR-031 requires the filter to match both direct and team-directed
requests. The per-repository refresh query already carries `reviewRequests` inline
at no extra request cost, so the only additional data needed is the viewer's team
slugs. Verified live: the query returns real memberships (3 teams in one
organization, 5 in another, 1 in a third), cost 1 point per organization and
aliasable into a single request.

This approach is deterministic and needs no search API, and it degrades exactly as
FR-032 specifies: if the team query fails for lack of scope, the filter falls back
to direct requests and says so.

**Alternatives considered**: `search(query: "is:pr is:open review-requested:@me")`.
It runs and returns results, but its exact semantics regarding team-directed
requests are not documented in a way worth depending on, and it would be a second
data path returning pull requests that the per-repository query already provides.
Rejected in favour of the deterministic client-side match.

**Note on scope**: reading team memberships requires `read:org`. The token in use
during research carried `admin:org`, which includes it. FR-032's fallback exists
precisely for tokens that do not.

## R5: HTTP client, reqwest over octocrab

**Decision**: Use `reqwest` with `serde` directly. Do not depend on `octocrab`.

**Rationale**: octocrab's principal value is its typed REST surface, which R1
rules out. Its GraphQL support is a thin passthrough that takes a query string and
returns JSON, which is what `reqwest` already does. Since the aliased multi-repo
query in R1 must be constructed dynamically anyway, the typed layer buys nothing
and costs a large dependency tree.

Authentication, rate-limit header parsing, and error mapping amount to a small
amount of code against a single endpoint.

**Alternatives considered**: octocrab for the settings screen's organization and
repository listing while using raw GraphQL for the refresh. Rejected: two API
styles, two auth paths, and two error taxonomies for no benefit, since the
listing queries are equally easy in GraphQL.

## R6: Credential acquisition

**Decision**: Run `gh auth token` as a subprocess and read its stdout. If `gh` is
absent, or exits non-zero, fall back to the `GITHUB_TOKEN` environment variable.
Never persist the value.

**Rationale**: FR-037 names both sources in that order. `gh auth token` is the
supported way to ask the CLI for its token and works regardless of whether the CLI
stores it in its own config or is itself reading `GITHUB_TOKEN`. Verified: `gh`
2.100.0 is present and authenticated in this environment.

FR-039 requires reporting which mechanisms were tried when neither works, so both
failures must be captured and reported together rather than collapsing to a single
"no token" message.

## R7: Configuration file

**Decision**: TOML, at the `directories` crate's config directory for the
application, written atomically, with a read-modify-merge cycle before every write.

**Rationale**:

- *Format*: TOML satisfies the spec's "human-readable text format the user may edit
  by hand" (FR-021's readability clause). It handles a list of table entries
  carrying id, owner, and name naturally.
- *Location*: `directories` resolves the per-platform convention, giving
  `~/.config/devdash/` on Linux and the macOS equivalent, without hand-rolling XDG
  logic.
- *Atomic write* (FR-024): write to a temporary file in the same directory, then
  rename over the target. Rename within a filesystem is atomic, so a crash mid-write
  leaves either the old file or the new one, never a truncated one. `tempfile`'s
  persist operation does exactly this.
- *Merge before write* (FR-023): record the file's modification time and a hash of
  its contents at load. Before writing, re-stat. If it changed, re-read, merge the
  tracked sets by repository id, then write. Keying the merge on the stable id from
  R3 makes the merge unambiguous.

**Alternatives considered**: An advisory lock file held for the process lifetime.
Rejected during clarification: a stale lock after a crash blocks the user, and the
merge is cheap.

## R8: Logging

**Decision**: `tracing` with `tracing-subscriber`, writing to a rolling file via
`tracing-appender`, located in the `directories` crate's state or data directory.

**Rationale**: FR-063 requires a bounded log file holding the detail behind every
on-screen error, and FR-064 requires its path to be discoverable. A TUI cannot log
to stdout or stderr while it owns the alternate screen, so a file is the only
destination that works during a run.

`tracing-appender`'s rolling file writer bounds growth by rotation, satisfying
FR-063's size constraint. Its non-blocking writer keeps logging off the render path,
which matters for SC-005's 100 millisecond input budget.

FR-038 and SC-008 forbid the credential appearing in log output, so the token must
never be placed in a tracing field or included in a logged request description.

## R9: Testing a terminal interface

**Decision**: `ratatui`'s `TestBackend` for rendering assertions, `insta` for
snapshot tests of rendered frames, plain `cargo test` for everything else. The
fixture data source is the input for all interface tests.

**Rationale**: `TestBackend` renders into an in-memory buffer that can be asserted
against directly, with no terminal and no network. Combined with the fixture data
source required by FR-054 through FR-060, this delivers SC-007 (rendering, rollup,
filtering and navigation all verifiable without network or credentials) and SC-006.

`insta` snapshots are well suited to a full rendered frame, where the assertion is
"this is what the screen looks like" and the diff on failure is the useful output.
Snapshot tests of the buffer's text content also validate FR-005 directly: if the
four CI states are distinguishable in a text-only buffer, colour is not
load-bearing, which is SC-013.

## R10: Concurrency architecture

**Decision**: A `tokio` runtime for I/O, a pure synchronous core for state and
rendering, and channels between them. Terminal input is read on a dedicated blocking
task and forwarded as events.

**Rationale**: SC-005 requires every navigation action to respond within 100
milliseconds and never wait on the network, and FR-048 requires the interface to
stay responsive during a refresh. That is only achievable if the render path never
awaits I/O.

The shape is a single event loop consuming one merged stream of events: key presses,
a periodic tick for the refresh interval, and completed data-source results. State
transitions are pure functions from state and event to new state, which makes them
directly unit-testable and keeps FR-045's selection-preservation logic out of the
I/O layer where it would be hard to test.

## R11: Repositories with many open pull requests

**Decision**: Request the 100 most recently updated open pull requests per
repository (GraphQL's per-connection maximum). Take the open pull request count from
`totalCount`, which is exact regardless of how many nodes are fetched. When
`totalCount` exceeds the number fetched, issue follow-up paginated requests for the
remainder before computing the repository rollup.

**Rationale**: This is a correctness issue that the specification does not address.
FR-002 requires the count of open pull requests, and FR-006 derives the repository
indicator from *all* of them. `totalCount` covers the count exactly. The indicator
does not: a repository with 300 open pull requests whose only failure sits at
position 250 would show as passing if only the first 100 were fetched.

Verified as a real case, not a hypothetical: `rust-lang/cargo` had 90 open pull
requests and `ratatui/ratatui` had 76 during research, so the 100 boundary is
within reach of ordinary repositories.

Paginating only the repositories that need it keeps the common case at one request
while keeping the rollup honest.

**Alternatives considered**: Capping at 100 and documenting the rollup as
approximate. Rejected: FR-006 is written as an exact rule, and a health indicator
that is quietly wrong on the largest repositories undermines the feature's core
claim (SC-002).

## Resolved unknowns

| Unknown from Technical Context | Resolution |
|---|---|
| API style and request budget | R1: GraphQL, one aliased query, 1 point per refresh |
| Four-state CI derivation | R2: `statusCheckRollup`, `null` is the no-checks state |
| Stable repository identity | R3: `databaseId` |
| Team-directed review requests | R4: viewer team slugs plus inline `reviewRequests` |
| HTTP client choice | R5: `reqwest` and `serde`, no octocrab |
| Credential source | R6: `gh auth token`, then `GITHUB_TOKEN` |
| Config format, location, atomicity, merge | R7: TOML, `directories`, temp-file rename, id-keyed merge |
| Log destination and bounding | R8: `tracing-appender` rolling file in the state directory |
| Interface testing without a terminal | R9: `TestBackend` plus `insta`, driven by fixtures |
| Keeping input responsive during refresh | R10: tokio for I/O, pure synchronous core, channels |
| Repositories exceeding one page of pull requests | R11: `totalCount` for the count, paginate for the rollup |
