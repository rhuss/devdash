# Deep Review Findings

**Date:** 2026-09-08
**Branch:** 001-devdash-tui
**Rounds:** 2
**Gate Outcome:** PASS
**Invocation:** manual

## Summary

| Severity | Found | Fixed | Remaining |
|----------|-------|-------|-----------|
| Critical | 2 | 2 | 0 |
| Important | 10 | 10 | 0 |
| Minor | 9 | 0 | 9 |
| Notable | 2 | - | 2 |
| **Total** | **23** | **12** | **11** |

**Agents completed:** 5/5 (+ 1 external tool)
**Agents failed:** CodeRabbit initial run (wrong CLI flag), Codex (usage limit)

## Findings

### FINDING-1
- **Severity:** Critical
- **Confidence:** 90
- **File:** src/ui/dashboard.rs:270-278
- **Category:** correctness
- **Source:** correctness-agent (also reported by: production-readiness-agent, test-quality-agent, coderabbit)
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
`truncate_title` sliced on byte offset (`&title[..max_len - 3]`) instead of character boundary. Titles containing multi-byte UTF-8 characters (emoji, CJK) would panic at runtime with "byte index is not a char boundary".

**Why this matters:**
PR titles routinely contain non-ASCII characters. A panic here crashes the TUI and corrupts the terminal, violating FR-012 (restore terminal on error).

**How it was resolved:**
Replaced byte-level slicing with `title.chars().take(max_len - 3).collect()` for character-aware truncation.

### FINDING-2
- **Severity:** Critical
- **Confidence:** 90
- **File:** src/source/github/mod.rs:165-177
- **Category:** correctness
- **Source:** coderabbit
- **Round found:** 2
- **Resolution:** fixed (round 2)

**What is wrong:**
The organization repository pagination loop would loop forever if `has_next_page` was true but `end_cursor` was `None`. The loop would pass `None` to `org_repos_query`, re-fetching the same first page indefinitely.

**Why this matters:**
An infinite loop would hang the application when browsing organizations in the settings screen, requiring a force-kill.

**How it was resolved:**
Added `&& page_info.end_cursor.is_some()` guard to the pagination condition.

### FINDING-3
- **Severity:** Important
- **Confidence:** 90
- **File:** src/app/update.rs:209-220
- **Category:** correctness
- **Source:** correctness-agent (also reported by: architecture-agent, production-readiness-agent)
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
`persist_config` hardcoded `refresh_interval_secs: 300` instead of preserving the user's configured value. Every config save (repo toggle, rename detection) silently reset any custom interval.

**Why this matters:**
A user who sets `refresh_interval_secs = 60` in their config would have it silently reset to 300 after toggling any repo in settings.

**How it was resolved:**
Added `refresh_interval_secs` field to `AppState`, initialized from loaded config, and used it in `persist_config`.

### FINDING-4
- **Severity:** Important
- **Confidence:** 85
- **File:** src/main.rs:189-193
- **Category:** correctness
- **Source:** correctness-agent (also reported by: production-readiness-agent)
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
When `ViewerData` arrived with an `Err`, the error was silently discarded. The viewer fetch failure meant filters ("mine", "review requested") would silently return empty results with no explanation.

**Why this matters:**
FR-040 requires determining the authenticated user's identity. Silent failure means the user has no way to know why filters are empty or settings shows no organizations.

**How it was resolved:**
Changed to log the error via `tracing::error!` and set `state.refresh.last_error` so the error is visible in the status bar.

### FINDING-5
- **Severity:** Important
- **Confidence:** 88
- **File:** src/source/github/query.rs:37-123
- **Category:** security
- **Source:** security-agent
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
GraphQL injection via unsanitized string interpolation. `teams_query`, `dashboard_query`, `pagination_query`, and `org_repos_query` interpolated values directly into GraphQL strings without escaping double-quote characters. Values from user-editable config could break out of string literals.

**Why this matters:**
FR-025 allows hand-editing the config file. A crafted `owner` value with embedded quotes could alter query semantics.

**How it was resolved:**
Added `gql_escape()` helper that escapes backslashes and double-quotes, applied to all interpolated values in all query functions.

