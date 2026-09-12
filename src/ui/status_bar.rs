use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::state::{AppState, FilterMode, Screen, SettingsScreen};
use crate::source::SourceKind;

pub fn render(state: &AppState, frame: &mut Frame, area: Rect) {
    let mut spans: Vec<Span> = Vec::new();

    // Filter mode label
    let filter_label = match state.filter {
        FilterMode::All => "all",
        FilterMode::Mine => "mine",
        FilterMode::ReviewRequested => "review",
    };
    spans.push(Span::styled(
        format!(" [{filter_label}]"),
        Style::default().fg(Color::Cyan),
    ));

    // Degraded filter notice (T068: FR-032)
    if state.filter == FilterMode::ReviewRequested {
        if let Some(viewer) = &state.viewer {
            if !viewer.teams.is_available() {
                spans.push(Span::styled(
                    " ⚠ team filter limited",
                    Style::default().fg(Color::Yellow),
                ));
            }
        }
    }

    // Source indicator
    if state.source_kind == SourceKind::Fixture {
        spans.push(Span::styled(
            " [FIXTURE]",
            Style::default().fg(Color::Yellow),
        ));
    }

    // Refresh state
    if state.refresh.in_flight {
        spans.push(Span::styled(
            " ⟳ refreshing",
            Style::default().fg(Color::Cyan),
        ));
    }

    // Last refresh time (T077)
    if let Some(last) = state.refresh.last_success {
        let now = time::OffsetDateTime::now_utc();
        let elapsed = (now - last).whole_seconds();
        let label = if elapsed < 60 {
            format!(" {elapsed}s ago")
        } else {
            format!(" {}m ago", elapsed / 60)
        };
        spans.push(Span::styled(label, Style::default().fg(Color::DarkGray)));
    }

    if let Some(ref err) = state.refresh.last_error {
        spans.push(Span::styled(
            format!(" ⚠ {err}"),
            Style::default().fg(Color::Red),
        ));
        spans.push(Span::styled(
            format!(" (log: {})", state.log_path.display()),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Rate-limit reset time (T079)
    if let Some(until) = state.refresh.rate_limited_until {
        let now = time::OffsetDateTime::now_utc();
        if until > now {
            let secs = (until - now).whole_seconds();
            spans.push(Span::styled(
                format!(" rate-limited ({secs}s)"),
                Style::default().fg(Color::Red),
            ));
        }
    }

    if let Some(ref msg) = state.status_message {
        if state.status_is_error {
            // FR-064: an error indicator names the log file holding the detail.
            spans.push(Span::styled(
                format!(" ⚠ {msg}"),
                Style::default().fg(Color::Red),
            ));
            spans.push(Span::styled(
                format!(" (log: {})", state.log_path.display()),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {msg}"),
                Style::default().fg(Color::Yellow),
            ));
        }
    }

    // Spacer to push key hints to the right
    let left_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let hints = match &state.screen {
        Screen::Dashboard => "↑↓:nav ←→:pane a/m/v:filter s:settings r:refresh Enter:open q:quit",
        Screen::Settings(SettingsScreen::Organizations) => {
            if state.viewer.is_none() {
                "r:retry Esc:dashboard q:quit"
            } else {
                "↑↓:nav Enter:open Esc:dashboard q:quit"
            }
        }
        Screen::Settings(SettingsScreen::Repositories { .. }) => {
            "↑↓:nav Space:toggle Esc:orgs s:dashboard q:quit"
        }
        Screen::Loading => "q:quit",
        Screen::LoadFailed { .. } => "r:retry q:quit",
    };
    let hints_len = hints.len();
    let total_width = area.width as usize;
    let gap = total_width.saturating_sub(left_len + hints_len + 1);

    if gap > 0 {
        spans.push(Span::raw(" ".repeat(gap)));
    }
    spans.push(Span::styled(hints, Style::default().fg(Color::DarkGray)));

    let bar = Paragraph::new(Line::from(spans));
    frame.render_widget(bar, area);
}
