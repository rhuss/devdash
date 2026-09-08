# Review Guide: devdash TUI

**Generated**: 2026-09-08 | **Spec**: [spec.md](spec.md)

## Why This Change

Tracking work across several GitHub repositories currently means cycling through
browser tabs: one per repository's pull request list, another for the checks on a
branch that was just pushed. The information is scattered and the context switch is
expensive, so the practical result is that people check less often than they should
and find out about a red build later than they could.

devdash collapses that into a single terminal view: the repositories being tracked,
the open pull requests on each, and whether CI is green. It is a read-and-jump
dashboard, not a replacement for the GitHub UI.

## What Changes

A new Rust terminal application. Two panes show tracked repositories with open pull
request counts and a rolled-up CI indicator on the left, and the selected
repository's open pull requests with author and check state on the right. A settings
screen lets the user pick repositories by drilling down through their GitHub
organizations, and that choice persists across restarts. Three filter modes turn the
board into a personal work queue: everything, authored by me, or awaiting my review.
Pressing Enter on a pull request opens it in the browser.

The application is strictly read-only against GitHub. Every action that would change
state is delegated to the browser. It stores no credential of its own, reading one
from an authenticated `gh` CLI or `GITHUB_TOKEN`.

No breaking changes: this is a new project with no existing users.

## How It Works

**Data access behind one abstraction.** Everything the interface displays arrives
through a single `DataSource` trait with two implementations: a live GitHub client
and a fixture client reading a committed JSON snapshot. The interface never sees an
API shape. This is what makes the whole application testable and demonstrable with
no network and no credential, which the spec requires as a success criterion, not
as a convenience.

**One GraphQL query per refresh.** Rather than the REST approach of one request per
repository plus one per pull request for check state, a single aliased GraphQL query
covers every tracked repository. This was measured during research, not assumed:
three repositories and sixty pull requests with full CI rollup came back in one
request costing one rate-limit point of 5000 per hour. GitHub's `statusCheckRollup`
also maps directly onto the four CI states the spec defines, including returning
`null` for a commit with no checks at all.

**Pure core, I/O at the edges.** State transitions and rendering are pure
synchronous functions. All I/O runs on a tokio runtime and reaches the core as
events over a channel. The render path never awaits, which is what keeps navigation
responsive within 100 milliseconds while a refresh is in flight.

**Identity over position.** The tracked set is keyed on GitHub's stable numeric
repository id, so renames and transfers are followed rather than silently dropping a
repository. Selection is likewise stored as identity, not as a list index, because
pull requests sort by last update and therefore reorder on nearly every refresh.

Module layout, dependency direction, and the full file map are in
[plan.md](plan.md). Contracts for the CLI, config file, data source, GraphQL
queries, and fixture schema are in [contracts/](contracts/).

## When It Applies

**Applies when**:

- Watching pull requests across roughly tens of GitHub repositories on github.com
- Working in a terminal and wanting status without leaving it
- A credential is available from `gh` or `GITHUB_TOKEN`, or the fixture source is
  selected for offline and demo use

**Does not apply when**:

- You need local working-copy state such as current branch, uncommitted changes, or
  ahead/behind counts. The application never reads the filesystem.
- You want to know whether the default branch is green. CI is reported only as an
  attribute of a pull request.
- You need to see which specific job failed. Only a rolled-up state is shown; the
  detail is one keypress away in the browser.
- You want to approve, merge, comment, or request review from the terminal. The
  application never writes to GitHub.
- You use GitLab, Bitbucket, or GitHub Enterprise Server.

## Key Decisions

1. **GitHub-only, no local git state.** Considered reading local clones for branch
   and dirty state, and a hybrid where GitHub is authoritative and a local clone
   enriches the row. Chose remote-only: it removes an entire subsystem (clone
   discovery, path configuration, filesystem access) and works identically on any
   machine. The cost is no visibility into uncommitted local work.

2. **Two-pane master/detail over a flat pull request list.** A flat list of every
   open pull request grouped by repository is a stronger work queue, but
   repositories with no open pull requests vanish from it entirely, making it a pull
   request dashboard rather than a repository dashboard. The two-pane layout shows
   repositories, pull requests and CI simultaneously, which is the stated goal, and
   the filter modes recover the work-queue behaviour without losing the repository
   overview.

