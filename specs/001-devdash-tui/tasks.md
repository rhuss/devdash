---

description: "Task list for devdash TUI implementation"
---

# Tasks: devdash TUI

**Input**: Design documents from `/specs/001-devdash-tui/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: Test tasks are included because the specification requires them. SC-007
states that rendering, CI rollup, filtering and navigation must all be verifiable in
automated tests needing neither network access nor credentials, and FR-054 through
FR-060 exist to make that possible. Tests here are a requirement, not an addition.

**Organization**: Tasks are grouped by user story so each can be implemented and
demonstrated independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task serves (US1..US6)
- Every task names the file it touches

## Path Conventions

Single Rust binary crate at the repository root, per plan.md's Structure Decision:
`src/` for sources, `tests/` for integration tests, `fixtures/` for the committed
snapshot.

## Global Constraints

Every task inherits plan.md's [Global Constraints](./plan.md#global-constraints)
section, G1 through G9. The four easiest to break without noticing: no credential
in any output (G1), no CI state carried by colour alone (G2), no network in fixture
mode (G3), no data reaching the interface except through `DataSource` (G6).

## Shared Interfaces

Names and signatures that tasks depend on across file boundaries. An implementer
working on one task needs these without reading the whole design. Canonical
definitions live in [data-model.md](./data-model.md) and
[contracts/data-source.md](./contracts/data-source.md).

```rust
// domain (T005-T008), consumed by every later phase
pub enum CiState { Passing, Failing, Pending, NoChecks }
pub struct RepoId(pub u64);
pub struct Repository  { id: RepoId, owner: String, name: String, status: RepoStatus }
pub enum   RepoStatus  { Pending, Ready { open_count: u32, pulls: Vec<PullRequest> },
                         Unreadable { reason: String } }
pub struct PullRequest { number: u32, title: String, author: String,
                         updated_at: OffsetDateTime, is_draft: bool, url: String,
                         ci: CiState, review_requests: Vec<ReviewRequest> }
pub enum   ReviewRequest { User { login: String }, Team { slug: String } }
pub struct OrgRepo     { id: RepoId, owner: String, name: String,
                         is_archived: bool, is_fork: bool }
pub struct Viewer      { login: String, organizations: Vec<String>, teams: TeamMemberships }
pub enum   TeamMemberships { Known(Vec<String>), Unavailable { reason: String } }

// T009, consumed by T028 and tested by T010.
// Returns None for a repository with no open pull requests (FR-007).
pub fn rollup(states: &[CiState]) -> Option<CiState>;

// source (T012-T013), implemented by T015 (fixture) and T039 (live)
#[async_trait]
pub trait DataSource: Send + Sync {
    async fn viewer(&self) -> Result<Viewer, SourceError>;
    async fn dashboard(&self, tracked: &[TrackedRepo])
        -> Result<Vec<RepositoryData>, SourceError>;
    async fn org_repositories(&self, org: &str) -> Result<Vec<OrgRepo>, SourceError>;
    fn kind(&self) -> SourceKind;
}
pub enum SourceKind { Live, Fixture }
pub struct RepositoryData { id: RepoId, owner: String, name: String,
                            result: Result<RepoPayload, SourceError> }
pub struct RepoPayload    { open_count: u32, pulls: Vec<PullRequest> }
pub enum SourceError { NoCredential { tried: String },
                       RateLimited { resets_at: OffsetDateTime },
                       Unauthorized { scope: String },
                       Repository { id: RepoId, reason: String },
                       Network(String), Malformed(String),
                       Fixture { path: PathBuf, reason: String } }

// TrackedRepo lives in domain (T006), not config, so that `source` does not
// depend on `config`. Config owns a Vec of them.
pub struct TrackedRepo { id: RepoId, owner: String, name: String }
// config (T036-T037), consumed by T046, T054, T055
pub struct Config      { refresh_interval_secs: u64, tracked: Vec<TrackedRepo> }

