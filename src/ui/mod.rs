pub mod detail;
pub mod help;
pub mod icons;
pub mod list;
pub mod tabs;
pub mod theme;

use crate::app::{AppState, SortDir, SortKey};
use crate::ui::{detail::DetailPanel, help::HelpOverlay, list::PrList, tabs::TabsBar};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, ListState, Paragraph, StatefulWidget, Widget},
    Frame,
};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let buf = frame.buffer_mut();
    render_app(area, buf, state);
}

fn render_app(area: Rect, buf: &mut Buffer, state: &AppState) {
    let constraints = if state.search_mode || !state.search_query.is_empty() {
        vec![
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else {
        vec![
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    TabsBar { state }.render(chunks[0], buf);
    render_body(chunks[1], buf, state);

    if state.search_mode || !state.search_query.is_empty() {
        render_search_bar(chunks[2], buf, state);
        render_footer(chunks[3], buf, state);
    } else {
        render_footer(chunks[2], buf, state);
    }

    if state.show_help {
        HelpOverlay.render(area, buf);
    }
}

fn render_body(area: Rect, buf: &mut Buffer, state: &AppState) {
    if let Some(err) = &state.error {
        Paragraph::new(Line::from(vec![
            Span::styled(" ERROR: ", theme::error()),
            Span::raw(err.clone()),
        ]))
        .render(area, buf);
        return;
    }

    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let tab = state.active_tab_state();
    let tab_enum = crate::app::Tab::from(state.active_tab);

    let visible_count = tab.visible_prs(&state.search_query, &state.sort).len();
    let sel_count = tab.selected_set.len();
    let title = if tab.loading {
        format!(" {} — loading… ", tab_enum.label())
    } else if sel_count > 0 {
        format!(" {} ({} selected) ", tab_enum.label(), sel_count)
    } else if !state.search_query.is_empty() {
        format!(" {} ({}/{}) ", tab_enum.label(), visible_count, tab.prs.len())
    } else {
        format!(" {} ", tab_enum.label())
    };

    let mut list_state = ListState::default();
    list_state.select(if visible_count == 0 {
        None
    } else {
        Some(tab.selected)
    });

    PrList {
        tab,
        title: Box::leak(title.into_boxed_str()),
        query: &state.search_query,
        sort: &state.sort,
    }
    .render(split[0], buf, &mut list_state);

    DetailPanel {
        tab,
        query: &state.search_query,
        sort: &state.sort,
    }
    .render(split[1], buf);
}

fn render_search_bar(area: Rect, buf: &mut Buffer, state: &AppState) {
    let cursor = if state.search_mode { "█" } else { "" };
    let text = format!("/{}{}", state.search_query, cursor);
    Paragraph::new(Line::from(vec![
        Span::styled(text, theme::header()),
    ]))
    .block(Block::default().borders(Borders::NONE))
    .render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer, state: &AppState) {
    let refresh_info = state
        .last_refresh
        .map(|t| {
            let secs = (chrono::Utc::now() - t).num_seconds();
            format!("  ⟳ {}s ago", secs)
        })
        .unwrap_or_default();

    let sort_info = if state.sort.key != SortKey::Default {
        let arrow = if state.sort.dir == SortDir::Asc { "↑" } else { "↓" };
        format!("  · sort: {} {}", state.sort.key.label(), arrow)
    } else {
        String::new()
    };

    let sel_count = state.active_tab_state().selected_set.len();
    let text = if state.search_mode {
        " Esc cancel  Enter confirm".to_string()
    } else if sel_count > 0 {
        format!(
            " v toggle  V all  Esc clear  o open ({} selected)  ? help{}",
            sel_count, refresh_info
        )
    } else {
        format!(
            " q quit  o open  / search  s sort{}  ? help{}",
            sort_info, refresh_info
        )
    };

    Paragraph::new(Line::from(Span::styled(text, theme::dim()))).render(area, buf);
}
