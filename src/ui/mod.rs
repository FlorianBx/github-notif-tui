pub mod detail;
pub mod help;
pub mod icons;
pub mod list;
pub mod tabs;
pub mod theme;

use crate::app::{AppState, FilterPreset, SortDir, SortKey};
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
    let has_search = state.search_mode || !state.search_query.is_empty();
    let has_snooze = state.snooze_mode;
    let mut constraints = vec![
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(0),
    ];
    if has_search || has_snooze {
        constraints.push(Constraint::Length(1));
    }
    constraints.push(Constraint::Length(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    TabsBar { state }.render(chunks[0], buf);
    render_filter_bar(chunks[1], buf, state);
    render_body(chunks[2], buf, state);

    let mut footer_idx = 3;
    if has_snooze {
        render_snooze_bar(chunks[3], buf);
        footer_idx = 4;
    } else if has_search {
        render_search_bar(chunks[3], buf, state);
        footer_idx = 4;
    }
    render_footer(chunks[footer_idx], buf, state);

    if state.show_help {
        HelpOverlay.render(area, buf);
    }
}

fn render_filter_bar(area: Rect, buf: &mut Buffer, state: &AppState) {
    let presets = [
        FilterPreset::All,
        FilterPreset::Ready,
        FilterPreset::NeedsReview,
        FilterPreset::NeedsWork,
        FilterPreset::Draft,
        FilterPreset::Done,
        FilterPreset::Snoozed,
    ];
    let dot = |p: FilterPreset| -> (&str, ratatui::style::Style) {
        match p {
            FilterPreset::All => ("", theme::dim()),
            FilterPreset::Ready => ("● ", theme::ci_pass()),
            FilterPreset::NeedsReview => ("● ", theme::ci_pending()),
            FilterPreset::NeedsWork => ("● ", theme::ci_fail()),
            FilterPreset::Draft => ("○ ", theme::dim()),
            FilterPreset::Done => ("✓ ", theme::dim()),
            FilterPreset::Snoozed => ("◷ ", theme::ci_pending()),
        }
    };
    let mut spans: Vec<Span> = vec![Span::styled(" f ", theme::dim())];
    for p in presets {
        let active = p == state.filter;
        let (d, ds) = dot(p);
        let label_style = if active { theme::tab_active() } else { theme::dim() };
        if !d.is_empty() {
            spans.push(Span::styled(d, ds));
        }
        spans.push(Span::styled(p.label(), label_style));
        spans.push(Span::styled("  ", theme::dim()));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
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

    let local = &state.local_state;
    let visible_count = tab
        .visible_prs(&state.search_query, &state.sort, state.filter, local)
        .len();
    let sel_count = tab.selected_set.len();
    let total = tab.prs.len();
    let title = if tab.loading {
        format!(" {} — loading… ", tab_enum.label())
    } else if sel_count > 0 {
        format!(" {} ({} selected) ", tab_enum.label(), sel_count)
    } else if state.filter != FilterPreset::All || !state.search_query.is_empty() {
        format!(" {} ({}/{}) ", tab_enum.label(), visible_count, total)
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
        filter: state.filter,
        local,
    }
    .render(split[0], buf, &mut list_state);

    DetailPanel {
        tab,
        query: &state.search_query,
        sort: &state.sort,
        filter: state.filter,
        local,
    }
    .render(split[1], buf);
}

fn render_snooze_bar(area: Rect, buf: &mut Buffer) {
    let spans = vec![
        Span::styled(" Snooze: ", theme::header()),
        Span::styled("1", theme::tab_active()),
        Span::styled("=1h  ", theme::dim()),
        Span::styled("4", theme::tab_active()),
        Span::styled("=4h  ", theme::dim()),
        Span::styled("t", theme::tab_active()),
        Span::styled("=tomorrow  ", theme::dim()),
        Span::styled("w", theme::tab_active()),
        Span::styled("=next week  ", theme::dim()),
        Span::styled("Esc", theme::tab_active()),
        Span::styled("=cancel", theme::dim()),
    ];
    Paragraph::new(Line::from(spans)).render(area, buf);
}

fn render_search_bar(area: Rect, buf: &mut Buffer, state: &AppState) {
    let cursor = if state.search_mode { "█" } else { "" };
    let text = format!("/{}{}", state.search_query, cursor);
    Paragraph::new(Line::from(vec![Span::styled(text, theme::header())]))
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
        let arrow = if state.sort.dir == SortDir::Asc {
            "↑"
        } else {
            "↓"
        };
        format!("  · sort: {} {}", state.sort.key.label(), arrow)
    } else {
        String::new()
    };

    let sel_count = state.active_tab_state().selected_set.len();
    let text = if state.snooze_mode {
        " 1=1h  4=4h  t=tomorrow  w=next week  Esc cancel".to_string()
    } else if state.search_mode {
        " Esc cancel  Enter confirm".to_string()
    } else if sel_count > 0 {
        format!(
            " v toggle  V all  Esc clear  o open ({} selected)  ? help{}",
            sel_count, refresh_info
        )
    } else {
        format!(
            " q quit  o open  d done  z snooze  / search  f filter  s sort{}  ? help{}",
            sort_info, refresh_info
        )
    };

    Paragraph::new(Line::from(Span::styled(text, theme::dim()))).render(area, buf);
}
