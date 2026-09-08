use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiState {
    Passing,
    Failing,
    Pending,
    NoChecks,
}

impl CiState {
    /// Maps GraphQL `statusCheckRollup.state` to `CiState` (R2).
    /// `None` or unrecognized values map to `NoChecks`.
    pub fn from_graphql_state(state: Option<&str>) -> Self {
        match state {
            Some("SUCCESS") => Self::Passing,
            Some("FAILURE" | "ERROR") => Self::Failing,
            Some("PENDING" | "EXPECTED") => Self::Pending,
            _ => Self::NoChecks,
        }
    }
}

impl fmt::Display for CiState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Passing => write!(f, "passing"),
            Self::Failing => write!(f, "failing"),
            Self::Pending => write!(f, "pending"),
            Self::NoChecks => write!(f, "no checks"),
        }
    }
}

/// Rolls up multiple CI states into a single representative state (FR-006).
/// Returns `None` for an empty slice (FR-007: no indicator for zero PRs).
///
/// Precedence: `Failing` > `Pending` > `Passing` > `NoChecks`.
pub fn rollup(states: &[CiState]) -> Option<CiState> {
    if states.is_empty() {
        return None;
    }

    if states.contains(&CiState::Failing) {
        Some(CiState::Failing)
    } else if states.contains(&CiState::Pending) {
        Some(CiState::Pending)
    } else if states.contains(&CiState::Passing) {
        Some(CiState::Passing)
    } else {
        Some(CiState::NoChecks)
    }
}