// app state (T017-T019), consumed by every ui and update task
pub enum Screen     { Loading, LoadFailed { reason: String }, Dashboard, Settings(SettingsScreen) }
pub enum FilterMode { All, Mine, ReviewRequested }
pub enum Pane       { Repositories, PullRequests }
pub struct Selection { focus: Pane, repo: Option<RepoId>, pull: Option<u32> }
pub struct RefreshState { in_flight: bool, last_success: Option<OffsetDateTime>,
                          last_error: Option<String>,
                          rate_limited_until: Option<OffsetDateTime> }
```

Note that `Selection` holds `RepoId` and a pull request number, never indices. That
is what makes FR-045 work, and a task that stores an index instead will pass its own
test and break T071.

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Initialize the cargo binary crate and declare the dependency set from plan.md in `Cargo.toml` (ratatui 0.30, crossterm 0.29, tokio 1.53, reqwest 0.13, serde 1, serde_json 1, toml 1.1, directories 6, time 0.3 with the `serde` and `parsing` features, tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2, thiserror 2, anyhow 1, tempfile 3, clap 4, async-trait 0.1; dev: insta 1.48)
- [X] T002 [P] Create the module skeleton with empty declarations in `src/main.rs`, `src/domain/mod.rs`, `src/source/mod.rs`, `src/app/mod.rs`, `src/ui/mod.rs`
- [X] T003 [P] Add formatting and lint configuration in `rustfmt.toml` and the `[lints]` section of `Cargo.toml`
- [X] T004 [P] Create the `fixtures/` directory and confirm `/target` is ignored in `.gitignore`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The domain types, the data-source abstraction, and the terminal
lifecycle. Every user story depends on these, so nothing else can start until they
are done.

**Note**: the live GitHub client is deliberately *not* here. US1 is built and
demonstrated against fixtures first, so live data is Phase 4. This follows plan.md's
sequencing and keeps credentials off the critical path.

### Domain types

- [X] T005 [P] Define the `CiState` enum and the `statusCheckRollup` mapping table from research.md R2 in `src/domain/ci.rs`
- [X] T006 [P] Define the `RepoId` newtype with transparent serde, plus `Repository`, `RepoStatus`, `OrgRepo` and `TrackedRepo` (whose `id` is a `RepoId`, not a bare integer), in `src/domain/repository.rs`
- [X] T007 [P] Define `PullRequest` and the `ReviewRequest` enum in `src/domain/pull_request.rs`
- [X] T008 [P] Define `Viewer` and the `TeamMemberships` enum in `src/domain/viewer.rs`
- [X] T009 Implement `rollup()` returning `Option<CiState>` with the FR-006 precedence and the FR-007 empty case in `src/domain/ci.rs`
- [X] T010 Write the rollup precedence tests covering all five rows of the data-model.md table in `tests/ci_rollup.rs`
- [X] T011 [P] Implement the FR-014 ordering rules, repositories by owner then name and pull requests by `updated_at` descending, in `src/domain/mod.rs`

### Data source abstraction

- [X] T012 Define the `DataSource` trait, `SourceKind`, `RepositoryData` and `RepoPayload` per contracts/data-source.md in `src/source/mod.rs`
- [X] T013 Define the seven-variant `SourceError` enum with `thiserror` in `src/source/mod.rs`
- [X] T014 Author the fixture snapshot covering all seventeen required cases F1 through F17 from contracts/fixture.md, satisfying FR-059, in `fixtures/snapshot.json`
- [X] T015 Implement `FixtureDataSource`, constructed with no HTTP client so a network call is a compile error, in `src/source/fixture.rs`
- [X] T016 Write the fixture coverage test asserting every case F1 through F17 is present in `tests/fixture_source.rs`

### Application shell

- [X] T017 Define `AppState`, `Screen`, `SettingsScreen`, `OrgRepoState`, `FilterMode` and `Pane` in `src/app/state.rs`
- [X] T018 Implement `Selection` keyed on identity rather than index, with the three-step reconciliation rule from data-model.md, in `src/app/state.rs`
- [X] T019 [P] Define the `Event` enum covering key, tick, data and error events in `src/app/event.rs`
- [X] T020 Implement terminal setup and teardown with a panic hook that restores the terminal before the default hook runs, and a SIGINT/SIGTERM handler that restores it before exiting, so abrupt termination never leaves the terminal in raw mode, satisfying FR-012, in `src/main.rs`
- [X] T021 Implement the event loop: tokio runtime, a blocking input reader task, and a merged event channel, keeping input responsive while I/O is in flight per FR-048, in `src/app/mod.rs`
- [X] T022 Implement render dispatch by `Screen` in `src/ui/mod.rs`

**Checkpoint**: The binary starts, shows an empty frame, and exits cleanly with the terminal intact.

---

## Phase 3: User Story 1 - Survey open work across tracked repositories (Priority: P1) 🎯 MVP

**Goal**: Two panes showing tracked repositories with open pull request counts and
CI state, and the selected repository's pull requests.

**Independent test**: Run `cargo run -- --fixtures` and confirm both panes populate,
selection moves, and CI state is distinguishable per pull request and per
repository. Needs no network and no credential.

### Tests for User Story 1

- [X] T023 [P] [US1] Write the two-pane render test, covering FR-001, using `ratatui::backend::TestBackend` in `tests/render_dashboard.rs`
- [X] T024 [P] [US1] Write the test that a repository with zero open pull requests shows a count of zero, no CI indicator, and an explicit empty message, in `tests/render_dashboard.rs`
- [X] T025 [P] [US1] Write the test that a repository whose pull requests include a failure shows failing without being selected, in `tests/render_dashboard.rs`
- [X] T026 [P] [US1] Write the colour-independence test asserting all four CI states are distinguishable in the text buffer alone, satisfying SC-013, in `tests/render_dashboard.rs`

### Implementation for User Story 1

- [X] T027 [US1] Implement the CI indicator distinguishing the four states of FR-004, with a distinct symbol per state and colour as reinforcement only, per FR-005, in `src/ui/indicator.rs`
- [X] T028 [US1] Implement the repository pane showing owner/name, unfiltered open count and rolled-up indicator, per FR-002, in `src/ui/dashboard.rs`
- [X] T029 [US1] Implement the pull request pane showing number, title, author and CI state, per FR-003, in `src/ui/dashboard.rs`
- [X] T030 [US1] Implement selection movement within a pane and focus movement between panes, and update the pull request pane when the selected repository changes, per FR-008 and FR-009, in `src/app/update.rs`
- [X] T031 [US1] Implement scrolling that keeps the current selection visible in both panes, per FR-010, in `src/ui/dashboard.rs`
- [X] T032 [US1] Implement resize handling and the readable too-small message, per FR-011, in `src/ui/mod.rs`
- [X] T033 [US1] Implement the key hint bar making the current context's bindings discoverable, per FR-013, in `src/ui/status_bar.rs`
- [X] T034 [US1] Wire the quit key to a clean exit with terminal restore, per FR-012, in `src/app/update.rs`

**Checkpoint**: `cargo run -- --fixtures` shows a working dashboard. This is the MVP.

---

## Phase 4: Live Data Infrastructure (Blocking for real use)

**Purpose**: Credentials, configuration, and the GraphQL client. Not a user story,
but every story past US1 needs real data to be useful. Sequenced here rather than in
Phase 2 so that US1 lands first on fixtures.

- [X] T035 [P] Implement credential acquisition, `gh auth token` first then `GITHUB_TOKEN` per FR-037, reporting both failures together per FR-039, in `src/auth.rs`
- [X] T036 [P] Implement `Config` holding `Vec<TrackedRepo>` from the domain, with loading, parsing and validation per contracts/config.md, including the FR-025 rule that an unparseable file is reported and never overwritten, in `src/config.rs`
- [X] T037 Implement the atomic write and read-modify-merge protocol from contracts/config.md, keyed on repository id, per FR-023 and FR-024, in `src/config.rs`
- [X] T038 [P] Write the configuration tests for the merge table, atomic replacement, and the unparseable file case, in `tests/config_merge.rs`
- [X] T039 Implement the live `GithubDataSource` and its GraphQL client with the authorization header, never logging it, completing the pair of implementations FR-055 requires, in `src/source/github/mod.rs`
- [X] T040 Implement the aliased multi-repository dashboard query builder from contracts/github-graphql.md Q2 in `src/source/github/query.rs`
- [X] T041 Implement response deserialization into domain types, handling all six observed quirks from contracts/github-graphql.md, in `src/source/github/response.rs`
- [X] T042 Implement partial-failure mapping so a top-level `errors` entry becomes that repository's inner `Err` while the others survive, per FR-053, in `src/source/github/response.rs`
- [X] T043 Implement pull request pagination for repositories whose `totalCount` exceeds the fetched page, so the FR-006 rollup stays correct (research.md R11), in `src/source/github/mod.rs`
- [X] T044 Implement the viewer and team-membership queries Q1 from contracts/github-graphql.md, establishing identity and teams per FR-040 and degrading to `TeamMemberships::Unavailable` per FR-032, in `src/source/github/query.rs`
- [X] T045 Implement the `Loading` and `LoadFailed` screens, satisfying FR-042 and FR-043, in `src/ui/mod.rs`
- [X] T046 Implement the command-line surface from contracts/cli.md, wire data-source selection at startup, and trigger the initial fetch per FR-041, in `src/cli.rs` and `src/main.rs`

**Checkpoint**: The dashboard renders live data for a hand-written config file.

---

## Phase 5: User Story 2 - Choose which repositories to track (Priority: P2)

**Goal**: A settings screen listing organizations, drilling into repositories, and
toggling the tracked set, persisted across restarts.

**Independent test**: Launch with an empty tracked set, open settings, drill into an
organization, toggle two repositories, return to the dashboard and see them, then
restart and see them still there.

### Tests for User Story 2

- [X] T047 [P] [US2] Write the settings navigation and toggle tests in `tests/render_settings.rs`
- [X] T048 [P] [US2] Write the tests that archived repositories are excluded and that loading and failure states render distinctly from an empty organization, in `tests/render_settings.rs`

### Implementation for User Story 2

- [X] T049 [US2] Implement `org_repositories` using query Q4, paginating via `pageInfo.hasNextPage` so an organization with several hundred repositories is not silently truncated, and filtering archived repositories per FR-027, in `src/source/github/mod.rs`
- [X] T050 [US2] Implement the settings entry and return keys per FR-015, and the organization list including the personal account per FR-016, in `src/ui/settings.rs`
- [X] T051 [US2] Implement the repository list with per-row tracked state and toggling, per FR-017 and FR-019, in `src/ui/settings.rs`
- [X] T052 [US2] Implement the loading and failure states for an organization fetch, per FR-018 and FR-028, in `src/ui/settings.rs`
- [X] T053 [US2] Implement back navigation that restores the previously selected organization, per FR-020, in `src/app/update.rs`
- [X] T054 [US2] Persist the tracked set through the merge protocol whenever it changes and reflect the updated set on return to the dashboard without a restart, per FR-021, FR-023 and FR-026, in `src/app/update.rs`
- [X] T055 [US2] Implement rename following so a repository matched by id updates its stored owner and name, per FR-022, in `src/config.rs`
- [X] T056 [US2] Implement the empty tracked set state that names the settings key, per FR-021 and US2 scenario 7, in `src/ui/dashboard.rs`
- [X] T057 [US2] Implement the organization-list failure path that still offers the personal account, per FR-029, in `src/ui/settings.rs`

**Checkpoint**: Repositories are chosen in the interface and survive a restart.

---

## Phase 6: User Story 3 - Run without credentials or network (Priority: P3)

**Goal**: Fixture mode as a first-class, visible feature rather than a test harness.

**Independent test**: With no credential and no network, launch against the fixture
snapshot, see every screen render and all four CI states, and see the interface state
plainly that the data is not live.

### Tests for User Story 3

- [X] T058 [P] [US3] Write the test asserting zero network requests are made when the fixture source is active, per FR-057, in `tests/fixture_source.rs`
- [X] T059 [P] [US3] Write the tests for a missing and a malformed snapshot producing a named path, a stated reason, and exit code 2, per FR-060, in `tests/fixture_source.rs`

### Implementation for User Story 3

- [X] T060 [US3] Implement the `--fixtures [PATH]` flag selecting the source with no recompilation, per FR-056, in `src/cli.rs`
- [X] T061 [US3] Implement the on-screen active-source indicator shown whenever the source is not live, and add a render test asserting a viewer can tell live from fixture data and how old it is, verifying SC-011, per FR-058, in `src/ui/status_bar.rs`
- [X] T062 [US3] Implement fixture error handling and the exit codes from contracts/cli.md, per FR-060, in `src/main.rs`

**Checkpoint**: `cargo run -- --fixtures` works with the network disabled, and says so.

---

## Phase 7: User Story 4 - Narrow the list to what needs my attention (Priority: P4)

**Goal**: Three filter modes over the pull request pane, including team-directed
review requests.

**Independent test**: Cycle the three modes against fixture data and confirm the pane
contents change, the active mode is labelled, and the repository pane's counts do
not move.

### Tests for User Story 4

- [X] T063 [P] [US4] Write the tests for all three filter modes, including the team-directed review request case F11, in `tests/filtering.rs`
- [X] T064 [P] [US4] Write the test that repository pane counts and indicators are unchanged by the active filter, per FR-002, in `tests/filtering.rs`
- [X] T065 [P] [US4] Write the test distinguishing "no pull requests match this filter" from "no open pull requests", per FR-036, in `tests/filtering.rs`

### Implementation for User Story 4

- [X] T066 [US4] Implement the three filter modes with `All` as the startup default, per FR-030 and FR-034, in `src/app/update.rs`
- [X] T067 [US4] Implement review-request matching against the viewer's login and team slugs, per FR-031, in `src/domain/viewer.rs`
- [X] T068 [US4] Implement the degraded filter notice shown when team memberships are unavailable, per FR-032, in `src/ui/status_bar.rs`
- [X] T069 [US4] Implement the always-visible filter mode label and ensure a repository change does not reset it, per FR-033 and FR-035, in `src/ui/status_bar.rs`
- [X] T070 [US4] Implement the two distinct empty-pane messages, per FR-036, in `src/ui/dashboard.rs`

**Checkpoint**: The dashboard doubles as a personal work queue.

---

## Phase 8: User Story 5 - Trust that what I am looking at is current (Priority: P5)

**Goal**: Interval and manual refresh, visible freshness, and failure handling that
never blanks the screen.

**Independent test**: Watch the timestamp advance on a manual refresh, confirm
navigation stays responsive during it, then make the source fail and confirm the
previous data stays on screen.

### Tests for User Story 5

- [X] T071 [P] [US5] Write the tests that selection survives a reorder and falls back to the nearest survivor on removal, per FR-045, in `tests/selection.rs`
- [X] T072 [P] [US5] Write the tests that a failed, timed-out and rate-limited refresh each leave previous data on screen with no blank frame and no crash, verifying SC-009, and that an unreadable repository degrades to a marked row while the others still refresh, verifying SC-012, per FR-050 and FR-053, in `tests/render_dashboard.rs`

### Implementation for User Story 5

- [X] T073 [US5] Implement `RefreshState` tracking in-flight, last success, last error and rate-limit reset, in `src/app/state.rs`
- [X] T074 [US5] Implement the configurable interval timer, defaulting to 300 seconds, per FR-046, in `src/app/mod.rs`
- [X] T075 [US5] Implement the manual refresh key with a single-flight guard rejecting a concurrent refresh, per FR-047 and FR-052, in `src/app/update.rs`
- [X] T076 [US5] Implement progressive population so resolved repositories appear while others stay pending, per FR-044, in `src/app/update.rs`
- [X] T077 [US5] Implement the in-flight indicator and last-successful-refresh time, per FR-049, in `src/ui/status_bar.rs`
- [X] T078 [US5] Implement failed-refresh handling that retains previous data and shows an error indicator, per FR-050, in `src/app/update.rs`
- [X] T079 [US5] Implement rate-limit handling that reports the reset time and suspends automatic refresh until then, per FR-051, in `src/app/update.rs`
- [X] T080 [US5] Implement the unreadable repository row showing a marker and reason in place of count and indicator, per FR-053 and SC-012, in `src/ui/dashboard.rs`

**Checkpoint**: The dashboard is trustworthy: never stale-looking, never blank on failure.

---

## Phase 9: User Story 6 - Act on a pull request (Priority: P6)

**Goal**: Enter on a pull request opens it in the browser.

**Independent test**: Select a pull request, press Enter, and confirm the correct URL
is handed to the system browser while devdash keeps running.

### Tests for User Story 6

- [X] T081 [P] [US6] Write the test that the selected pull request's URL is the one handed to the opener, in `tests/browser_handoff.rs`

### Implementation for User Story 6

- [X] T082 [US6] Implement opening the selected pull request in the system browser without exiting, per FR-061, in `src/app/update.rs`
- [X] T083 [US6] Implement the fallback that reports the failure and displays the URL for copying, per FR-062, in `src/ui/status_bar.rs`

**Checkpoint**: The dashboard is a launchpad, not a read-only wall.

---

## Phase 10: Polish & Cross-Cutting Concerns

- [X] T084 [P] Implement the rolling log file subscriber in the platform state directory, bounded by rotation, per FR-063, in `src/logging.rs`
- [X] T085 [P] Implement log path discoverability from any error indicator and the `--print-log-path` flag, per FR-064, in `src/ui/status_bar.rs` and `src/cli.rs`
- [X] T086 [P] Write the tests asserting no credential value appears in log output, per FR-038 and SC-008, and that every error surfaced on screen has its underlying cause recorded in the log file, verifying SC-014 and FR-063, in `tests/logging.rs`
- [X] T087 [P] Run every offline quickstart scenario V1 through V4 from `specs/001-devdash-tui/quickstart.md` and record the results
- [X] T088 Run every live quickstart scenario V5 through V8 from `specs/001-devdash-tui/quickstart.md` and record the results
- [X] T089 [P] Verify the SC-001, SC-004 and SC-005 timings with 20 tracked repositories, confirming a frame within 1 second, every repository resolved and failing ones identifiable within 15 seconds with no keystrokes, and input response under 100 milliseconds during a refresh, as described in `specs/001-devdash-tui/quickstart.md`
- [X] T090 Verify SC-003 by timing a run from an empty configuration to five tracked repositories across two organizations, confirming it completes in under two minutes with no hand-editing, following scenario V5 in `specs/001-devdash-tui/quickstart.md`
- [X] T091 [P] Verify that SIGINT and SIGTERM during a session restore the terminal, by sending each to a running instance and checking `stty -a` afterwards, recorded against `specs/001-devdash-tui/quickstart.md`
- [X] T092 [P] Bring `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` to clean in the whole crate
- [X] T093 [P] Write usage documentation covering the flags and key bindings from contracts/cli.md in `README.md`

---

## Dependencies & Execution Order

### Phase Dependencies

```
Phase 1 Setup
    │
    v