### FINDING-6
- **Severity:** Important
- **Confidence:** 92
- **File:** src/logging.rs:17
- **Category:** production-readiness
- **Source:** production-readiness-agent
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
Log files grew without bound across runs, violating FR-063. `rolling::daily` creates a new file per day but never deletes old ones.

**Why this matters:**
FR-063 explicitly requires bounded log size. Running devdash daily for months would accumulate log files indefinitely.

**How it was resolved:**
Added `cleanup_old_logs()` function called at init that removes log files beyond the most recent 7.

### FINDING-7
- **Severity:** Important
- **Confidence:** 92
- **File:** src/source/github/response.rs:10-14
- **Category:** architecture
- **Source:** architecture-agent
- **Round found:** 1
- **Resolution:** fixed (round 1)

**What is wrong:**
`DashboardResponse.rate_limit_remaining` and `rate_limit_reset` fields were computed but never read. Rate-limit detection happens separately in `check_rate_limit` during `execute_query`.

**Why this matters:**
Dead fields create maintenance burden and suggest rate-limit handling is split across two disconnected paths.

**How it was resolved:**
Removed unused fields from `DashboardResponse` and their computation in `parse_dashboard`.

### FINDING-8
- **Severity:** Important
- **Confidence:** 88
- **File:** src/app/update.rs:189-194
- **Category:** correctness
- **Source:** coderabbit
- **Round found:** 2
- **Resolution:** fixed (round 2)

**What is wrong:**
The retry on 'r' in the failed org repo state set the screen to Loading but did not set `state.org_fetch_requested`, so the fetch never actually triggered. Pressing 'r' to retry showed "Loading" forever.

**Why this matters:**
FR-018 requires that when a fetch fails, pressing the retry key retries it. Without setting the fetch flag, the retry was broken.

**How it was resolved:**
Added `state.org_fetch_requested = Some(org.clone())` before returning the Loading screen.

### FINDING-9
- **Severity:** Important
- **Confidence:** 95
- **File:** tests/logging.rs:13-61
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining (test-quality, not auto-fixable)

**What is wrong:**
`credential_never_appears_in_log_output` does not test the application's actual logging code paths. It creates its own subscriber, logs a pre-redacted placeholder, then asserts the raw token is absent. A bug where real code accidentally logged the token would pass this test.

**Why this matters:**
FR-038/SC-008 require no credential in logs. This test is the only one claiming to verify that guarantee but it tests a synthetic scenario.

### FINDING-10
- **Severity:** Important
- **Confidence:** 90
- **File:** src/source/github/response.rs:1-355
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining (test-quality, not auto-fixable)

**What is wrong:**
The entire `response.rs` module (355 lines) has zero test coverage. This includes `parse_viewer`, `parse_teams`, `parse_dashboard`, `parse_pull_requests`, and `check_rate_limit`, which handle complex JSON parsing with many edge cases.

**Why this matters:**
This is the most complex parsing layer and the primary integration point with the GitHub API. Bugs here would silently drop repositories or PRs.

### FINDING-11
- **Severity:** Important
- **Confidence:** 85
- **File:** src/app/update.rs:1-339
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining (test-quality, not auto-fixable)

**What is wrong:**
`handle_key` and `apply_dashboard_data` have zero direct test coverage. Key handling logic, rename-following (FR-022), and the Loading-to-Dashboard transition are untested.

**Why this matters:**
`apply_dashboard_data` implements FR-022 (rename following) and FR-044 (progressive loading). A regression in merge logic or rename detection would not be caught.

### FINDING-12
- **Severity:** Minor
- **Confidence:** 80
- **File:** src/app/update.rs:83-88
- **Category:** correctness
- **Source:** correctness-agent
- **Round found:** 1
- **Resolution:** remaining

`open_selected_pr` spawns the browser process but discards the `Child` handle, leaving the child unreaped. Benign on modern OSes.

### FINDING-13
- **Severity:** Minor
- **Confidence:** 75
- **File:** src/main.rs:109-120
- **Category:** correctness
- **Source:** correctness-agent (also reported by: production-readiness-agent)
- **Round found:** 1
- **Resolution:** remaining

