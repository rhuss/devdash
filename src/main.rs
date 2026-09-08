use std::io;
use std::panic;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;

use devdash::app::event::Event;
use devdash::app::state::{AppState, OrgRepoState, Screen, SettingsScreen};
use devdash::app::update::{apply_dashboard_data, handle_key};
use devdash::auth;
use devdash::cli::Cli;
use devdash::config;
use devdash::logging;
use devdash::source::DataSource;
use devdash::source::fixture::FixtureDataSource;
use devdash::source::github::GithubDataSource;
use devdash::ui;

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode().context("failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .context("failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend).context("failed to create terminal")
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() {
    install_panic_hook();

    if let Err(e) = run().await {
        restore_terminal();
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

#[allow(clippy::too_many_lines)]
async fn run() -> Result<()> {
    let cli = Cli::parse();

    let log_path = logging::log_path();

    if cli.print_log_path {
        println!("{}", log_path.display());
        return Ok(());
    }

    let _log_guard = logging::init(&log_path, &cli.log_level);

    let config_path = config::config_path(cli.config.as_deref());
    let cfg = config::load(&config_path);

    let refresh_secs = cli.refresh_interval.unwrap_or(cfg.refresh_interval_secs);
    let refresh_interval = Duration::from_secs(refresh_secs.max(30));

    let source: Arc<dyn DataSource> = if let Some(fixture_path) = &cli.fixtures {
        match FixtureDataSource::load(fixture_path) {
            Ok(fixture) => Arc::new(fixture),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(2);
            }
        }
    } else {
        match auth::acquire_token().await {
            Ok(token) => Arc::new(GithubDataSource::new(token)),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    };

    let source_kind = source.kind();
    let mut state = AppState::new(source_kind, log_path);
    state.config_path = config_path.clone();
    state.tracked = cfg.tracked.clone();

    let mut terminal = setup_terminal()?;

    let (tx, mut rx) = mpsc::channel::<Event>(64);

    let key_tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        use crossterm::event::{self as ct_event, Event as CtEvent};
        loop {
            if ct_event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(CtEvent::Key(key)) = ct_event::read() {
                    if key_tx.blocking_send(Event::Key(key)).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let tracked = cfg.tracked.clone();
    let fetch_source = Arc::clone(&source);
    let fetch_tx = tx.clone();
    tokio::spawn(async move {
        let viewer_result = fetch_source.viewer().await;
        let _ = fetch_tx.send(Event::ViewerData(viewer_result)).await;

        let dashboard_result = fetch_source.dashboard(&tracked).await;
        let _ = fetch_tx.send(Event::DashboardData(dashboard_result)).await;
    });

    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);
        interval.tick().await;
        loop {
            interval.tick().await;
            if tick_tx.send(Event::Tick).await.is_err() {
                break;
            }
        }
    });

    loop {
        terminal.draw(|frame| ui::render(&state, frame))?;

        match tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            Ok(Some(event)) => match event {
                Event::Key(key) => handle_key(&mut state, key),
                Event::Tick => {
                    let rate_ok = state
                        .refresh
                        .rate_limited_until
                        .is_none_or(|until| time::OffsetDateTime::now_utc() >= until);
                    if !state.refresh.in_flight && rate_ok {
                        state.refresh.in_flight = true;
                        let src = Arc::clone(&source);
                        let t = state.tracked.clone();
                        let ev_tx = tx.clone();
                        tokio::spawn(async move {
                            let result = src.dashboard(&t).await;
                            let _ = ev_tx.send(Event::DashboardData(result)).await;
                        });
                    }
                }
                Event::DashboardData(result) => {
                    state.refresh.in_flight = false;
                    match result {
                        Ok(data) => {
                            state.refresh.last_error = None;
                            state.refresh.last_success = Some(time::OffsetDateTime::now_utc());
                            state.refresh.rate_limited_until = None;
                            apply_dashboard_data(&mut state, data);
                        }
                        Err(ref e) => {
                            if let devdash::source::SourceError::RateLimited { resets_at } = e {
                                state.refresh.rate_limited_until = Some(*resets_at);
                            }
                            if matches!(state.screen, Screen::Loading) {
                                state.screen = Screen::LoadFailed {
                                    reason: e.to_string(),
                                };
                            }
                            state.refresh.last_error = Some(e.to_string());
                        }
                    }
                }
                Event::ViewerData(result) => {
                    if let Ok(viewer) = result {
                        state.viewer = Some(viewer);
                    }
                }
                Event::OrgRepoData { org, result } => {
                    if let Screen::Settings(SettingsScreen::Repositories {
                        org: ref current_org,
                        state: ref mut org_state,
                    }) = state.screen
                    {
                        if *current_org == org {
                            *org_state = match result {
                                Ok(repos) => OrgRepoState::Ready { repos },
                                Err(e) => OrgRepoState::Failed {
                                    reason: e.to_string(),
                                },
                            };
                        }
                    }
                }
                Event::Error(_) => {}
            },
            Ok(None) => break,
            Err(_) => {}
        }

        if state.refresh_requested {
            state.refresh_requested = false;
            if !state.refresh.in_flight {
                state.refresh.in_flight = true;
                let src = Arc::clone(&source);
                let t = state.tracked.clone();
                let ev_tx = tx.clone();
                tokio::spawn(async move {
                    let result = src.dashboard(&t).await;
                    let _ = ev_tx.send(Event::DashboardData(result)).await;
                });
            }
        }

        if state.org_fetch_requested.take().is_some() {
            if let Screen::Settings(SettingsScreen::Repositories { ref org, .. }) = state.screen {
                let src = Arc::clone(&source);
                let org_name = org.clone();
                let ev_tx = tx.clone();
                tokio::spawn(async move {
                    let result = src.org_repositories(&org_name).await;
                    let _ = ev_tx
                        .send(Event::OrgRepoData {
                            org: org_name,
                            result,
                        })
                        .await;
                });
            }
        }

        if state.should_quit {
            break;
        }
    }

    restore_terminal();
    Ok(())
}
