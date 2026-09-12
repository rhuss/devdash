# Implementation Plan: devdash TUI

**Branch**: `001-devdash-tui` | **Date**: 2026-09-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-devdash-tui/spec.md`

## Summary

A terminal dashboard that shows tracked GitHub repositories, their open pull
requests, and CI state in one two-pane view, with a settings screen for choosing
repositories by organization.

The technical approach rests on three decisions from [research.md](./research.md).
First, all data comes from a single aliased GraphQL query per refresh, which
collapses what would be well over a hundred REST requests into one round trip
costing one rate-limit point. Second, all data reaches the interface through one
`DataSource` abstraction with a live implementation and a fixture implementation,
so the entire interface is testable and demonstrable with no network and no
credentials. Third, the state and rendering core is pure and synchronous while all
I/O happens on a `tokio` runtime behind channels, which is what keeps input
responsive during a refresh.

## Technical Context

**Language/Version**: Rust 1.93.0 (verified locally), edition 2024

**Primary Dependencies**:

| Crate | Version | Purpose |
|---|---|---|
| `ratatui` | 0.30 | Terminal interface rendering |
| `crossterm` | 0.29 | Terminal backend, raw mode, input events |
| `tokio` | 1.53 | Async runtime for all I/O |
| `reqwest` | 0.13 | HTTPS client for the GraphQL endpoint (see R5) |
| `serde` / `serde_json` | 1.0 | GraphQL response and fixture deserialization |
| `time` | 0.3 | `OffsetDateTime` for pull request update times, refresh timestamps, and rate-limit reset times. Features `serde` and `parsing` for the ISO-8601 values GraphQL returns. |
| `async-trait` | 0.1 | Async methods on the `DataSource` trait, required because the design holds it as `Box<dyn DataSource>` for runtime source selection (FR-056) |
| `toml` | 1.1 | Configuration file format |
| `directories` | 6.0 | Per-platform config and state directory resolution |
| `tracing`, `tracing-subscriber`, `tracing-appender` | 0.1 / 0.3 / 0.2 | Bounded rolling log file |
| `thiserror` | 2.0 | Typed domain errors |
| `anyhow` | 1.0 | Error context at the application boundary |
| `tempfile` | 3.27 | Atomic configuration writes |
| `clap` | 4 | Command-line argument parsing |
| `insta` (dev) | 1.48 | Snapshot tests of rendered frames |

Versions read from crates.io on 2026-09-08. No `octocrab`, per R5.

**Storage**: TOML configuration file in the platform config directory; JSON fixture
snapshot committed to the repository; rolling log file in the platform state
directory. No database.

**Testing**: `cargo test`, with `ratatui::backend::TestBackend` for render
assertions and `insta` for frame snapshots. Every interface test runs against the
fixture data source, so the suite needs neither network nor credentials.

**Target Platform**: macOS and Linux terminals supporting Unicode. Colour is used
where available but is never load-bearing (FR-005).

**Project Type**: Single Rust binary, a terminal user interface application.

**Performance Goals**: Drawn and accepting input within 1 second of launch; all 20
tracked repositories resolved within 15 seconds (SC-004); every navigation, filter
and pane-switch action visible within 100 milliseconds and never blocking on the
network (SC-005).

**Constraints**: No credential is ever written to disk, logs, or screen (FR-038,
SC-008). No network request at all when the fixture source is active (FR-057). All
four CI states remain distinguishable with colour removed (FR-005, SC-013).

**Scale/Scope**: Tens of tracked repositories, hundreds of open pull requests. Six
user stories, 64 functional requirements, two screens.

## Global Constraints

**Every task inherits this section.** These hold across the whole implementation, so
they are not restated per task. Values are copied verbatim from [spec.md](./spec.md).

| # | Constraint | Source |
|---|---|---|
| G1 | The application MUST NOT write any credential to its configuration file, to logs, or to the screen, and MUST NOT offer any way to enter a credential within the application. | FR-038, SC-008 |
| G2 | Each CI state MUST be carried by a distinct symbol, so that all four remain distinguishable with colour removed entirely. Colour MAY reinforce the state but MUST NOT be the only thing that separates one state from another. | FR-005, SC-013 |
| G3 | When the fixture data source is active, the application MUST make no network requests. | FR-057, SC-006 |
| G4 | The application never writes to GitHub. Every action that would change state is delegated to the browser. | Assumptions, read-only tool |
| G5 | The application MUST restore the terminal to its prior state on exit, including when it exits because of an error. | FR-012, SC-009 |
| G6 | All repository, pull request and check data MUST reach the interface through the `DataSource` abstraction, never through calls made from view code. | FR-054 |
| G7 | Rust 1.93.0 or later, edition 2024. Crate versions as pinned in Technical Context above. | Technical Context |
| G8 | The terminal supports Unicode. A pure-ASCII rendering mode is not required. | Assumptions |
| G9 | Targets github.com only. GitHub Enterprise Server and multiple simultaneous accounts are out of scope. | Assumptions |

G1, G2, G3 and G6 are the ones a task can violate without noticing. A logging call
that formats a request with its headers breaks G1; a status glyph that differs only
by colour breaks G2; an `#[allow]` on an unused HTTP client in the fixture path
breaks G3; a render function that reaches for `reqwest` breaks G6.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