Phase 2 Foundational ────────────── blocks everything below
    │
    v
Phase 3 US1 (P1) 🎯 MVP ─────────── fixtures only, no credentials needed
    │
    v
Phase 4 Live Data ───────────────── blocks US2, US5 for real use
    │
    ├──> Phase 5  US2 (P2)   settings
    ├──> Phase 6  US3 (P3)   fixture mode        (needs only Phase 3)
    ├──> Phase 7  US4 (P4)   filters             (needs Phase 4 for teams)
    ├──> Phase 8  US5 (P5)   refresh
    └──> Phase 9  US6 (P6)   browser handoff     (needs only Phase 3)
                 │
                 v
        Phase 10 Polish
```

### User Story Dependencies

| Story | Depends on | Why |
|---|---|---|
| US1 (P1) | Phase 2 | Nothing else. Runs on fixtures. |
| US2 (P2) | Phase 4 | Needs live organization listing and config persistence |
| US3 (P3) | Phase 3 | Only needs the dashboard to exist to be worth showing |
| US4 (P4) | Phase 4 | Team memberships come from the viewer query |
| US5 (P5) | Phase 4 | Refresh is meaningless against a static fixture |
| US6 (P6) | Phase 3 | Only needs a selected pull request with a URL |

US3 and US6 are the loosest: both can be built straight after the MVP without
waiting for live data.

### Within Each User Story

Tests first, then domain, then state transitions, then rendering. Rendering last
because it is the layer that consumes everything else.

### Parallel Opportunities

| Phase | Parallel tasks | Note |
|---|---|---|
| Phase 1 | T002, T003, T004 | Different files |
| Phase 2 | T005, T006, T007, T008 | One domain module each |
| Phase 2 | T012 then T013 | Same file, sequential |
| Phase 3 | T023, T024, T025, T026 | Same test file, but independent test functions |
| Phase 4 | T035, T036, T038 | Auth and config are independent of the GraphQL client |
| Phase 5 | T047, T048 | Independent test functions |
| Phase 7 | T063, T064, T065 | Independent test functions |
| Phase 10 | T084, T085, T086, T087, T089, T091, T092, T093 | T088 and T090 are sequential, both needing a live timed run |

Note that T040 through T043 are all in the GraphQL client and must run in sequence,
since each builds on the previous one's types.

## Parallel Example: User Story 1

```bash
# The four US1 tests are independent and can be written together:
#   T023 two-pane render
#   T024 empty repository
#   T025 rollup visible without selection
#   T026 colour independence

