use crate::app::{FilterPreset, PrStatus, SortKey, SortState, TabState};
use crate::state::LocalState;
use crate::ui::theme;
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

const AUTHOR_COL: usize = 10;
const AGE_COL: usize = 6;

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

fn status_dot(status: PrStatus) -> (&'static str, Style) {
    match status {
        PrStatus::Ready => ("● ", theme::ci_pass()),
        PrStatus::InProgress => ("● ", theme::ci_pending()),
        PrStatus::NeedsWork => ("● ", theme::ci_fail()),
        PrStatus::Draft => ("○ ", theme::dim()),
    }
}

fn truncate_pad(s: &str, width: usize) -> String {
    let truncated: String = s.chars().take(width).collect();
    let pad = width.saturating_sub(truncated.chars().count());
    format!("{}{:pad$}", truncated, "", pad = pad)
}

pub struct PrList<'a> {
    pub tab: &'a TabState,
    pub title: &'a str,
    pub query: &'a str,
    pub sort: &'a SortState,
    pub filter: FilterPreset,
    pub local: &'a LocalState,
}

impl StatefulWidget for PrList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let inner_width = area.width.saturating_sub(2) as usize;
        let has_selection = self.tab.has_selection();

        let visible = self.tab.visible_prs(self.query, self.sort, self.filter, self.local);
        let is_muted_filter = self.filter == FilterPreset::Done || self.filter == FilterPreset::Snoozed;
        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(i, pr)| {
                let status = self.tab.pr_status(pr);
                let (dot, dot_style) = status_dot(status);
                let (age, age_style) = format_age(&pr.created_at);

                let sel_prefix = if has_selection {
                    if self.tab.selected_set.contains(&i) {
                        ("▸ ", theme::ci_pass())
                    } else {
                        ("  ", theme::dim())
                    }
                } else {
                    ("", theme::dim())
                };

                let score_col = if self.sort.key == SortKey::Priority {
                    let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
                    let details = self.tab.details_cache.get(&pr_id);
                    let s = crate::score::compute_priority(pr, details);
                    let style = if s > 70 {
                        theme::ci_pass()
                    } else if s > 40 {
                        theme::ci_pending()
                    } else {
                        theme::ci_fail()
                    };
                    (format!("{:>2} ", s), style)
                } else {
                    (String::new(), theme::dim())
                };

                let author_str = truncate_pad(&pr.author.login, AUTHOR_COL);
                let age_str = format!("{:>width$}", age, width = AGE_COL);

                let fixed_width = sel_prefix.0.chars().count()
                    + score_col.0.chars().count()
                    + 1 // pin column
                    + dot.chars().count()
                    + 1
                    + AUTHOR_COL
                    + 1
                    + AGE_COL;

                let title_width = inner_width.saturating_sub(fixed_width);
                let title_padded = truncate_pad(&pr.title, title_width);

                let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
                let is_pinned = self.local.pinned.contains(&pr_id);
                let is_unread = !self.local.read.contains(&pr_id);
                let row_style = if is_muted_filter {
                    theme::dim()
                } else if i == self.tab.selected {
                    theme::selected_row()
                } else if is_unread {
                    theme::unread_row()
                } else {
                    theme::normal_row()
                };

                let pin_col = if is_pinned { ("▪", theme::header()) } else { (" ", theme::dim()) };

                let mut spans = Vec::with_capacity(10);
                if !sel_prefix.0.is_empty() {
                    spans.push(Span::styled(sel_prefix.0, sel_prefix.1));
                }
                if !score_col.0.is_empty() {
                    spans.push(Span::styled(score_col.0, score_col.1));
                }
                spans.push(Span::styled(pin_col.0, pin_col.1));
                spans.push(Span::styled(dot, dot_style));
                spans.push(Span::styled(title_padded, row_style));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(author_str, theme::dim()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(age_str, age_style));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(self.title))
            .highlight_style(theme::selected_row());

        StatefulWidget::render(list, area, buf, state);
    }
}
