use crossterm::event::KeyEvent;

use crate::domain::{OrgRepo, Viewer};
use crate::source::{RepositoryData, SourceError};

pub enum Event {
    Key(KeyEvent),
    Tick,
    DashboardData(Result<Vec<RepositoryData>, SourceError>),
    ViewerData(Result<Viewer, SourceError>),
    OrgRepoData {
        org: String,
        result: Result<Vec<OrgRepo>, SourceError>,
    },
    Error(String),
}