# Then the two panes, which touch the same file and must be sequential:
#   T028 repository pane, then T029 pull request pane in src/ui/dashboard.rs
```

## Implementation Strategy

### MVP First (User Story 1 only)

Phases 1 through 3, tasks T001 to T034. Thirty-four tasks produce a dashboard that
renders repositories, pull requests and CI state from fixture data, with working
navigation, on any machine, with no credential and no network.

That is a genuinely demonstrable result and the natural first checkpoint.

### Incremental Delivery

1. **MVP**: Phases 1-3. A working dashboard on fixtures.
2. **Real data**: Phase 4. The same dashboard against live GitHub.
3. **Self-service**: Phase 5. Repositories chosen in the interface, not a text editor.
4. **Offline mode**: Phase 6. Fixture mode as a shipped feature.
5. **Work queue**: Phase 7. Filters turn the board into a queue.
6. **Trustworthy**: Phase 8. Refresh and freshness.
7. **Launchpad**: Phase 9. Browser handoff.
8. **Production**: Phase 10. Diagnostics and verification.

Each numbered step is independently demonstrable, which is what makes this
suitable for a live walkthrough.

### Parallel Team Strategy

After Phase 4, four tracks can proceed independently:

- Track A: US2 settings (`src/ui/settings.rs`, `src/config.rs`)
- Track B: US4 filters (`src/app/update.rs`, `src/domain/viewer.rs`)
- Track C: US5 refresh (`src/app/mod.rs`, `src/app/state.rs`)
- Track D: US3 and US6 (`src/cli.rs`, `src/ui/status_bar.rs`)

Tracks B and C both touch `src/app/update.rs` and will need coordination there.
Track D is the most isolated.

## Notes

- **Deviation from the standard phase structure**: Phase 4 is infrastructure with no
  story label, sitting between user story phases. This matches plan.md's sequencing,
  which puts live data after US1 so the first visible result arrives early and
  credentials stay off the critical path. Auth and the GraphQL client are not in
  Phase 2 because they do not block US1, which is the template's test for
  Foundational.
- **Test placement**: tests come before implementation within each story because
  SC-007 makes them a deliverable, not a follow-up.
- **T043 is easy to skip and should not be**: research.md R11 found that ordinary
  repositories exceed the 100 pull request page size, and without pagination the
  repository CI indicator would be quietly wrong on exactly the busiest
  repositories, which is the lie SC-002 depends on not telling.
- **T014 is the highest-leverage task in Phase 2.** Seventeen fixture cases drive
  every interface test in the project. Getting the coverage right there is what
  makes the rest of the suite possible.
