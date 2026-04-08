pub mod detail;
pub mod list;
pub mod tabs;
pub mod theme;

use crate::app::AppState;
use crate::ui::{detail::DetailPanel, list::PrList, tabs::TabsBar};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{ListState, Paragraph, StatefulWidget, Widget},
    Frame,
};

pub fn draw(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let buf = frame.buffer_mut();
    render_app(area, buf, state);
}

fn render_app(area: Rect, buf: &mut Buffer, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    TabsBar { state }.render(chunks[0], buf);
    render_body(chunks[1], buf, state);
    render_footer(chunks[2], buf, state);
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

    let title = if tab.loading {
        format!(" {} — loading… ", tab_enum.label())
    } else {
        format!(" {} ", tab_enum.label())
    };

    let mut list_state = ListState::default();
    list_state.select(if tab.prs.is_empty() {
        None
    } else {
        Some(tab.selected)
    });

    PrList {
        tab,
        title: Box::leak(title.into_boxed_str()),
    }
    .render(split[0], buf, &mut list_state);

    DetailPanel { tab }.render(split[1], buf);
}

fn render_footer(area: Rect, buf: &mut Buffer, state: &AppState) {
    let refresh_info = state
        .last_refresh
        .map(|t| {
            let secs = (chrono::Utc::now() - t).num_seconds();
            format!("  ⟳ {}s ago", secs)
        })
        .unwrap_or_default();

    let text = format!(
        " q quit  r refresh  o open  Tab/S-Tab switch  j/k navigate{}",
        refresh_info
    );

    Paragraph::new(Line::from(Span::styled(text, theme::dim()))).render(area, buf);
}
