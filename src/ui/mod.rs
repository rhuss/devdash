pub mod dashboard;
pub mod indicator;
pub mod settings;
pub mod status_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::state::{AppState, Screen};

const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 10;

pub fn render(state: &AppState, frame: &mut Frame) {
    let area = frame.area();

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        render_too_small(frame, area);
        return;
    }

    match &state.screen {
        Screen::Dashboard => dashboard::render(state, frame, area),
        // FR-042: the loading state belongs inside the dashboard's own layout.
        Screen::Loading => dashboard::render_loading(state, frame, area),
        other => {
            let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
            match other {
                Screen::LoadFailed { reason } => render_load_failed(frame, chunks[0], reason),
                Screen::Settings(settings_screen) => {
                    settings::render(state, settings_screen, frame, chunks[0]);
                }
                Screen::Dashboard | Screen::Loading => unreachable!(),
            }
            status_bar::render(state, frame, chunks[1]);
        }
    }
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let msg = Paragraph::new("Terminal too small. Resize to continue.")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(msg, area);
}

fn render_load_failed(frame: &mut Frame, area: Rect, reason: &str) {
    let chunks = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(5),
        Constraint::Fill(1),
    ])
    .split(area);

    let error = Paragraph::new(vec![
        Line::from(Span::styled(
            " Failed to load",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(format!(" {reason}")),
        Line::from(""),
        Line::from(Span::styled(
            " Press r to retry, q to quit",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL).title(" devdash "));

    frame.render_widget(error, chunks[1]);
}
