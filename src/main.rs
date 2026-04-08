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
use crossterm::event::{KeyCode, KeyModifiers};
use events::{AppEvent, spawn_event_task};
use std::io;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

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

        let Some(event) = rx.recv().await else { break };

        match event {
            AppEvent::Key(key) => {
                if state.search_mode {
                    handle_search_key(&mut state, key.code, key.modifiers, tx.clone());
                } else {
                    if events::is_quit(&key) {
                        break;
                    }
                    handle_normal_key(&mut state, key.code, tx.clone());
                }
            }

            AppEvent::TabLoaded(idx, prs) => {
                state.tabs[idx].loading = false;
                state.tabs[idx].selected = state.tabs[idx]
                    .selected
                    .min(prs.len().saturating_sub(1));
                state.tabs[idx].prs = prs;
                state.last_refresh = Some(Utc::now());
                spawn_fetch_all_details(&state.tabs[idx].prs, tx.clone());
            }

            AppEvent::TabError(idx, err) => {
                state.tabs[idx].loading = false;
                state.error = Some(err);
            }

            AppEvent::DetailLoaded(pr_id, details) => {
                for tab in &mut state.tabs {
                    if tab.prs.iter().any(|pr| {
                        pr.repository.name_with_owner == pr_id.0 && pr.number == pr_id.1
                    }) {
                        tab.loading_detail = false;
                        tab.failed_details.remove(&pr_id);
                        tab.details_cache.insert(pr_id, details);
                        break;
                    }
                }
            }

            AppEvent::DetailError(pr_id) => {
                for tab in &mut state.tabs {
                    if tab.prs.iter().any(|pr| {
                        pr.repository.name_with_owner == pr_id.0 && pr.number == pr_id.1
                    }) {
                        tab.loading_detail = false;
                        tab.failed_details.insert(pr_id);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_normal_key(state: &mut AppState, code: KeyCode, tx: mpsc::UnboundedSender<AppEvent>) {
    let query = state.search_query.clone();
    match code {
        KeyCode::Tab | KeyCode::Char('l') => {
            state.next_tab();
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::BackTab | KeyCode::Char('h') => {
            state.prev_tab();
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.active_tab_state_mut().move_down(&query);
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.active_tab_state_mut().move_up(&query);
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('G') => {
            state.active_tab_state_mut().go_to_last(&query);
            state.pending_g = false;
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('g') => {
            if state.pending_g {
                state.active_tab_state_mut().go_to_first();
                state.pending_g = false;
                spawn_fetch_detail_if_needed(state, tx);
            } else {
                state.pending_g = true;
            }
        }
        KeyCode::Char('/') => {
            state.search_mode = true;
            state.search_query.clear();
            state.active_tab_state_mut().selected = 0;
            state.pending_g = false;
        }
        KeyCode::Char('r') => {
            state.last_refresh = None;
            state.pending_g = false;
            for tab in &mut state.tabs {
                tab.details_cache.clear();
                tab.failed_details.clear();
            }
            spawn_fetch_all(tx);
        }
        KeyCode::Char('o') | KeyCode::Enter => {
            open_in_browser(state);
            state.pending_g = false;
        }
        _ => {
            state.pending_g = false;
        }
    }
}

fn handle_search_key(
    state: &mut AppState,
    code: KeyCode,
    _modifiers: KeyModifiers,
    tx: mpsc::UnboundedSender<AppEvent>,
) {
    match code {
        KeyCode::Esc => {
            state.reset_search();
        }
        KeyCode::Enter => {
            state.search_mode = false;
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            state.active_tab_state_mut().selected = 0;
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            state.active_tab_state_mut().selected = 0;
        }
        _ => {}
    }
}

fn spawn_fetch_all(tx: mpsc::UnboundedSender<AppEvent>) {
    for (idx, label) in [(0usize, "personal"), (1, "team"), (2, "mentioned"), (3, "assigned")] {
        let tx = tx.clone();
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

fn spawn_fetch_all_details(prs: &[crate::gh::PullRequest], tx: mpsc::UnboundedSender<AppEvent>) {
    let sem = Arc::new(Semaphore::new(25));
    for pr in prs {
        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        let repo = pr.repository.name_with_owner.clone();
        let number = pr.number;
        let tx = tx.clone();
        let sem = sem.clone();
        tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.unwrap();
            match gh::fetch_pr_details(&repo, number).await {
                Ok(details) => { let _ = tx.send(AppEvent::DetailLoaded(pr_id, details)); }
                Err(_) => { let _ = tx.send(AppEvent::DetailError(pr_id)); }
            }
        });
    }
}

fn spawn_fetch_detail_if_needed(state: &AppState, tx: mpsc::UnboundedSender<AppEvent>) {
    let tab = state.active_tab_state();
    let query = &state.search_query;
    let Some(pr) = tab.selected_pr(query) else { return };
    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
    if tab.details_cache.contains_key(&pr_id) || tab.failed_details.contains(&pr_id) {
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
    let query = &state.search_query;
    let Some(pr) = state.active_tab_state().selected_pr(query) else { return };
    let url = pr.url.clone();
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("open").arg(&url).output().await;
    });
}