**Status: not applicable, with a caveat.**

`.specify/memory/constitution.md` exists but is the unmodified spec-kit template.
Every principle, section, and governance rule is still a bracketed placeholder
(`[PRINCIPLE_1_NAME]`, `[GOVERNANCE_RULES]`, and so on). The project has no
ratified constitution, so there are no gates to evaluate and none can fail.

This is recorded rather than passed silently, because a vacuous gate is not the
same as a satisfied one. If principles are ratified later, this plan should be
re-checked against them. The design choices most likely to interact with a future
constitution are noted here so that re-check is cheap:

| Design choice | Would engage a principle about |
|---|---|
| `DataSource` trait with live and fixture implementations | Testability, dependency inversion |
| Pure synchronous state core, I/O behind channels | Simplicity, testability |
| Single binary, no library split | Library-first structure |
| Snapshot tests as the primary interface assertion | Test-first discipline |
| Rolling log file, no structured telemetry export | Observability |

**Post-Phase 1 re-check**: unchanged. No constitution exists, so the Phase 1 design
introduces no violations. Complexity Tracking is therefore empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-devdash-tui/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── cli.md           # Command-line surface
│   ├── config.md        # Configuration file schema
│   ├── fixture.md       # Fixture snapshot schema
│   ├── data-source.md   # The DataSource abstraction
│   └── github-graphql.md# GraphQL queries and response shapes
├── checklists/
│   └── requirements.md  # Spec quality checklist
└── tasks.md             # Phase 2 output (/speckit-tasks, NOT created here)
```

### Source Code (repository root)

```text
Cargo.toml
fixtures/
└── snapshot.json             # Committed fixture data (FR-059)

src/
├── main.rs                   # Entry, terminal lifecycle, panic hook (FR-012)
├── cli.rs                    # Argument parsing (contracts/cli.md)
├── auth.rs                   # gh auth token, then GITHUB_TOKEN (FR-037, FR-039)
├── config.rs                 # Load, merge, atomic write (FR-021, FR-023, FR-024, FR-025)
├── logging.rs                # Rolling file subscriber (FR-063, FR-064)
├── domain/
│   ├── mod.rs
│   ├── ci.rs                 # CiState and the rollup rule (FR-004, FR-006, FR-007)
│   ├── repository.rs         # Repository, RepoId, RepoStatus
│   ├── pull_request.rs       # PullRequest, ReviewRequest
│   └── viewer.rs             # Viewer identity and team slugs (FR-040)
├── source/
│   ├── mod.rs                # DataSource trait (FR-054)
│   ├── github/
│   │   ├── mod.rs            # GithubDataSource
│   │   ├── query.rs          # Aliased query construction (R1)
│   │   └── response.rs       # Deserialization into domain types (R2)
│   └── fixture.rs            # FixtureDataSource (FR-055, FR-057, FR-060)
├── app/
│   ├── mod.rs                # Event loop wiring
│   ├── state.rs              # AppState, Screen, FilterMode, Selection
│   ├── event.rs              # Event enum
│   └── update.rs             # Pure state transitions (FR-045)
└── ui/
    ├── mod.rs                # Render dispatch by screen
    ├── dashboard.rs          # Two-pane view (FR-001..FR-014)
    ├── settings.rs           # Organization and repository lists (FR-015..FR-029)
    ├── indicator.rs          # CI symbols, colour-independent (FR-005)
    └── status_bar.rs         # Filter mode, freshness, source, log path

