# Quickstart: devdash TUI

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

How to run and validate the feature end to end. Each scenario maps to a user story
and is checkable by eye or by a command, with no reading of source required.

## Prerequisites

| Requirement | Check | Needed for |
|---|---|---|
| Rust 1.93 or later | `rustc --version` | Everything |
| A terminal supporting Unicode | | Everything |
| `gh` authenticated, or `GITHUB_TOKEN` set | `gh auth status` | Live scenarios only |

Scenarios V1 through V4 need **no credential and no network**. That is the point of
the fixture data source, and it is what SC-006 and SC-007 measure.

## Build and test

```bash
cargo build
cargo test          # entire suite runs offline, no credential
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The full test suite passing without network access or a token is itself a check on
SC-007. If any test needs either, the `DataSource` abstraction has leaked.

## Offline validation

These run against the committed fixture. Schema and required coverage are in
[contracts/fixture.md](./contracts/fixture.md).

### V1: The dashboard renders (US1, P1)

```bash
cargo run -- --fixtures
```

**Expect**: two panes. Left lists repositories with an open pull request count and a
CI symbol. Right lists the selected repository's open pull requests with number,
title, author and CI symbol. A status bar shows the filter mode and states that
fixture data is in use.

| Check | Requirement |
|---|---|
| Arrow keys move the selection; the right pane follows the left | FR-008, FR-009 |
| `Tab` moves focus between panes | FR-008 |
| A repository with zero open pull requests shows `0` and **no** CI symbol | FR-007 |
| Selecting it shows "no open pull requests", not a blank pane | US1 scenario 3 |
| At least one repository shows failing without being selected | FR-006, SC-002 |
| A no-checks pull request is clearly not a failure | FR-004 |
| `q` exits and the terminal is left usable | FR-012 |

### V2: Colour is not load-bearing (SC-013)

```bash
NO_COLOR=1 cargo run -- --fixtures
```

**Expect**: all four CI states still tell apart by symbol alone. This is the check
that FR-005 is real rather than aspirational. Run it in a monochrome terminal too if
you have one.

### V3: Filters, including team-directed reviews (US4, P4)

With V1 running, press `a`, then `m`, then `v`.

| Check | Requirement |
|---|---|
| The status bar names the active mode at all times | FR-033 |
| `m` shows only pull requests authored by the fixture viewer | FR-030 |
| `v` includes a pull request requested from a *team* the viewer belongs to, not only direct requests | FR-031 |
| Left pane counts and symbols **do not change** when the filter changes | FR-002 |
| A repository with open pull requests but no matches says so, distinctly from a repository with none | FR-036 |

The fourth row is the one worth watching. It is the clarification decision that the
repository pane stays an unfiltered health overview.

### V4: Fixture failure modes (FR-060)

```bash
cargo run -- --fixtures /nonexistent.json ; echo "exit=$?"
```

**Expect**: a message naming the path, exit code 2, terminal intact, no panic.
Repeat with a deliberately malformed JSON file and expect the parse error named.

## Live validation

These need a credential. They spend a handful of rate-limit points.

### V5: First run and repository selection (US2, P2)

```bash
cargo run -- --config /tmp/devdash-demo.toml
```

**Expect**: an empty dashboard naming the settings key, since the tracked set is
empty (US2 scenario 7). Then press `s`.

| Check | Requirement |
|---|---|
| Organizations are listed, plus the personal account | FR-016 |
| Entering one shows a loading state, not an empty list | FR-018 |
| `Space` toggles a repository, visibly and immediately | FR-019 |
| `Esc` returns to the organization list, then to the dashboard | FR-020, FR-015 |
| Newly tracked repositories appear without a restart | FR-026 |
| Archived repositories are absent | FR-027 |

Then quit and relaunch with the same `--config`, and confirm the tracked set
survived (FR-021, SC-010).

```bash
cat /tmp/devdash-demo.toml   # id, owner, name per entry; readable and editable
```

### V6: Refresh and freshness (US5, P5)

| Action | Expect | Requirement |
|---|---|---|
| Watch the status bar | Time of last successful refresh | FR-049 |
| Press `r` | In-flight indicator appears; arrow keys keep working during it | FR-047, FR-048 |
| Press `r` twice quickly | One refresh, not two | FR-052 |
| Disconnect the network, press `r` | Previous data stays on screen, error indicator appears, no exit | FR-050 |
| Reconnect, press `r` | Recovers | |

Selection is the subtle one. Note the selected pull request, trigger a refresh that
reorders the list, and confirm the same pull request is still selected rather than
the same row position (FR-045).

### V7: Browser handoff (US6, P6)

Select a pull request and press `Enter`. It opens in the browser and devdash keeps
running (FR-061). With no browser available, expect the URL shown for copying rather
than a silent failure (FR-062).

### V8: Diagnostics (FR-063, FR-064)

Trigger any error, such as V6's disconnected refresh.

| Check | Requirement |
|---|---|
| The interface makes the log file path discoverable | FR-064 |
| The log contains the underlying cause, not just the summary | FR-063 |
| The log does **not** contain the token | FR-038, SC-008 |

```bash
grep -ri "ghp_\|github_pat_\|bearer" "$(devdash --print-log-path)" && echo "LEAK" || echo "clean"
```

The last check is worth running deliberately. SC-008 is the one success criterion
where a quiet failure would be genuinely damaging.

## Performance checks

| Criterion | How to check |
|---|---|
| SC-004: drawn and accepting input within 1 second | Launch with 20 tracked repositories; the frame appears before data resolves, per FR-042 |
| SC-004: all repositories resolved within 15 seconds | Watch pending rows settle |
| SC-005: input responds within 100 ms and never blocks | Hold an arrow key during a refresh; movement stays smooth |

Per [research.md](./research.md) R1, a full refresh is one request costing one
rate-limit point, so these should be comfortable rather than marginal. If SC-004 is
tight in practice, the aliased query is not being used as designed.

## Requirement to scenario map

| User story | Scenario | Needs network |
|---|---|---|
| US1, survey open work (P1) | V1, V2 | no |
| US2, choose repositories (P2) | V5 | yes |
| US3, run without credentials (P3) | V1, V2, V3, V4 | no |
| US4, filter to what needs me (P4) | V3 | no |
| US5, trust the data is current (P5) | V6 | yes |
| US6, act on a pull request (P6) | V7 | yes |
| Diagnostics | V8 | yes |