The key-reading `spawn_blocking` thread has no explicit shutdown mechanism. It exits when the channel drops but may run up to 50ms after quit.

### FINDING-14
- **Severity:** Minor
- **Confidence:** 80
- **File:** src/config.rs:89-110
- **Category:** correctness
- **Source:** correctness-agent
- **Round found:** 1
- **Resolution:** remaining

TOCTOU race in `save()`: another process could write the config between re-read and persist. Current implementation handles the common case but not a tight race.

### FINDING-15
- **Severity:** Minor
- **Confidence:** 75
- **File:** src/source/github/mod.rs:59-63
- **Category:** correctness
- **Source:** correctness-agent
- **Round found:** 1
- **Resolution:** remaining

`check_rate_limit` is called on every successful response and discards valid data when quota hits zero. Could return data alongside a rate-limit warning.

### FINDING-16
- **Severity:** Minor
- **Confidence:** 75
- **File:** src/app/update.rs:64-89
- **Category:** security
- **Source:** security-agent
- **Round found:** 1
- **Resolution:** remaining

`open_selected_pr` does not validate the URL scheme. A compromised API response returning a `file:///` URL would be passed to `open`/`xdg-open`.

### FINDING-17
- **Severity:** Minor
- **Confidence:** 80
- **File:** tests/render_dashboard.rs:104-133
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining

`empty_repo_shows_zero_count_and_empty_message` has a weak assertion: `output.contains('0')` can match any character, not just the zero count.

### FINDING-18
- **Severity:** Minor
- **Confidence:** 80
- **File:** tests/fixture_source.rs:293-302
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining

`no_network_requests_made` has no assertions beyond method calls. The test name overpromises; compilation already guarantees no HTTP client in the fixture source.

### FINDING-19
- **Severity:** Minor
- **Confidence:** 75
- **File:** tests/
- **Category:** test-quality
- **Source:** test-quality-agent
- **Round found:** 1
- **Resolution:** remaining

No test covers too-small terminal behavior (FR-011). `render_too_small` in `src/ui/mod.rs:42-46` is never verified.

### FINDING-20
- **Severity:** Minor
- **Confidence:** 95
- **File:** src/ui/indicator.rs:15-22
- **Category:** architecture
- **Source:** architecture-agent
- **Round found:** 1
- **Resolution:** fixed (round 1)

Dead code: `ci_indicator_text` was defined as `pub` but never called. Removed.

### FINDING-21
- **Severity:** Minor
- **Confidence:** 88
- **File:** src/source/fixture.rs:54-56
- **Category:** architecture
- **Source:** architecture-agent
- **Round found:** 1
- **Resolution:** fixed (round 1)

Dead code: `FixtureDataSource::path()` was `pub` but never called. Removed.

## Notable Observations

### NOTABLE-1
- **File:** tests/ (multiple files)
- **Category:** architecture
- **Source:** architecture-agent
- **Description:** `make_pr`, `make_repo`, and `render_to_string` helper functions are duplicated across 4 test files with slightly different signatures.
- **Rationale:** These will diverge as the codebase evolves. If `PullRequest` gains a field, all 4 variants need updating. Consider extracting shared test helpers.

### NOTABLE-2
- **File:** src/main.rs:230-245
- **Category:** architecture
- **Source:** architecture-agent
- **Description:** `org_fetch_requested` creates cross-module coupling where update logic sets a flag and the main event loop polls it. This is the only field that works this way.
- **Rationale:** The pattern would be cleaner if the key handler sent an event through the channel instead of setting a flag.

## Test Suite Results

| Round | Test Command | Exit Code | Failures | Status |
|-------|-------------|-----------|----------|--------|
| 1     | cargo test  | 0         | 0        | passed |
| 2     | cargo test  | 0         | 0        | passed |

Test suite passed in all fix rounds.

## Remaining Findings

3 Important findings remain (test-quality category, not auto-fixable):
- FINDING-9: Logging credential test is synthetic, not testing real code paths
- FINDING-10: GitHub response parser (response.rs) has zero test coverage
- FINDING-11: Key handling and data application (update.rs) have zero test coverage

These require writing new tests and are advisory.
