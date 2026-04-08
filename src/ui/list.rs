use crate::app::{SortState, TabState};
use crate::ui::{icons, theme};
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
    pub sort: &'a SortState,
}

impl StatefulWidget for PrList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let inner_width = area.width.saturating_sub(2) as usize;

        let visible = self.tab.visible_prs(self.query, self.sort);
        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(i, pr)| {
                let (age, age_style) = format_age(&pr.created_at);
                let draft = if pr.is_draft { " [D]" } else { "" };
                let draft_len = draft.chars().count();

                let age_col = format!(" {}", age);
                let age_col_len = age_col.chars().count();

                let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
                let (number_style, badge, badge_style) =
                    if let Some(d) = self.tab.details_cache.get(&pr_id) {
                        let approved = d.reviews.iter()
                            .filter(|r| r.state == "APPROVED")
                            .map(|r| &r.author.login)
                            .collect::<std::collections::HashSet<_>>()
                            .len();
                        let pending = d.requested_reviewers.len();
                        let total = approved + pending;

                        let has_pending_after_changes = d.review_decision.as_deref()
                            == Some("CHANGES_REQUESTED")
                            && pending > 0;

                        if has_pending_after_changes || (d.review_decision.is_none() && total > 0) || (d.review_decision.as_deref() == Some("REVIEW_REQUIRED")) {
                            (
                                theme::ci_pending(),
                                format!("{}{} ", approved, icons::CHECK),
                                theme::ci_pending(),
                            )
                        } else {
                            match d.review_decision.as_deref() {
                                Some("APPROVED") => (
                                    theme::ci_pass(),
                                    format!("{}{} ", approved, icons::CHECK),
                                    theme::ci_pass(),
                                ),
                                Some("CHANGES_REQUESTED") => {
                                    let changes = d.reviews.iter()
                                        .filter(|r| r.state == "CHANGES_REQUESTED")
                                        .map(|r| &r.author.login)
                                        .collect::<std::collections::HashSet<_>>()
                                        .len();
                                    (
                                        theme::ci_fail(),
                                        format!("{}{} ", changes, icons::CROSS),
                                        theme::ci_fail(),
                                    )
                                },
                                _ if approved > 0 => (
                                    theme::ci_pending(),
                                    format!("{}{} ", approved, icons::CHECK),
                                    theme::ci_pending(),
                                ),
                                _ => (theme::dim(), "    ".to_string(), theme::dim()),
                            }
                        }
                    } else {
                        (theme::dim(), "    ".to_string(), theme::dim())
                    };

                let number_str = format!("#{:<5}", pr.number);
                let number_len = number_str.chars().count() + badge.chars().count() + 1;

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
                    Span::styled(number_str, number_style),
                    Span::raw(" "),
                    Span::styled(badge, badge_style),
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
