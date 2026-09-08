# Feature Specification: devdash TUI

**Feature Branch**: `main`

**Created**: 2026-09-08

**Status**: Draft

**Input**: Brainstorm document `brainstorm/01-devdash-tui.md`. Original description: "Build a developer dashboard TUI that shows my git repos, open PRs, and CI status in a single terminal view. Repos are configurable in a dedicated settings screen with drilldown by GitHub organization."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Survey open work across tracked repositories (Priority: P1)

A developer opens the dashboard in a terminal and immediately sees the repositories they track, how many pull requests are open on each, and whether those pull requests are passing CI. Selecting a repository shows its open pull requests with author and check state. No browser tabs, no navigation, no waiting.

**Why this priority**: This is the entire premise. Without it there is no dashboard. Every other story adds convenience or control on top of this view.

**Independent Test**: With a tracked repository set already present in configuration, launch the application and confirm the repository pane and pull request pane both populate, that selecting different repositories updates the pull request pane, and that CI state is distinguishable per pull request and per repository.

**Acceptance Scenarios**:

1. **Given** a configured set of tracked repositories and valid credentials, **When** the application launches, **Then** the repository pane lists each tracked repository with its open pull request count and a rolled-up CI indicator, and the pull request pane shows the open pull requests of the first repository.
2. **Given** the dashboard is displayed, **When** the user moves the selection to a different repository, **Then** the pull request pane replaces its contents with that repository's open pull requests.
3. **Given** a tracked repository with no open pull requests, **When** it is selected, **Then** the repository row shows a count of zero and the pull request pane shows an explicit "no open pull requests" message rather than appearing broken.
4. **Given** a repository whose open pull requests include at least one failing check, **When** the repository pane is displayed, **Then** that repository's rolled-up indicator shows the failing state without the user having to select the repository.
5. **Given** a pull request whose repository has no CI configured, **When** it is displayed, **Then** its indicator is visually distinct from a failing indicator.
6. **Given** the dashboard is displayed, **When** the user presses the quit key, **Then** the application exits and the terminal is restored to its prior state with no leftover artifacts.

---

### User Story 2 - Choose which repositories to track (Priority: P2)

The developer opens a settings screen from the dashboard, sees the GitHub organizations they belong to plus their personal account, drills into one, and toggles individual repositories into or out of the tracked set. The choice survives restarts.

**Why this priority**: Without this, the tracked set can only be changed by hand-editing a configuration file. The dashboard still works, so it is not P1, but the tool is not usable by anyone else until this exists.

**Independent Test**: Launch with an empty tracked set, open settings, drill into an organization, toggle two repositories on, return to the dashboard and confirm both appear. Restart and confirm they are still there.

**Acceptance Scenarios**:

1. **Given** the dashboard is displayed, **When** the user presses the settings key, **Then** the settings screen replaces the dashboard and lists the organizations the authenticated user belongs to, plus their personal account.
2. **Given** the organization list is displayed, **When** the user selects an organization, **Then** the repositories of that organization that the user can access are listed, each showing whether it is currently tracked.
3. **Given** the repository list of an organization is displayed, **When** the user toggles a repository, **Then** its tracked state changes visibly and immediately.
4. **Given** repositories have been toggled, **When** the user returns to the dashboard, **Then** newly tracked repositories appear in the repository pane and untracked ones are gone.
5. **Given** repositories have been toggled, **When** the application is quit and relaunched, **Then** the same tracked set is present.
6. **Given** the repository list of an organization is displayed, **When** the user presses the back key, **Then** the organization list is shown again with the previously selected organization still highlighted.
7. **Given** the tracked set is empty on first launch, **When** the dashboard is displayed, **Then** it shows an empty state that names the settings key rather than a blank or error screen.

---

### User Story 3 - Run without credentials or network (Priority: P3)

The developer runs the dashboard against a committed snapshot of fixture data instead of the live GitHub API. Every screen renders, every CI state appears, and no network request is made. The screen states plainly that the data is not live.

**Why this priority**: This is what makes the application testable and demonstrable at all. Rendering, the CI rollup, filtering and navigation can be verified against known data with no token, no network, and no rate-limit budget spent. It also decouples work on the interface from having credentials at hand. It sits below the settings screen only because it serves the developer rather than the end user.

