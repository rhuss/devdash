use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::domain::CiState;

pub fn ci_indicator(state: CiState) -> Span<'static> {
    match state {
        CiState::Passing => Span::styled("✓", Style::default().fg(Color::Green)),
        CiState::Failing => Span::styled("✗", Style::default().fg(Color::Red)),
        CiState::Pending => Span::styled("●", Style::default().fg(Color::Yellow)),
        CiState::NoChecks => Span::styled("○", Style::default().fg(Color::DarkGray)),
    }
}
