use crate::app::{SortKey, SortState, TabState};
use crate::review::{analyze_reviewers, approved_count, has_active_changes};
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
                let is_selected = self.tab.selected_set.contains(&i);
                let sel_prefix = if is_selected { "▸ " } else { "  " };

                let (age, age_style) = format_age(&pr.created_at);
                let draft = if pr.is_draft { " [D]" } else { "" };
                let draft_len = draft.chars().count();

                let age_col = format!(" {}", age);
                let age_col_len = age_col.chars().count();

                let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
                let (number_style, badge, badge_style, ci_badge, ci_badge_style) =
                    if let Some(d) = self.tab.details_cache.get(&pr_id) {
                        let reviewers = analyze_reviewers(d, &pr.author.login);
                        let n_approved = approved_count(&reviewers);
                        let active = has_active_changes(&reviewers);
                        let has_pending = reviewers.iter().any(|e| {
                            e.status == crate::review::ReviewStatus::Pending
                        });
                        let fully_approved = d.review_decision.as_deref() == Some("APPROVED")
                            && !has_pending;

                        let (ci_b, ci_s) = match d.ci_status {
                            crate::gh::CiStatus::Pass => (format!("{} ", icons::CHECK), theme::ci_pass()),
                            crate::gh::CiStatus::Fail => (format!("{} ", icons::CROSS), theme::ci_fail()),
                            crate::gh::CiStatus::Pending => (format!("{} ", icons::CLOCK), theme::ci_pending()),
                            crate::gh::CiStatus::None => ("  ".to_string(), theme::dim()),
                        };

                        if active {
                            let n_changes = reviewers.iter()
                                .filter(|e| e.status == crate::review::ReviewStatus::ChangesRequested)
                                .count();
                            (theme::ci_fail(), format!("{}{} ", n_changes, icons::CROSS), theme::ci_fail(), ci_b, ci_s)
                        } else if fully_approved || n_approved >= 2 {
                            (theme::ci_pass(), format!("{}{} ", n_approved, icons::CHECK), theme::ci_pass(), ci_b, ci_s)
                        } else if n_approved > 0 {
                            (theme::ci_pending(), format!("{}{} ", n_approved, icons::CHECK), theme::ci_pending(), ci_b, ci_s)
                        } else {
                            (theme::dim(), "    ".to_string(), theme::dim(), ci_b, ci_s)
                        }
                    } else {
                        (theme::dim(), "    ".to_string(), theme::dim(), "  ".to_string(), theme::dim())
                    };

                let score_badge = if self.sort.key == SortKey::Priority {
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
                    (format!("[{:>2}] ", s), style)
                } else {
                    (String::new(), theme::dim())
                };

                let number_str = format!("#{:<5}", pr.number);
                let prefix_len = sel_prefix.chars().count()
                    + score_badge.0.chars().count()
                    + number_str.chars().count()
                    + 1
                    + badge.chars().count()
                    + ci_badge.chars().count();

                let title_width = inner_width
                    .saturating_sub(prefix_len)
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

                let sel_style = if is_selected { theme::ci_pass() } else { theme::dim() };
                let spans = vec![
                    Span::styled(sel_prefix, sel_style),
                    Span::styled(score_badge.0, score_badge.1),
                    Span::styled(number_str, number_style),
                    Span::raw(" "),
                    Span::styled(badge, badge_style),
                    Span::styled(ci_badge, ci_badge_style),
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
