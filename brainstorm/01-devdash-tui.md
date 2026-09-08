# Brainstorm: devdash TUI

**Date:** 2026-09-08
**Status:** active

## Problem Framing

Keeping track of work in flight across several GitHub repositories means
cycling through browser tabs: one for each repo's PR list, another for the
checks on a PR that just got pushed. The context switch is expensive and the
information is scattered.

devdash collapses that into one terminal view: the repositories being tracked,
the open pull requests on each, and whether CI is green. It is a read-and-jump
dashboard, not a replacement for the GitHub UI. When something needs action,
the dashboard hands off to the browser.

The repositories worth watching change over time, so the tracked set is
configurable inside the application through a settings screen that drills down
by GitHub organization.

## Approaches Considered

Two design axes were explored: where the repository data comes from, and how
the main view is laid out.

### Data source

#### A: Local clones on disk
Rows are local working copies, showing current branch, dirty state, and
ahead/behind counts, with PR and CI data fetched for the matching remote.

- Pros: reflects actual working state, answers "what have I left uncommitted"
- Cons: requires every tracked repo to be cloned; filesystem scanning and path
  configuration add a whole subsystem; repos not yet cloned are invisible

#### B: GitHub repos, remote only (chosen)
The repository list comes from GitHub for the configured organizations. No
filesystem access at all.

- Pros: one data source, one auth mechanism, no path management; works
  identically on any machine
- Cons: no visibility into uncommitted local work

#### C: Both, local state enriching remote
GitHub is the source of truth, and a local clone (when found) adds branch and
dirty state to the row.

- Pros: richest view
- Cons: two data sources to reconcile, clone discovery heuristics, significantly
  more surface area for a first version

### Main view layout

#### A: Two-pane master/detail (chosen)
Repository list on the left with an open-PR count and a rolled-up CI glyph;
the selected repository's open PRs on the right with number, title, author and
CI state. Settings is a separate full-screen mode.

- Pros: repos, PRs and CI are all visible simultaneously, which is the stated
  goal; the repo list stays compact as the tracked set grows
- Cons: only one repository's PRs are visible at a time

#### B: Flat unified PR list
A single scrollable list of every open PR across all tracked repos, grouped
under repo headings.

- Pros: strongest "what can I act on now" framing; combined with filters it
  becomes a genuine work queue
- Cons: repositories with no open PRs disappear entirely, making it a PR
  dashboard rather than a repo dashboard; the list grows long under the
  "all open PRs" default

#### C: Tabbed views
Separate tabs for repos, all PRs, and settings.

- Pros: each view is uncluttered, room to grow
- Cons: three views rather than one, which works against the premise; extra
  navigation state for little benefit at this size

## Decision

Build a GitHub-backed TUI (data source B) with a two-pane master/detail main
view (layout A).

The two chosen options reinforce each other. Dropping local git state removes an
entire subsystem, which keeps the first version small enough to be genuinely
finishable, and the master/detail layout delivers the "single terminal view"
promise directly: repositories, their open PRs, and CI state are all on screen
at once.

Layout B's main advantage, the personal work queue, is recovered without giving
up the repository list by putting a filter toggle on the PR pane: all open PRs
(the default), PRs authored by the user, or PRs where the user is a requested
reviewer.

CI is treated as an attribute of a pull request rather than a separate concept.
Each PR row shows the rolled-up check state of its head commit. Default-branch
health and per-job drilldown are deliberately deferred; they are separate
questions that would each pull in more of the Checks API than a first version
needs.

Authentication reuses the token from an authenticated `gh` CLI, falling back to
`GITHUB_TOKEN`. devdash never stores a credential of its own. Data refreshes on
an interval and on demand via a key press.

The data layer sits behind a single trait rather than calling the GitHub API
directly from the UI. Two implementations ship: the live GitHub client, and a
fixture client that reads a checked-in JSON snapshot. This is not a hedge
against the API, it is what makes the application testable at all. Rendering,
filtering, navigation and the rolled-up CI glyph can be exercised against known
data, without a network round trip, a valid token, or a rate-limit budget spent
on every test run. It also means the UI can be developed and demonstrated
offline, which decouples work on the view from having credentials at hand.

Implemented in Rust with ratatui.

## Key Requirements

### Main view
- Two panes: repository list (left), pull requests of the selected repo (right)
- Repository rows show the repository name, open PR count, and a rolled-up CI
  glyph derived from that repo's open PRs
- Pull request rows show the PR number, title, author, and the rolled-up check
  state of the head commit
- Keyboard navigation between and within panes
- Pressing Enter on a pull request opens it in the system browser

### Filtering
- The PR pane supports three filter modes: all open PRs (default), PRs authored
  by the authenticated user, PRs where the authenticated user is a requested
  reviewer
- The active filter mode is visible on screen
- Filter mode applies across repository selections rather than resetting

### Settings screen
- Reachable from the main view via a key press, and returns to it
- Lists the GitHub organizations the authenticated user belongs to, plus the
  user's personal account
- Selecting an organization lists its repositories
- Individual repositories are toggled on or off to form the tracked set
- The tracked set persists across application restarts
- Changes to the tracked set are reflected in the main view

### Data and authentication
- Repository, pull request, and check data come from the GitHub API
- The API token is read from the authenticated `gh` CLI, falling back to the
  `GITHUB_TOKEN` environment variable
- devdash stores no credentials
- Data refreshes automatically on an interval and immediately on a manual
  refresh key press
- The user can tell when data was last refreshed, and when a refresh is in
  flight

### Data source abstraction
- All repository, pull request and check data reaches the UI through one trait,
  not through direct API calls from view code
- A live implementation backed by the GitHub API
- A fixture implementation backed by a JSON snapshot committed to the repository
- The fixture implementation is selectable at startup without recompiling, and
  the application makes no network calls when it is active
- The active data source is visible on screen, so fixture data is never mistaken
  for live data
- The fixture snapshot covers the states the UI must render: repositories with
  and without open PRs, PRs that are passing, failing, pending, and with no CI
  configured

## Out of Scope

- Local clone state: branch, dirty files, ahead/behind counts
- Default-branch CI health as a separate signal from PR checks
- Drilling into individual check runs to see which job failed
- Acting on pull requests from within the TUI (approve, merge, comment, close)
- Forges other than GitHub
- Incremental search or filtering within the settings repository list
- Entering or storing a personal access token inside the application

## Open Questions

- What the application does when `gh` is not installed, the token is absent,
  the token is expired, or the API rate limit has been exhausted. Does it show
  an error screen, degrade to a partial view, or refuse to start?
- Where the configuration file lives and what format it uses.
- Sort order for the repository list and for the pull request list.
- How a repository with no CI configured is rendered, so that it is clearly
  distinguishable from one that is genuinely failing.
- Rate-limit budget: how many tracked repositories the interval refresh can
  support before it becomes a problem, and what the default interval should be.
- Whether the rolled-up repository CI glyph should reflect all open PRs or only
  the PRs matching the active filter.
- How the fixture data source is selected at startup: a command-line flag, an
  environment variable, or a setting in the configuration file.
- Whether the fixture snapshot is handwritten or recorded from a real API
  response, and if recorded, how it gets refreshed when the shape of the data
  changes.