3. **GraphQL, not REST.** Verified by measurement rather than argument: the aliased
   query returns all tracked repositories in one round trip at one rate-limit point,
   where REST would need roughly 120 requests for 20 repositories. It also supplies
   the four-state CI rollup natively instead of requiring client-side aggregation
   over individual check runs.

4. **The repository pane ignores the active filter.** The count and CI indicator
   always describe all open pull requests. The alternative, following the filter,
   keeps the two panes consistent but destroys the left pane's value as a stable
   health overview. This trade-off is called out again under Areas Needing Attention.

5. **"Awaiting my review" includes team-directed requests.** Matching only direct
   requests is simpler and more precise, but in organizations that route reviews
   through teams the filter would return nothing at all. Team memberships are read
   once per run, and the filter degrades visibly to direct-only when the token lacks
   the scope to read them.

6. **A fixture data source as a shipped feature, not a test double.** It could have
   been a test-only construct. Making it a first-class mode selectable by a flag,
   with the active source shown on screen, means the interface can be developed,
   demonstrated, and reviewed offline.

7. **No credential storage of any kind.** No token entry in the application, no
   token in the config file. Only `gh auth token` and `GITHUB_TOKEN`. This removes a
   whole class of security review from the project.

## Areas Needing Attention

**The two panes can appear to disagree.** Under the "authored by me" filter, a
repository may show a failing indicator while its visible pull requests all look
clean, because the failure belongs to someone else's pull request. This is the
deliberate consequence of decision 4 and is recorded as an edge case. A reviewer may
reasonably think a filtered count, or a combined `2/7` display, would be less
confusing. Worth a second opinion.

**Pagination is easy to skip and would be quietly wrong.** GraphQL caps a page at
100 pull requests. Research found `rust-lang/cargo` with 90 open and
`ratatui/ratatui` with 76, so ordinary repositories are near the boundary. Without
the follow-up paginated request, the repository indicator would be wrong on exactly
the busiest repositories, which is the one place the dashboard's core claim matters
most. Task T043 exists for this; please confirm it is not dropped as an optimization.

**Eager check fetching is assumed.** Check state is fetched for every open pull
request of every tracked repository on each refresh, rather than lazily on
selection. This is what makes the repository indicator meaningful at a glance, and
the GraphQL finding makes it cheap, but it is a deliberate choice worth confirming.

**The five-minute default refresh interval is a guess.** It is comfortable against
the rate limit but nobody has used the tool yet. It may prove too slow to feel live
or too fast to be useful.

**Phase 4 in tasks.md is infrastructure sitting between user story phases.** This
deviates from the standard setup-foundational-stories-polish structure. The reason
is sequencing: US1 is built and demonstrated on fixtures before any network code
exists, keeping credentials off the critical path. Reviewers who prefer strict phase
conventions may object.

**Snapshot tests can become brittle.** Full-frame snapshots are reserved for a few
representative screens, with behavioural properties asserted against the text buffer
instead. If the layout churns during implementation, this balance may need revisiting.

**The project has no constitution.** `.specify/memory/constitution.md` is still the
unmodified template, so the Constitution Check gate was recorded as not applicable
rather than passed. No principle has been checked against because none exists.

## Open Questions

None blocking. Five ambiguities were raised and resolved during clarification, and
are recorded in spec.md's Clarifications section: log destination, repository
identity across renames, CI state encoding, settings loading behaviour, and
concurrent configuration writes.

Two items are deliberately deferred rather than unresolved:

- Whether the default refresh interval should change once the tool has real use.
- Whether the fixture snapshot should eventually be recorded from live responses
  rather than hand-authored, if its maintenance burden grows.

## Review Checklist

- [ ] Key decisions are justified
- [ ] Breaking changes are documented with migration guidance
- [ ] Scope matches the stated boundaries
- [ ] Success criteria are achievable
- [ ] No unstated assumptions
- [ ] The repository pane's unfiltered counts (decision 4) are the right call
- [ ] Pagination for repositories exceeding 100 open pull requests is retained
- [ ] No credential can reach the config file, the log file, or the screen
- [ ] All four CI states stay distinguishable with colour removed

---

<!-- Code phase sections are appended below this line by the phase-manager command -->
