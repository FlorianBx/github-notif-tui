mod app;
mod categories;
mod events;
mod gh;
mod ui;

use app::AppState;
use chrono::Utc;
use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use crossterm::event::KeyCode;
use events::{AppEvent, spawn_event_task};
use std::io;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let result = run(&mut terminal).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
) -> Result<()> {
    let mut state = AppState::default();
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    spawn_event_task(tx.clone());
    spawn_fetch_all(tx.clone());

    loop {
        terminal.draw(|f| ui::draw(f, &state))?;

        let Some(event) = rx.recv().await else {
            break;
        };

        match event {
            AppEvent::Key(key) => {
                if events::is_quit(&key) {
                    break;
                }
                match key.code {
                    KeyCode::Tab => {
                        state.next_tab();
                        spawn_fetch_detail_if_needed(&state, tx.clone());
                    }
                    KeyCode::BackTab => {
                        state.prev_tab();
                        spawn_fetch_detail_if_needed(&state, tx.clone());
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        state.active_tab_state_mut().move_down();
                        spawn_fetch_detail_if_needed(&state, tx.clone());
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.active_tab_state_mut().move_up();
                        spawn_fetch_detail_if_needed(&state, tx.clone());
                    }
                    KeyCode::Char('r') => {
                        state.last_refresh = None;
                        spawn_fetch_all(tx.clone());
                    }
                    KeyCode::Char('o') | KeyCode::Enter => {
                        open_in_browser(&state);
                    }
                    _ => {}
                }
            }

            AppEvent::TabLoaded(idx, prs) => {
                state.tabs[idx].loading = false;
                state.tabs[idx].selected = state.tabs[idx].selected.min(prs.len().saturating_sub(1));
                state.tabs[idx].prs = prs;
                state.last_refresh = Some(Utc::now());
                if idx == state.active_tab {
                    spawn_fetch_detail_if_needed(&state, tx.clone());
                }
            }

            AppEvent::TabError(idx, err) => {
                state.tabs[idx].loading = false;
                state.error = Some(err);
            }

            AppEvent::DetailLoaded(pr_id, details) => {
                let tab = state.active_tab_state_mut();
                tab.loading_detail = false;
                tab.details_cache.insert(pr_id, details);
            }
        }
    }

    Ok(())
}

fn spawn_fetch_all(tx: mpsc::UnboundedSender<AppEvent>) {
    for (idx, fut) in [
        (0usize, "personal"),
        (1, "team"),
        (2, "mentioned"),
        (3, "assigned"),
    ] {
        let tx = tx.clone();
        let label = fut;
        tokio::spawn(async move {
            let result = match label {
                "personal" => categories::fetch_personal().await,
                "team" => categories::fetch_team().await,
                "mentioned" => categories::fetch_mentioned().await,
                _ => categories::fetch_assigned().await,
            };
            match result {
                Ok(prs) => { let _ = tx.send(AppEvent::TabLoaded(idx, prs)); }
                Err(e) => { let _ = tx.send(AppEvent::TabError(idx, e.to_string())); }
            }
        });
    }
}

fn spawn_fetch_detail_if_needed(state: &AppState, tx: mpsc::UnboundedSender<AppEvent>) {
    let tab = state.active_tab_state();
    let Some(pr) = tab.selected_pr() else { return };
    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
    if tab.details_cache.contains_key(&pr_id) || tab.loading_detail {
        return;
    }

    let repo = pr.repository.name_with_owner.clone();
    let number = pr.number;

    tokio::spawn(async move {
        if let Ok(details) = gh::fetch_pr_details(&repo, number).await {
            let _ = tx.send(AppEvent::DetailLoaded(pr_id, details));
        }
    });
}

fn open_in_browser(state: &AppState) {
    let Some(pr) = state.active_tab_state().selected_pr() else { return };
    let url = pr.url.clone();
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("open").arg(&url).output().await;
    });
}
