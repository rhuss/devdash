use devdash::domain::ci::{CiState, rollup};

#[test]
fn empty_slice_returns_none() {
    assert_eq!(rollup(&[]), None);
}

#[test]
fn single_no_checks_returns_no_checks() {
    assert_eq!(rollup(&[CiState::NoChecks]), Some(CiState::NoChecks));
}

#[test]
fn passing_with_no_checks_returns_passing() {
    assert_eq!(
        rollup(&[CiState::Passing, CiState::NoChecks]),
        Some(CiState::Passing)
    );
}

#[test]
fn passing_with_pending_returns_pending() {
    assert_eq!(
        rollup(&[CiState::Passing, CiState::Pending]),
        Some(CiState::Pending)
    );
}

#[test]
fn pending_with_failing_returns_failing() {
    assert_eq!(
        rollup(&[CiState::Pending, CiState::Failing]),
        Some(CiState::Failing)
    );
}

#[test]
fn all_passing_returns_passing() {
    assert_eq!(
        rollup(&[CiState::Passing, CiState::Passing, CiState::Passing]),
        Some(CiState::Passing)
    );
}

#[test]
fn failing_wins_over_all_others() {
    assert_eq!(
        rollup(&[
            CiState::Passing,
            CiState::Pending,
            CiState::Failing,
            CiState::NoChecks
        ]),
        Some(CiState::Failing)
    );
}
