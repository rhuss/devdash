# Specification Quality Checklist: devdash TUI

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-08
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

### Iteration 1 (2026-09-08)

Two [NEEDS CLARIFICATION] markers remain, both raised to the user:

- **FR-002**: whether the repository pane's open pull request count and rolled-up CI
  indicator follow the active filter or always describe all open pull requests.
  Scope-significant: it determines whether the repository pane is a constant
  overview or a filtered summary, and whether the two panes can appear to
  disagree.
- **FR-022**: whether "review requested" includes requests directed at a team the
  user belongs to. In organizations that route reviews through teams, the
  narrow reading makes the filter return nothing.

Both were left as markers rather than defaulted because each has two defensible
answers with materially different user-visible behaviour.

### Iteration 2 (2026-09-08)

Both markers resolved by the user, and the consequences propagated through the
spec rather than being recorded only at the point of the question:

- **FR-002** now states that the repository pane's count and CI indicator always
  describe all open pull requests, independent of the filter. New acceptance
  scenario US4.5 covers it, and a new edge case records the resulting tension
  where a repository reads red while the filtered pull request pane looks clean.
- **FR-022a** now includes team-directed review requests; **FR-022b** defines the
  degraded behaviour when team memberships cannot be read, so the mode fails
  visibly rather than silently returning a short list. Propagated to FR-030
  (identity now includes team memberships), FR-045 (the fixture snapshot must
  carry a team-requested pull request), the Pull Request and Authenticated User
  entities, acceptance scenario US4.6, a new edge case, and a token scope
  assumption.

### Iteration 3 (2026-09-08) - review-spec gate

The `review-spec` gate found 6 Important and 4 Minor issues. All were fixed.

Important:

1. **No Out of Scope section.** The brainstorm's exclusions had not been carried
   into the spec, so nothing told a planner they were excluded. Added as a
   dedicated section with 8 entries.
2. **FR-005 rollup had a hole for empty repositories.** A repository with zero
   open pull requests fell through the precedence chain to "no checks
   configured", which is a CI claim about a repository that has nothing to
   report. New FR-006 requires no indicator at all in that case.
3. **The pre-first-fetch state was undefined.** FR-044's "previously fetched data
   remains visible" is vacuous on first launch, leaving the "network unavailable
   at startup" edge case ungoverned. Added FR-036 (loading state, quit key live
   throughout) and FR-037 (first-fetch failure shows reason and retry key rather
   than an empty pane).
4. **Selection behaviour across a refresh was undefined.** Since FR-013 sorts
   pull requests by last update, ordering churns on every refresh. Added FR-039:
   selection follows the item, and falls back to the nearest survivor when the
   item is gone.
5. **SC-004 conflicted with the eager-fetching assumption.** "Complete data within
   5 seconds" for 20 repositories implies over a hundred round trips. Rewritten
   as two separate measurements: drawn and accepting input within 1 second,
   fully resolved within 15 seconds. SC-001 was realigned to match, the eager
   fetching assumption now states the concurrency it depends on, and new FR-038
   requires progressive population so the criterion has a requirement behind it.
6. **SC-002 was not measurable.** It restated FR-002 and FR-004 as a design
   property and duplicated SC-001. Rewritten as an observable user outcome.

Minor:

7. FR numbering flattened. FR-022a/FR-022b style suffixes were removed and all
   requirements renumbered sequentially. The single cross-reference in the
   Assumptions section was re-pointed at the correct requirement.
8. FR-047 now states what an unreadable repository shows in place of its count
   and indicator, instead of "marked as such".
9. Typo fixed in US2 acceptance scenario 7.
10. Added FR-023, covering failure to retrieve the organization list at all.
    FR-022 had only covered per-organization repository listing.

Five acceptance scenarios were added to User Story 5 so the new requirements
carry test coverage, and two edge cases were added earlier for the filter and
team-membership consequences.

All checklist items pass. Final counts: 56 functional requirements,
12 success criteria, 6 user stories, 0 unresolved markers.

### Resolved by informed default (documented in the Assumptions section)

The remaining open questions carried over from `brainstorm/01-devdash-tui.md` were
resolved with reasonable defaults rather than escalated: configuration file
location and format, repository and pull request sort order, rendering of the
no-CI state (now FR-004), default refresh interval and rate-limit budget, how the
fixture data source is selected at startup, and whether the fixture snapshot is
hand-authored or recorded.

### Content quality note

The GitHub API, the `gh` CLI, and the system browser appear in the specification.
These are the feature's problem domain and external dependencies, not chosen
implementation technology. No programming language, framework, or internal code
structure is named. The data source abstraction (FR-040 through FR-046) is stated
in user-observable terms: interchangeable sources, selectable at startup, with the
active one shown on screen.
