# Contract: Command-line interface

**Feature**: [../spec.md](../spec.md) | **Plan**: [../plan.md](../plan.md)

The command-line surface is small by design. Everything the user configures during
normal use lives in the settings screen (FR-015 through FR-019), not in flags.

## Synopsis

```
devdash [OPTIONS]
```

## Options

| Flag | Argument | Default | Requirement | Behaviour |
|---|---|---|---|---|
| `--fixtures [PATH]` | optional path | `fixtures/snapshot.json` | FR-056 | Use the fixture data source instead of the live API. No network request is made (FR-057). |
| `--config PATH` | path | platform config dir | FR-021 | Use an alternate configuration file. Mainly for tests and for demonstrating a clean first run. |
| `--log-level LEVEL` | `error`\|`warn`\|`info`\|`debug`\|`trace` | `info` | FR-063 | Verbosity of the log file. Never changes what is written to the screen. |
| `--refresh-interval SECS` | integer, minimum 30 | from config, else 300 | FR-046 | Override the automatic refresh interval for this run only. Not persisted. |
| `--print-log-path` | | | FR-064 | Print the resolved log file path to stdout and exit. Gives the path a scriptable route as well as an on-screen one. |
| `--version` | | | | Print version and exit. |
| `--help` | | | | Print usage and exit. |

`--fixtures` satisfies FR-056's requirement that the data source be selectable at
startup without recompiling. It takes an optional path so that the common case is
a bare `--fixtures` while tests can point at their own snapshot.

## Exit codes

| Code | Meaning | Requirement |
|---|---|---|
| 0 | Normal exit via the quit key | FR-012 |
| 1 | No credential could be obtained in live mode | FR-039 |
| 2 | Fixture snapshot missing or malformed | FR-060 |
| 3 | Terminal could not be initialised |  |

Every non-zero exit prints its reason to stderr *after* the terminal has been
restored, so the message is readable rather than being swallowed by the alternate
screen (FR-012).

## Standard streams

- **stdout**: used only by `--help`, `--version` and `--print-log-path`. During a session the terminal
  is in raw mode on the alternate screen and stdout carries the rendered interface.
- **stderr**: carries the final error message on a non-zero exit, and nothing during
  a normal session. Diagnostic detail goes to the log file instead (FR-063), because
  a TUI cannot show stderr while it owns the screen.

## Environment

| Variable | Requirement | Use |
|---|---|---|
| `GITHUB_TOKEN` | FR-037 | Fallback credential when `gh auth token` is unavailable |
| `NO_COLOR` | FR-005 | When set, colour is suppressed. All four CI states remain distinguishable by symbol, so no information is lost (SC-013). |

The application reads no other environment variables and writes none.

## Key bindings

The interface must make its own bindings discoverable on screen (FR-013), so this
table is the contract, not the documentation the user is expected to consult.

### Dashboard

| Key | Action | Requirement |
|---|---|---|
| `↑` / `k`, `↓` / `j` | Move selection within the focused pane | FR-008 |
| `←` / `h`, `→` / `l`, `Tab` | Move focus between panes | FR-008 |
| `a` | Filter: all open pull requests | FR-030 |
| `m` | Filter: authored by me | FR-030 |
| `v` | Filter: awaiting my review | FR-030 |
| `s` | Open settings | FR-015 |
| `r` | Refresh now | FR-047 |
| `Enter` | Open the selected pull request in the browser | FR-061 |
| `q`, `Ctrl-C` | Quit | FR-012 |

`v` rather than `r` for the review filter, because `r` is refresh and a mistaken
refresh is more annoying than a mistaken filter change.

### Settings

| Key | Action | Requirement |
|---|---|---|
| `↑` / `k`, `↓` / `j` | Move selection | FR-016, FR-017 |
| `Enter` | Drill into the selected organization | FR-017 |
| `Space` | Toggle the selected repository | FR-019 |
| `Esc`, `h`, `←` | Back to the organization list, or to the dashboard | FR-015, FR-020 |
| `r` | Retry a failed organization listing | FR-018 |
| `q`, `Ctrl-C` | Quit | FR-012 |

### Loading and failure screens

| Key | Action | Requirement |
|---|---|---|
| `q`, `Ctrl-C` | Quit, available throughout loading | FR-042 |
| `r` | Retry the first fetch | FR-043 |