tests/
├── ci_rollup.rs              # Rollup precedence and the empty-repo case
├── filtering.rs              # Three filter modes, including team requests
├── selection.rs              # Selection survives refresh (FR-039)
├── config_merge.rs           # Concurrent write merge, atomicity, bad file
├── render_dashboard.rs       # TestBackend and insta snapshots
├── render_settings.rs        # Loading, error, and empty states
└── fixture_source.rs         # Full run against fixtures, no network
```

**Structure Decision**: A single binary crate with modules split by
responsibility rather than by layer. The four top-level modules mirror the
architecture: `domain` holds types with no dependencies, `source` holds everything
that touches the network or disk, `app` holds pure state transitions, and `ui`
holds pure rendering. The dependency direction is strictly `ui` and `source` onto
`domain`, and `app` onto all three, with no cycles.

No library and binary split, because nothing outside this project consumes these
types. Integration tests reach the modules through the binary crate's test target.

## Implementation Phases

Phases follow the specification's user story priorities. Each is independently
demonstrable, which is what makes this suitable for a live walkthrough.

| Phase | User story | Delivers | Key requirements |
|---|---|---|---|
| 1 | Foundation | Domain types, CI rollup, fixture source, terminal lifecycle | FR-004..FR-007, FR-012, FR-054..FR-060 |
| 2 | US1 (P1) | Two-pane dashboard rendering against fixtures | FR-001..FR-014 |
| 3 | Live data | Auth, GraphQL client, config load, first fetch | FR-021, FR-037..FR-043 |
| 4 | US2 (P2) | Settings screen, tracked set persistence and merge | FR-015..FR-029, FR-065 |
| 5 | US4 (P4) | Filter modes including team-directed requests | FR-030..FR-036 |
| 6 | US5 (P5) | Interval and manual refresh, freshness, failure handling | FR-044..FR-053 |
| 7 | US6 (P6) | Browser handoff | FR-061, FR-062 |
| 8 | Diagnostics | Rolling log file and discoverable path | FR-063, FR-064 |

Phase 1 precedes US1 because the fixture source is what US1 is tested against, and
Phase 3 follows US1 because the dashboard can be built and demonstrated entirely
against fixtures before any network code exists. That ordering is deliberate: it
keeps the first visible result early and keeps credentials off the critical path.

## Design Risks

Recorded here rather than discovered during implementation.

| Risk | Mitigation |
|---|---|
| A repository with more than 100 open pull requests would produce a wrong rollup if only the first page is fetched (R11) | `totalCount` supplies the exact count; paginate the remainder before computing the rollup. Only affects repositories that need it. |
| GraphQL response shape drift breaks deserialization silently | Deserialize into explicit structs with `deny_unknown_fields` off but required fields present; a shape change becomes a typed error surfaced per repository, not a panic. |
| The token lacks `read:org`, so team memberships cannot be read | FR-032 already specifies the fallback: direct requests only, stated on screen. |
| A panic while the terminal is in raw mode leaves the terminal unusable | Install a panic hook that restores the terminal before the default hook runs (FR-012). |
| Snapshot tests become brittle as the layout evolves | Assert on the text buffer for behavioural properties (rollup, filtering, symbols) and reserve full-frame snapshots for a small number of representative screens. |

## Complexity Tracking

No constitution exists, so there are no violations to justify. This section is
intentionally empty.
