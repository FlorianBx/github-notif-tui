use crate::app::TabState;
use crate::ui::theme;
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

fn format_age(dt: &chrono::DateTime<chrono::Utc>) -> (String, ratatui::style::Style) {
    let secs = (Utc::now() - *dt).num_seconds().unsigned_abs();
    let duration = std::time::Duration::from_secs(secs);
    let s = humantime::format_duration(duration).to_string();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let short = parts.first().map(|s| *s).unwrap_or("?").to_string();

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
}

impl StatefulWidget for PrList<'_> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ListState) {
        let items: Vec<ListItem> = self
            .tab
            .prs
            .iter()
            .enumerate()
            .map(|(i, pr)| {
                let (age, age_style) = format_age(&pr.created_at);
                let (last_act, _) = format_age(&pr.updated_at);

                let draft = if pr.is_draft { " [D]" } else { "" };
                let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
                let details = self.tab.details_cache.get(&pr_id);
                let decision = details
                    .and_then(|d| d.review_decision.as_deref())
                    .map(|s| match s {
                        "APPROVED" => "✓",
                        "CHANGES_REQUESTED" => "✗",
                        "REVIEW_REQUIRED" => "⧗",
                        _ => "·",
                    })
                    .unwrap_or("·");

                let row_style = if i == self.tab.selected {
                    theme::selected_row()
                } else {
                    theme::normal_row()
                };

                let repo_short = pr
                    .repository
                    .name_with_owner
                    .split('/')
                    .last()
                    .unwrap_or(&pr.repository.name_with_owner);

                let diff_span = if let Some(d) = details {
                    vec![
                        Span::styled(format!("+{:<4}", d.additions), theme::additions()),
                        Span::styled(format!("-{:<4}", d.deletions), theme::deletions()),
                    ]
                } else {
                    vec![Span::styled("         ", theme::dim())]
                };

                let mut spans = vec![
                    Span::styled(format!("#{:<5} ", pr.number), theme::dim()),
                    Span::styled(
                        format!("{}{:<38} ", pr.title.chars().take(38).collect::<String>(), draft),
                        row_style,
                    ),
                    Span::styled(format!("{:<8}", age), age_style),
                    Span::styled(format!("act:{:<7}", last_act), theme::dim()),
                    Span::styled(format!(" {} ", decision), row_style),
                ];
                spans.extend(diff_span);
                spans.push(Span::styled(format!(" {}", repo_short), theme::dim()));

                let line = Line::from(spans);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(self.title))
            .highlight_style(theme::selected_row());

        StatefulWidget::render(list, area, buf, state);
    }
}
