mod app;
mod categories;
mod events;
mod gh;
mod review;
mod score;
mod state;
mod ui;

use app::{AppState, SortDir, SortKey, SortState};
use chrono::{Duration, Utc};
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
                if state.show_help {
                    match key.code {
                        KeyCode::Char('?') | KeyCode::Esc => state.show_help = false,
                        _ => {}
                    }
                } else if state.snooze_mode {
                    handle_snooze_key(&mut state, key.code);
                } else if state.search_mode {
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
                let tab_prs = state.tabs[idx].prs.clone();
                spawn_fetch_all_details(&tab_prs, tx.clone());
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
                        tab.details_cache.insert(pr_id.clone(), details.clone());
                    }
                }
            }

            AppEvent::DetailError(pr_id) => {
                for tab in &mut state.tabs {
                    if tab.prs.iter().any(|pr| {
                        pr.repository.name_with_owner == pr_id.0 && pr.number == pr_id.1
                    }) {
                        tab.loading_detail = false;
                        tab.failed_details.insert(pr_id.clone());
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_normal_key(state: &mut AppState, code: KeyCode, tx: mpsc::UnboundedSender<AppEvent>) {
    let query = state.search_query.clone();
    let sort = state.sort.clone();
    let filter = state.filter;
    let local = state.local_state.clone();
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
            state.active_tab_state_mut().move_down(&query, &sort, filter, &local);
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.active_tab_state_mut().move_up();
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('G') => {
            state.active_tab_state_mut().go_to_last(&query, &sort, filter, &local);
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
            state.active_tab_state_mut().clear_selection();
            state.pending_g = false;
        }
        KeyCode::Char('r') => {
            state.last_refresh = None;
            state.pending_g = false;
            for tab in &mut state.tabs {
                tab.details_cache.clear();
                tab.failed_details.clear();
                tab.clear_selection();
            }
            spawn_fetch_all(tx);
        }
        KeyCode::Char('d') => {
            toggle_done(state);
            state.pending_g = false;
        }
        KeyCode::Char('z') => {
            state.snooze_mode = true;
            state.pending_g = false;
        }
        KeyCode::Char('u') => {
            toggle_read(state);
            state.pending_g = false;
        }
        KeyCode::Char('p') => {
            toggle_pin(state);
            state.pending_g = false;
        }
        KeyCode::Char('f') => {
            state.filter = state.filter.next();
            state.active_tab_state_mut().selected = 0;
            state.active_tab_state_mut().clear_selection();
            state.pending_g = false;
        }
        KeyCode::Char('F') => {
            state.filter = state.filter.prev();
            state.active_tab_state_mut().selected = 0;
            state.active_tab_state_mut().clear_selection();
            state.pending_g = false;
        }
        KeyCode::Char('v') => {
            let idx = state.active_tab_state().selected;
            state.active_tab_state_mut().toggle_selection(idx);
            state.active_tab_state_mut().move_down(&query, &sort, filter, &local);
            state.pending_g = false;
            spawn_fetch_detail_if_needed(state, tx);
        }
        KeyCode::Char('V') => {
            let count = state.active_tab_state().visible_prs(&query, &sort, filter, &local).len();
            state.active_tab_state_mut().select_all_visible(count);
            state.pending_g = false;
        }
        KeyCode::Esc => {
            if state.active_tab_state().has_selection() {
                state.active_tab_state_mut().clear_selection();
            }
            state.pending_g = false;
        }
        KeyCode::Char('o') | KeyCode::Enter => {
            if state.active_tab_state().has_selection() {
                open_selected_in_browser(state);
            } else {
                open_in_browser(state);
            }
            state.pending_g = false;
        }
        KeyCode::Char('s') => {
            let next = state.sort.key.next();
            if next == SortKey::Default {
                state.sort = SortState::default();
            } else {
                state.sort.key = next;
            }
            state.active_tab_state_mut().selected = 0;
            state.active_tab_state_mut().clear_selection();
            state.pending_g = false;
        }
        KeyCode::Char('S') => {
            state.sort.dir = if state.sort.dir == SortDir::Asc {
                SortDir::Desc
            } else {
                SortDir::Asc
            };
            state.active_tab_state_mut().selected = 0;
            state.active_tab_state_mut().clear_selection();
            state.pending_g = false;
        }
        KeyCode::Char('?') => {
            state.show_help = true;
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
    for (idx, label) in [(0usize, "personal"), (1, "team"), (2, "mentioned"), (3, "assigned"), (4, "mine")] {
        let tx = tx.clone();
        tokio::spawn(async move {
            let result = match label {
                "personal" => categories::fetch_personal().await,
                "team" => categories::fetch_team().await,
                "mentioned" => categories::fetch_mentioned().await,
                "mine" => categories::fetch_mine().await,
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

fn handle_snooze_key(state: &mut AppState, code: KeyCode) {
    let duration = match code {
        KeyCode::Char('1') => Some(Duration::hours(1)),
        KeyCode::Char('4') => Some(Duration::hours(4)),
        KeyCode::Char('t') => {
            let now = Utc::now();
            let tomorrow = (now + Duration::days(1))
                .date_naive()
                .and_hms_opt(9, 0, 0)
                .and_then(|dt| dt.and_local_timezone(chrono::Utc).single());
            tomorrow.map(|t| t - now)
        }
        KeyCode::Char('w') => {
            let now = Utc::now();
            let days_until_monday = (8 - now.format("%u").to_string().parse::<i64>().unwrap_or(1)) % 7;
            let days_until_monday = if days_until_monday == 0 { 7 } else { days_until_monday };
            let monday = (now + Duration::days(days_until_monday))
                .date_naive()
                .and_hms_opt(9, 0, 0)
                .and_then(|dt| dt.and_local_timezone(chrono::Utc).single());
            monday.map(|t| t - now)
        }
        KeyCode::Esc => {
            state.snooze_mode = false;
            return;
        }
        _ => return,
    };

    if let Some(dur) = duration {
        apply_snooze(state, Utc::now() + dur);
    }
    state.snooze_mode = false;
}

fn apply_snooze(state: &mut AppState, wake_at: chrono::DateTime<Utc>) {
    let query = state.search_query.clone();
    let sort = state.sort.clone();
    let filter = state.filter;
    let local = state.local_state.clone();
    let tab = state.active_tab_state();

    if tab.has_selection() {
        let visible = tab.visible_prs(&query, &sort, filter, &local);
        let ids: Vec<(String, u64)> = tab
            .selected_set
            .iter()
            .filter_map(|&idx| {
                visible
                    .get(idx)
                    .map(|pr| (pr.repository.name_with_owner.clone(), pr.number))
            })
            .collect();
        for id in ids {
            state.local_state.snoozed.insert(id, wake_at);
        }
        state.active_tab_state_mut().clear_selection();
    } else {
        let Some(pr) = tab.selected_pr(&query, &sort, filter, &local) else {
            return;
        };
        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        state.local_state.snoozed.insert(pr_id, wake_at);
    }

    state::save_state(&state.local_state);
}

fn toggle_pin(state: &mut AppState) {
    let query = state.search_query.clone();
    let sort = state.sort.clone();
    let filter = state.filter;
    let local = state.local_state.clone();
    let tab = state.active_tab_state();
    let Some(pr) = tab.selected_pr(&query, &sort, filter, &local) else {
        return;
    };
    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
    if !state.local_state.pinned.remove(&pr_id) {
        state.local_state.pinned.insert(pr_id);
    }
    state::save_state(&state.local_state);
}

fn toggle_read(state: &mut AppState) {
    let query = state.search_query.clone();
    let sort = state.sort.clone();
    let filter = state.filter;
    let local = state.local_state.clone();
    let tab = state.active_tab_state();
    let Some(pr) = tab.selected_pr(&query, &sort, filter, &local) else {
        return;
    };
    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
    if !state.local_state.read.remove(&pr_id) {
        state.local_state.read.insert(pr_id);
    }
    state::save_state(&state.local_state);
}

fn toggle_done(state: &mut AppState) {
    let query = state.search_query.clone();
    let sort = state.sort.clone();
    let filter = state.filter;
    let local = state.local_state.clone();
    let tab = state.active_tab_state();

    if tab.has_selection() {
        let visible = tab.visible_prs(&query, &sort, filter, &local);
        let ids: Vec<(String, u64)> = tab
            .selected_set
            .iter()
            .filter_map(|&idx| {
                visible
                    .get(idx)
                    .map(|pr| (pr.repository.name_with_owner.clone(), pr.number))
            })
            .collect();
        for id in ids {
            if !state.local_state.done.remove(&id) {
                state.local_state.done.insert(id);
            }
        }
        state.active_tab_state_mut().clear_selection();
    } else {
        let Some(pr) = tab.selected_pr(&query, &sort, filter, &local) else {
            return;
        };
        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        if !state.local_state.done.remove(&pr_id) {
            state.local_state.done.insert(pr_id);
        }
    }

    state::save_state(&state.local_state);
}

fn spawn_fetch_detail_if_needed(state: &AppState, tx: mpsc::UnboundedSender<AppEvent>) {
    let tab = state.active_tab_state();
    let query = &state.search_query;
    let Some(pr) = tab.selected_pr(query, &state.sort, state.filter, &state.local_state) else { return };
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

fn open_in_browser(state: &mut AppState) {
    let query = &state.search_query.clone();
    let local = state.local_state.clone();
    let Some(pr) = state.active_tab_state().selected_pr(query, &state.sort, state.filter, &local) else { return };
    let url = pr.url.clone();
    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
    state.local_state.read.insert(pr_id);
    state::save_state(&state.local_state);
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("open").arg(&url).output().await;
    });
}

const MAX_BULK_OPEN: usize = 10;

fn open_selected_in_browser(state: &mut AppState) {
    let tab = state.active_tab_state();
    let visible = tab.visible_prs(&state.search_query, &state.sort, state.filter, &state.local_state);
    let urls: Vec<String> = tab
        .selected_set
        .iter()
        .filter_map(|&idx| visible.get(idx).map(|pr| pr.url.clone()))
        .take(MAX_BULK_OPEN)
        .collect();
    for url in urls {
        tokio::spawn(async move {
            let _ = tokio::process::Command::new("open").arg(&url).output().await;
        });
    }
    state.active_tab_state_mut().clear_selection();
}
