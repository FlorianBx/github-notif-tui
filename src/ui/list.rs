use crate::app::TabState;
use crate::ui::theme;
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

fn format_age(dt: &chrono::DateTime<chrono::Utc>) -> (String, Style) {
    let secs = (Utc::now() - *dt).num_seconds().unsigned_abs();
    let duration = std::time::Duration::from_secs(secs);
    let short = humantime::format_duration(duration)
        .to_string()
        .split_whitespace()
        .next()
        .unwrap_or("?")
        .to_string();

    let style = if secs > 7 * 24 * 3600 {
        theme::age_old()
    } else if secs > 2 * 24 * 3600 {
        theme::age_medium()
    } else {
        theme::age_fresh()
    };
    (short, style)
}

pub struct PrList<'a> {
    pub tab: &'a TabState,
    pub title: &'a str,
    pub query: &'a str,
}

impl StatefulWidget for PrList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let inner_width = area.width.saturating_sub(2) as usize;

        let visible = self.tab.visible_prs(self.query);
        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(i, pr)| {
                let (age, age_style) = format_age(&pr.created_at);
                let draft = if pr.is_draft { " [D]" } else { "" };
                let draft_len = draft.chars().count();

                let age_col = format!(" {}", age);
                let age_col_len = age_col.chars().count();

                let number_str = format!("#{:<5} ", pr.number);
                let number_len = number_str.chars().count();

                let title_width = inner_width
                    .saturating_sub(number_len)
                    .saturating_sub(draft_len)
                    .saturating_sub(age_col_len);

                let title_truncated: String = pr.title.chars().take(title_width).collect();
                let pad = title_width.saturating_sub(title_truncated.chars().count());
                let title_padded = format!("{}{:pad$}", title_truncated, "", pad = pad);

                let row_style = if i == self.tab.selected {
                    theme::selected_row()
                } else {
                    theme::normal_row()
                };

                let spans = vec![
                    Span::styled(number_str, theme::dim()),
                    Span::styled(title_padded, row_style),
                    Span::styled(draft.to_string(), theme::dim()),
                    Span::styled(age_col, age_style),
                ];

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(self.title))
            .highlight_style(theme::selected_row());

        StatefulWidget::render(list, area, buf, state);
    }
}