**Independent Test**: With no credentials available and network access removed, launch the application against the fixture snapshot and confirm the dashboard renders repositories, pull requests, all CI states, and an indicator that fixture data is active.

**Acceptance Scenarios**:

1. **Given** no credentials are available and no network access exists, **When** the application is launched against the fixture snapshot, **Then** the dashboard renders fully and makes no network request.
2. **Given** the fixture data source is active, **When** any screen is displayed, **Then** the screen indicates that the data is fixture data and not live.
3. **Given** the fixture snapshot, **When** the dashboard is displayed, **Then** it contains at least one repository with open pull requests, one repository with none, and pull requests in each of the passing, failing, pending and no-checks states.
4. **Given** the fixture data source is selected, **When** the application starts, **Then** no recompilation was required to select it.
5. **Given** the fixture snapshot file is missing or unreadable, **When** the application is launched against it, **Then** it reports the problem and the path it tried, and exits without a panic or a corrupted terminal.

---

### User Story 4 - Narrow the list to what needs my attention (Priority: P4)

The developer switches the pull request pane between showing all open pull requests, only the ones they authored, and only the ones where their review has been requested.

**Why this priority**: Turns a status board into a work queue. Valuable, but the dashboard is useful without it.

**Independent Test**: With fixture data containing pull requests authored by the fixture user and pull requests with a review requested from them, cycle through the three filter modes and confirm the pull request pane contents change accordingly and the active mode is labelled on screen.

**Acceptance Scenarios**:

1. **Given** the dashboard is displayed, **When** the application starts, **Then** the filter mode is "all open pull requests" and that mode is shown on screen.
2. **Given** any filter mode, **When** the user presses the key for a different mode, **Then** the pull request pane shows only matching pull requests and the on-screen label changes to the new mode.
3. **Given** a non-default filter mode is active, **When** the user selects a different repository, **Then** the filter mode remains in effect rather than resetting.
4. **Given** a filter mode under which the selected repository has no matching pull requests but does have open ones, **When** the pull request pane is displayed, **Then** it distinguishes "no pull requests match this filter" from "this repository has no open pull requests".
5. **Given** the "authored by me" filter is active and it hides some of a repository's open pull requests, **When** the repository pane is displayed, **Then** that repository's count and CI indicator still describe all of its open pull requests, unchanged by the filter.
6. **Given** a pull request whose review is requested from a team the authenticated user belongs to and not from the user directly, **When** the "awaiting my review" filter is active, **Then** that pull request is listed.

---

### User Story 5 - Trust that what I am looking at is current (Priority: P5)

The data refreshes on its own periodically and on demand. The developer can always tell when it was last updated, when an update is running, and when one has failed.

**Why this priority**: A stale dashboard that looks fresh is worse than no dashboard. But an initial fetch at startup, which belongs to P1, already delivers most of the value for a short session.

**Independent Test**: Launch, note the last-updated time, trigger a manual refresh and confirm the in-flight indicator appears and the timestamp advances. Then make the data source fail and confirm the previously fetched data stays on screen alongside an error indicator.

**Acceptance Scenarios**:

1. **Given** the dashboard is displayed, **When** data has been fetched successfully, **Then** the time of the last successful refresh is shown.
2. **Given** the dashboard is displayed, **When** the user presses the refresh key, **Then** a refresh begins immediately, an in-flight indicator is shown, and navigation keys keep working while it runs.
3. **Given** the dashboard is idle, **When** the configured refresh interval elapses, **Then** a refresh begins without user action.
4. **Given** previously fetched data is on screen, **When** a refresh fails, **Then** the previous data remains visible, an error indicator appears, and the application does not exit.
5. **Given** the credential's API rate limit has been exhausted, **When** a refresh is attempted, **Then** the application reports the exhaustion and the time the limit resets, and does not issue further automatic refreshes before that time.
6. **Given** the application has just launched and the first fetch has not returned, **When** the dashboard is displayed, **Then** it shows its layout with an explicit loading state and responds to the quit key.
7. **Given** the very first fetch of a run fails, **When** the dashboard is displayed, **Then** it shows the reason and the retry key, rather than an empty repository pane that could be mistaken for an empty tracked set.
8. **Given** a tracked set where some repositories resolve faster than others, **When** the first fetch is in progress, **Then** resolved repositories appear with their counts and indicators while unresolved ones read as pending, rather than the whole pane waiting on the slowest.
9. **Given** a repository and a pull request are selected and a refresh reorders the pull request list, **When** the refresh completes, **Then** the same repository and the same pull request remain selected.
10. **Given** the selected pull request has been closed and disappears in a refresh, **When** the refresh completes, **Then** the selection moves to the nearest remaining pull request rather than jumping to the top of the list.

