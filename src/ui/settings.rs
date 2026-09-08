use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::state::{AppState, OrgRepoState, SettingsScreen};

pub fn render(state: &AppState, settings_screen: &SettingsScreen, frame: &mut Frame, area: Rect) {
    match settings_screen {
        SettingsScreen::Organizations => render_orgs(state, frame, area),
        SettingsScreen::Repositories {
            org,
            state: org_state,
        } => render_repos(state, org, org_state, frame, area),
    }
}

fn render_orgs(state: &AppState, frame: &mut Frame, area: Rect) {
    let orgs = state.organizations();

    if orgs.is_empty() {
        let msg = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                " No organizations available",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " Press Esc to return",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Settings ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = orgs
        .iter()
        .enumerate()
        .map(|(i, org)| {
            let label = if i == 0 {
                format!("{org} (personal)")
            } else {
                org.clone()
            };
            ListItem::new(Line::from(Span::raw(format!(" {label}"))))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings - Organizations ")
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut list_state = ListState::default().with_selected(Some(state.settings_state.org_index));

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_repos(
    state: &AppState,
    org: &str,
    org_state: &OrgRepoState,
    frame: &mut Frame,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Settings - {org} "))
        .border_style(Style::default().fg(Color::Cyan));

    match org_state {
        OrgRepoState::Loading => {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    " Loading repositories...",
                    Style::default().fg(Color::Cyan),
                )),
            ])
            .block(block);
            frame.render_widget(msg, area);
        }
        OrgRepoState::Failed { reason } => {
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!(" Failed: {reason}"),
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Press r to retry, Esc to go back",
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(block);
            frame.render_widget(msg, area);
        }
        OrgRepoState::Ready { repos } => {
            let visible: Vec<_> = repos.iter().filter(|r| !r.is_archived).collect();

            if visible.is_empty() {
                let msg = Paragraph::new(vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        " No repositories (archived repos are hidden)",
                        Style::default().fg(Color::DarkGray),
                    )),
                ])
                .block(block);
                frame.render_widget(msg, area);
                return;
            }

            let items: Vec<ListItem> = visible
                .iter()
                .map(|repo| {
                    let tracked = state.is_tracked(repo.id);
                    let marker = if tracked { "[✓]" } else { "[ ]" };
                    let style = if tracked {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {marker} "), style),
                        Span::raw(&repo.name),
                        if repo.is_fork {
                            Span::styled(" (fork)", Style::default().fg(Color::DarkGray))
                        } else {
                            Span::raw("")
                        },
                    ]))
                })
                .collect();

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            let mut list_state =
                ListState::default().with_selected(Some(state.settings_state.repo_index));

            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}
