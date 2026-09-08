use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::state::{AppState, FilterMode, Pane};
use crate::domain::pull_request::PullRequest;
use crate::domain::repository::RepoStatus;
use crate::domain::viewer::Viewer;
use crate::domain::{CiState, rollup};
use crate::ui::indicator::ci_indicator;
use crate::ui::status_bar;

pub fn render(state: &AppState, frame: &mut Frame, area: Rect) {
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);

    let main_area = chunks[0];
    let bar_area = chunks[1];

    if state.repos.is_empty() {
        render_empty_dashboard(frame, main_area);
    } else {
        render_panes(state, frame, main_area);
    }

    status_bar::render(state, frame, bar_area);
}

fn render_empty_dashboard(frame: &mut Frame, area: Rect) {
    let msg = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            " No repositories tracked",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(" Press s to open settings and choose repositories."),
    ])
    .block(Block::default().borders(Borders::ALL).title(" devdash "));
    frame.render_widget(msg, area);
}

fn render_panes(state: &AppState, frame: &mut Frame, area: Rect) {
    let panes =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);

    render_repo_pane(state, frame, panes[0]);
    render_pr_pane(state, frame, panes[1]);
}

fn render_repo_pane(state: &AppState, frame: &mut Frame, area: Rect) {
    let focused = state.selection.focus == Pane::Repositories;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = state
        .repos
        .iter()
        .map(|repo| {
            let ci_states: Vec<CiState> = match &repo.status {
                RepoStatus::Ready { pulls, .. } => pulls.iter().map(|p| p.ci).collect(),
                _ => Vec::new(),
            };

            let mut spans = Vec::new();

            // CI rollup indicator
            match rollup(&ci_states) {
                Some(ci) => {
                    spans.push(ci_indicator(ci));
                    spans.push(Span::raw(" "));
                }
                None => {
                    spans.push(Span::styled("  ", Style::default()));
                }
            }

            // Repository name
            spans.push(Span::raw(format!("{}/{}", repo.owner, repo.name)));

            // Open count
            match &repo.status {
                RepoStatus::Ready { open_count, .. } => {
                    spans.push(Span::styled(
                        format!(" ({open_count})"),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                RepoStatus::Pending => {
                    spans.push(Span::styled(" ...", Style::default().fg(Color::DarkGray)));
                }
                RepoStatus::Unreadable { reason } => {
                    spans.push(Span::styled(
                        format!(" ⚠ {reason}"),
                        Style::default().fg(Color::Red),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(" Repositories ");

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let selected_index = state.selection.selected_repo_index(&state.repos);
    let mut list_state = ListState::default().with_selected(selected_index);

    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_pr_pane(state: &AppState, frame: &mut Frame, area: Rect) {
    let focused = state.selection.focus == Pane::PullRequests;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selected_repo = state
        .selection
        .repo
        .and_then(|id| state.repos.iter().find(|r| r.id == id));

    let unreadable_msg;
    let (title, content) = match selected_repo {
        None => (
            " Pull Requests ".to_string(),
            PrPaneContent::Empty("Select a repository"),
        ),
        Some(repo) => {
            let title = format!(" Pull Requests - {}/{} ", repo.owner, repo.name);
            match &repo.status {
                RepoStatus::Ready {
                    pulls, open_count, ..
                } => {
                    if pulls.is_empty() && *open_count == 0 {
                        (title, PrPaneContent::Empty("No open pull requests"))
                    } else {
                        let filtered = filtered_pulls(pulls, state.filter, state.viewer.as_ref());
                        if filtered.is_empty() {
                            (
                                title,
                                PrPaneContent::Empty("No pull requests match this filter"),
                            )
                        } else {
                            (title, PrPaneContent::Pulls(filtered))
                        }
                    }
                }
                RepoStatus::Pending => (title, PrPaneContent::Empty("Loading...")),
                RepoStatus::Unreadable { reason } => {
                    unreadable_msg = format!("Unreadable: {reason}");
                    (title, PrPaneContent::Empty(&unreadable_msg))
                }
            }
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title);

    match content {
        PrPaneContent::Empty(msg) => {
            let paragraph = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!(" {msg}"),
                    Style::default().fg(Color::DarkGray),
                )),
            ])
            .block(block);
            frame.render_widget(paragraph, area);
        }
        PrPaneContent::Pulls(pulls) => {
            let items: Vec<ListItem> = pulls
                .iter()
                .map(|pr| {
                    let spans = vec![
                        ci_indicator(pr.ci),
                        Span::raw(" "),
                        Span::styled(
                            format!("#{}", pr.number),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" "),
                        Span::raw(truncate_title(
                            &pr.title,
                            area.width.saturating_sub(25) as usize,
                        )),
                        Span::styled(
                            format!("  @{}", pr.author),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ];
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let selected_index = state
                .selection
                .pull
                .and_then(|pr_num| pulls.iter().position(|p| p.number == pr_num));

            let list = List::new(items)
                .block(block)
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            let mut list_state = ListState::default().with_selected(selected_index);
            frame.render_stateful_widget(list, area, &mut list_state);
        }
    }
}

enum PrPaneContent<'a> {
    Empty(&'a str),
    Pulls(Vec<&'a PullRequest>),
}

fn filtered_pulls<'a>(
    pulls: &'a [PullRequest],
    filter: FilterMode,
    viewer: Option<&Viewer>,
) -> Vec<&'a PullRequest> {
    match filter {
        FilterMode::All => pulls.iter().collect(),
        FilterMode::Mine => pulls
            .iter()
            .filter(|p| viewer.is_some_and(|v| p.author == v.login))
            .collect(),
        FilterMode::ReviewRequested => pulls
            .iter()
            .filter(|p| {
                viewer.is_some_and(|v| {
                    p.review_requests.iter().any(|rr| {
                        rr.is_for_user(&v.login)
                            || v.teams.slugs().iter().any(|slug| rr.is_for_team(slug))
                    })
                })
            })
            .collect(),
    }
}

fn truncate_title(title: &str, max_len: usize) -> String {
    if title.len() <= max_len {
        title.to_string()
    } else if max_len > 3 {
        format!("{}...", &title[..max_len - 3])
    } else {
        title[..max_len].to_string()
    }
}