---

### User Story 6 - Act on a pull request (Priority: P6)

The developer presses a key on a selected pull request and it opens in their browser, where they can actually review or merge it.

**Why this priority**: Small, but it is what turns the dashboard from a wall of text into a launchpad. Last because everything else is usable without it.

**Independent Test**: Select a pull request, press the open key, and confirm the correct pull request URL is handed to the system browser.

**Acceptance Scenarios**:

1. **Given** a pull request is selected, **When** the user presses the open key, **Then** that pull request's page is opened in the system browser and the dashboard remains running.
2. **Given** no browser can be launched, **When** the user presses the open key, **Then** the application reports that it could not open a browser and shows the URL so the user can copy it, rather than failing silently.

---

### Edge Cases

- The tracked set is empty on first launch, so the dashboard has nothing to show.
- A tracked repository has been deleted, renamed, or made inaccessible since it was added. It must not break the whole refresh.
- The credential is valid but not authorized for a specific organization, for example because that organization enforces SSO. Other organizations must still load.
- The user belongs to no organizations at all, so only their personal account is available in settings.
- An organization contains several hundred repositories, making the settings repository list long to scroll.
- A repository has an unusually large number of open pull requests.
- A pull request's checks are queued but have not started, which must read as pending rather than as no checks.
- The terminal is resized during use, including to a size too small to show two panes meaningfully.
- Network access is unavailable at startup, before any data has ever been fetched.
- The API rate limit is exhausted partway through a refresh, leaving some repositories updated and others not.
- A manual refresh is requested while an automatic refresh is already in flight.
- The credential cannot read the user's team memberships, so team-directed review requests cannot be resolved.
- A repository shows a failing indicator while the active filter hides every failing pull request, so the two panes appear to disagree.
- The fixture snapshot file is absent or malformed.
- The application is terminated abruptly, for example by a signal, and must not leave the terminal in an unusable state.

## Requirements *(mandatory)*

### Functional Requirements

#### Dashboard view

- **FR-001**: The dashboard MUST present two panes simultaneously: a repository pane and a pull request pane for the selected repository.
- **FR-002**: Each repository row MUST show the repository's full name including its owner, the number of open pull requests, and a rolled-up CI indicator. The count and the indicator MUST always describe all open pull requests of that repository, regardless of the active filter mode, so that the repository pane remains a constant overview of repository health.
- **FR-003**: Each pull request row MUST show the pull request number, its title, its author, and its CI indicator.
- **FR-004**: The CI indicator MUST distinguish four states from one another: all checks passing, at least one check failing, checks pending or running, and no checks configured. The no-checks state MUST NOT be presented in a way that could be read as failing.
- **FR-005**: A repository's rolled-up CI indicator MUST be derived from the CI state of its open pull requests using this precedence: failing if any open pull request is failing, otherwise pending if any is pending, otherwise passing if at least one is passing, otherwise no-checks.
- **FR-006**: A repository with no open pull requests MUST show no CI indicator at all. It MUST NOT be shown as no-checks, passing, or failing, because with nothing open there is nothing for CI to report.
- **FR-007**: Users MUST be able to move the selection within the focused pane and move focus between the two panes using the keyboard.
- **FR-008**: Changing the selected repository MUST update the pull request pane to that repository's open pull requests.
- **FR-009**: Both panes MUST scroll when their contents exceed the available height, keeping the current selection visible.
- **FR-010**: The dashboard MUST re-lay out correctly when the terminal is resized, and MUST show a readable message rather than corrupted output when the terminal is too small for the two-pane layout.
- **FR-011**: The application MUST exit on a quit key and MUST restore the terminal to its prior state on exit, including when it exits because of an error.
- **FR-012**: The key bindings available in the current context MUST be discoverable on screen without consulting documentation.
- **FR-013**: Repositories MUST be ordered alphabetically by owner and then by repository name. Pull requests MUST be ordered by their last update time, most recent first.

