# devdash

A terminal dashboard for GitHub pull requests and CI state. Track repositories
across organizations, see which PRs need attention, and open them in a browser,
all without leaving the terminal.

Built live with Spec-Driven Development for the talk
*"Spec-Driven Development: Engineering with Intent"* (Code Europe 2026, Warsaw).

## Quick start

```bash
# Build
cargo build --release

# Run against fixture data (no credentials needed)
cargo run -- --fixtures

# Run against live GitHub data
cargo run
```

## Installation

Requires Rust 1.93 or later.

```bash
cargo install --path .
```

## Usage

```
devdash [OPTIONS]
```

### Options

| Flag | Description |
|------|-------------|
| `--fixtures [PATH]` | Use fixture data instead of live API (default: `fixtures/snapshot.json`) |
| `--config PATH` | Alternate configuration file |
| `--log-level LEVEL` | Log verbosity: error, warn, info, debug, trace (default: info) |
| `--refresh-interval SECS` | Override refresh interval for this run |
| `--print-log-path` | Print log file path and exit |

### Key bindings

#### Dashboard

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Move selection |
| `h` / `l` / Tab | Switch pane focus |
| `a` | Filter: all PRs |
| `m` | Filter: my PRs |
| `v` | Filter: awaiting my review |
| `s` | Open settings |
| `r` | Refresh now |
| `Enter` | Open selected PR in browser |
| `q` / Ctrl-C | Quit |

#### Settings

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Move selection |
| `Enter` | Drill into organization |
| `Space` | Toggle repository tracking |
| `Esc` / `h` | Go back |
| `q` / Ctrl-C | Quit |

## Configuration

Configuration is stored at `~/.config/devdash/config.toml` (Linux) or
`~/Library/Application Support/devdash/config.toml` (macOS).

```toml
refresh_interval_secs = 300

[[tracked]]
id = 600886023
owner = "ratatui"
name = "ratatui"
```

Tracked repositories are chosen through the settings screen (`s` key). The
file is safe to hand-edit while devdash is not running.

## Credentials

devdash reads a GitHub token from `gh auth token` (preferred) or the
`GITHUB_TOKEN` environment variable. It never stores credentials.

## Fixture mode

Run `devdash --fixtures` for a fully offline demo. The committed fixture at
`fixtures/snapshot.json` covers all interface states: four CI states, draft
PRs, team review requests, empty repos, and more.

## Demo checkpoints

Each phase of the SDD flow is captured as a branch pointer:

| Branch | State |
|--------|-------|
| `00-start` | Empty project, spex installed |
| `01-after-brainstorm` | Brainstorm document |
| `02-after-spec` | Formal spec (user stories, acceptance criteria) |
| `03-after-plan` | Implementation plan |
| `04-after-tasks` | Task breakdown + REVIEWERS.md |
| `05-after-implement` | Working TUI |
| `06-after-review` | Deep review output |