#### Repository configuration

- **FR-014**: Users MUST be able to open a settings screen from the dashboard with a single key press, and return to the dashboard from it.
- **FR-015**: The settings screen MUST list the GitHub organizations the authenticated user is a member of, together with the user's own personal account, as selectable entries.
- **FR-016**: Selecting an organization MUST list the repositories within it that the authenticated user can access, each showing whether it is currently tracked.
- **FR-017**: Users MUST be able to toggle any listed repository into or out of the tracked set, with the change reflected on screen immediately.
- **FR-018**: Users MUST be able to navigate back from an organization's repository list to the organization list.
- **FR-019**: The tracked repository set MUST be persisted to a user-level configuration file and MUST be restored on the next launch.
- **FR-020**: Returning to the dashboard after changing the tracked set MUST show the updated set without requiring a restart.
- **FR-021**: Archived repositories MUST be excluded from the settings repository list.
- **FR-022**: When an organization's repositories cannot be listed, for example because the credential is not authorized for it, the settings screen MUST report that for the affected organization while leaving the others usable.
- **FR-023**: When the organization list itself cannot be retrieved, the settings screen MUST report why and MUST still offer the user's personal account, so that repositories remain trackable.

#### Filtering

- **FR-024**: The pull request pane MUST support three filter modes: all open pull requests, pull requests authored by the authenticated user, and pull requests where the authenticated user's review has been requested.
- **FR-025**: The "review requested" mode MUST match both pull requests that name the authenticated user directly as a reviewer and pull requests that name a team the authenticated user belongs to.
- **FR-026**: When the authenticated user's team memberships cannot be determined, the "review requested" mode MUST fall back to direct requests only and MUST indicate on screen that team-based requests are not included, rather than silently returning an incomplete list.
- **FR-027**: The active filter mode MUST be visible on screen at all times.
- **FR-028**: The filter mode MUST default to "all open pull requests" at startup.
- **FR-029**: Changing the selected repository MUST NOT reset the active filter mode.
- **FR-030**: When a filter yields no results for a repository that does have open pull requests, the pane MUST say that no pull requests match the filter, distinct from the message shown when the repository has no open pull requests at all.

#### Authentication and credentials

- **FR-031**: The application MUST obtain its API credential from an authenticated `gh` CLI installation, and MUST fall back to the `GITHUB_TOKEN` environment variable when that is unavailable.
- **FR-032**: The application MUST NOT write any credential to its configuration file, to logs, or to the screen, and MUST NOT offer any way to enter a credential within the application.
- **FR-033**: When the live data source is selected and no credential can be obtained, the application MUST report which mechanisms it tried and exit without a panic, rather than presenting an empty dashboard.
- **FR-034**: The application MUST determine the authenticated user's identity and their team memberships, since the filter modes depend on both.

#### Data freshness

- **FR-035**: The application MUST fetch data once at startup.
- **FR-036**: Before the first fetch completes, the dashboard MUST display its layout with an explicit loading state in place of repository and pull request content, and MUST accept the quit key throughout.
- **FR-037**: When the first fetch of a run fails, the dashboard MUST replace the loading state with the reason it failed and the key that retries it. It MUST NOT present an empty repository pane that is indistinguishable from a tracked set of zero repositories.
- **FR-038**: Repositories MUST populate the dashboard progressively as their data resolves, rather than the whole view waiting on the slowest repository. A repository whose data has not resolved yet MUST read as pending rather than being shown with a count of zero or a settled CI indicator.
- **FR-039**: Selection state MUST survive a refresh. After a refresh, the selected repository and the selected pull request MUST remain the same items they were before, even if their positions changed. When a selected item is no longer present, the selection MUST move to the nearest surviving item in the previous ordering rather than jumping to the top of the list.
- **FR-040**: The application MUST refresh data automatically on a configurable interval.
- **FR-041**: Users MUST be able to trigger an immediate refresh with a key press.
- **FR-042**: The interface MUST remain responsive to navigation and filter keys while a refresh is in flight.
- **FR-043**: The application MUST show when a refresh is in flight and the time of the last successful refresh.
- **FR-044**: A failed refresh MUST leave the previously fetched data on screen and surface an error indicator, rather than clearing the view or exiting.
- **FR-045**: When the API rate limit is exhausted, the application MUST report it together with the time the limit resets, and MUST suspend automatic refreshes until then.
- **FR-046**: A refresh requested while one is already in flight MUST NOT start a second concurrent refresh.
- **FR-047**: A tracked repository that can no longer be read MUST remain in the repository pane, showing an unreadable marker in place of its pull request count and CI indicator, together with the reason. Selecting it MUST show that reason in the pull request pane. The remaining repositories MUST refresh normally.

#### Data source selection

- **FR-048**: All repository, pull request and check data MUST reach the interface through a single data access abstraction with interchangeable implementations, rather than through calls made from view code.
- **FR-049**: The application MUST provide a live implementation backed by the GitHub API and a fixture implementation backed by a snapshot committed to the repository.
- **FR-050**: The data source MUST be selectable at startup without recompiling the application.
- **FR-051**: When the fixture data source is active, the application MUST make no network requests.
- **FR-052**: The active data source MUST be identified on screen whenever it is not the live source, so fixture data can never be mistaken for live data.
- **FR-053**: The fixture snapshot MUST cover the states the interface has to render: a repository with open pull requests, a repository with none, and pull requests that are passing, failing, pending, and without CI configured, including at least one authored by the fixture user, one with a review requested from them directly, and one with a review requested from a team they belong to.
- **FR-054**: A missing or malformed fixture snapshot MUST produce a message naming the path and the problem, and an exit without a panic.

#### Pull request handoff

- **FR-055**: Users MUST be able to open the selected pull request in the system browser with a key press, without the dashboard exiting.
- **FR-056**: When no browser can be launched, the application MUST report that and display the pull request's URL.

### Key Entities

- **Organization**: A GitHub account that owns repositories, either an organization the user belongs to or the user's own personal account. Selecting one lists its repositories in settings.
- **Repository**: A GitHub repository identified by owner and name. Carries its set of open pull requests, a rolled-up CI state derived from them, and whether it is currently tracked.
- **Pull Request**: An open pull request on a repository. Carries a number, title, author, last update time, its requested reviewers as both individuals and teams, a CI state, and a web address.
- **CI State**: The rolled-up outcome of the checks on a pull request's head commit, being exactly one of passing, failing, pending, or no checks configured.
- **Tracked Set**: The user's chosen collection of repositories, persisted between runs. It is the input to the dashboard's repository pane.
- **Authenticated User**: The identity behind the credential, together with the organizations and teams they belong to. Determines the organization list in settings and drives the "mine" and "awaiting my review" filters.
- **Data Source**: The provider of repositories, pull requests and check state. Exactly one is active per run: live or fixture.
- **Refresh State**: Whether a fetch is in flight, when the last one succeeded, and what the last failure was, if any.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user with a tracked set of 20 repositories can see which of them have failing pull requests within 15 seconds of launching, with no keystrokes beyond launching.
- **SC-002**: A user asked which of their tracked repositories currently has a failing pull request answers correctly from the first screen, without pressing any key.
- **SC-003**: A new user can go from an empty configuration to a tracked set of at least 5 repositories across 2 organizations in under 2 minutes, without editing any file by hand.
- **SC-004**: With 20 tracked repositories, the dashboard is drawn and accepting input within 1 second of launch, and every repository has resolved to a final count and CI indicator within 15 seconds. Repositories fill in progressively rather than the whole view waiting on the slowest one.
- **SC-005**: Every navigation, filter and pane-switch action produces a visible response within 100 milliseconds, and never waits on a network request.
- **SC-006**: The application runs end to end with no network access and no credentials, rendering every screen and all four CI states.
- **SC-007**: The interface behaviours of rendering, CI rollup, filtering and navigation are all verifiable in automated tests that require neither network access nor credentials.
- **SC-008**: No credential value appears in any file the application writes, in any log output, or anywhere on screen.
- **SC-009**: A refresh that fails, times out, or hits the rate limit never leaves the user with a blank screen, a crash, or a corrupted terminal. Previously fetched data stays visible in every such case.
- **SC-010**: Quitting and relaunching restores the identical tracked set with no reconfiguration.
- **SC-011**: A user can tell at a glance whether the data on screen is live or fixture data, and how old it is.
- **SC-012**: An inaccessible or deleted tracked repository degrades to a marked row and never prevents the other tracked repositories from refreshing.

## Out of Scope

These were considered during brainstorming and deliberately excluded. They are not oversights, and a plan generated from this specification should not include them.

- **Local clone state**: The application does not read the filesystem. Current branch, uncommitted changes, and ahead/behind counts of local clones are not shown. The repository list comes entirely from GitHub.
- **Default branch CI health**: CI is reported only as an attribute of a pull request. Whether the default branch is currently green is a separate signal and is not shown.
- **Check run drilldown**: The interface reports a single rolled-up CI state per pull request. It does not list individual check runs or name which job failed. That detail is reached through the browser.
- **Acting on pull requests**: The application never writes to GitHub. Approving, merging, commenting, closing, and requesting review are all delegated to the browser.
- **Other forges**: GitLab, Bitbucket, and self-hosted forges are not supported. Neither is GitHub Enterprise Server.
- **Search within settings**: The settings repository list is navigated by scrolling. Incremental search and filtering are not provided.
- **Credential entry**: There is no way to enter, store, or manage a token inside the application. Credentials come from `gh` or the environment only.
- **Notifications**: The application does not alert on changes, run in the background, or integrate with system notifications. It shows state while it is open.

## Assumptions

These are reasonable defaults chosen where the brainstorm left a question open. Each can be revisited during planning.

- **Single host**: The application targets github.com with a single authenticated user. GitHub Enterprise Server and multiple simultaneous accounts are out of scope.
- **Credential availability**: For live use, the user has either an authenticated `gh` CLI or `GITHUB_TOKEN` set. The application does not attempt to perform a login flow itself.
- **Configuration location**: The tracked set and settings live in a single user-level configuration file in the platform's standard configuration directory, in a human-readable text format the user may edit by hand.
- **Refresh interval**: The automatic refresh interval defaults to 5 minutes and is configurable in the configuration file. This keeps a tracked set of a few dozen repositories comfortably inside normal API rate limits.
- **Eager check fetching**: Check state is fetched for every open pull request of every tracked repository on each refresh, rather than lazily when a repository is selected. This is what makes the repository-level indicator meaningful at a glance, and it is the reason the refresh interval is measured in minutes rather than seconds. Because a tracked set of 20 repositories implies well over a hundred requests, these are expected to run concurrently and to land in the interface as they resolve, which is what the timing in SC-004 assumes.
- **Expected scale**: The tracked set is expected to hold tens of repositories, not hundreds. Behaviour remains correct beyond that, but refresh duration and rate-limit headroom are only guaranteed at the stated scale.
- **Draft pull requests**: Draft pull requests are open pull requests and are included.
- **Forks**: Forked repositories appear in the settings repository list. Archived ones do not.
- **Filter persistence**: The active filter mode is session state and resets to "all" on each launch. Only the tracked set is persisted.
- **Token scope**: The credential is expected to carry enough scope to read the user's organization and team memberships. Without it the "awaiting my review" filter degrades to direct requests only, per FR-026, and everything else continues to work.
- **Fixture selection**: The fixture data source is selected by a command-line flag at startup, consistent with how command-line tools are normally switched into alternative modes.
- **Fixture authoring**: The fixture snapshot is hand-authored and committed, rather than recorded from a live API response, so that it can be edited directly to cover a new interface state without capturing traffic.
- **Terminal capability**: The terminal supports Unicode and colour. A pure-ASCII rendering mode is not required.
- **Read-only tool**: The application never writes to GitHub. Every action that would change state is delegated to the browser.

## Dependencies

- The GitHub API, for repositories, organizations, pull requests, and check state.
- The `gh` CLI, as the primary source of the API credential.
- A system browser, for the pull request handoff. Its absence degrades one feature but nothing else.
