use crate::app::{SortState, TabState};
use crate::gh::PrDetails;
use crate::ui::{icons, theme};
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

fn rel_age(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let secs = (Utc::now() - *dt).num_seconds().unsigned_abs();
    let duration = std::time::Duration::from_secs(secs);
    humantime::format_duration(duration).to_string()
}

pub struct DetailPanel<'a> {
    pub tab: &'a TabState,
    pub query: &'a str,
    pub sort: &'a SortState,
}

impl Widget for DetailPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title("Detail");

        let Some(pr) = self.tab.selected_pr(self.query, self.sort) else {
            Paragraph::new("No PR selected")
                .block(block)
                .render(area, buf);
            return;
        };

        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        let details = self.tab.details_cache.get(&pr_id);

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Title:   ", theme::header()),
                Span::raw(pr.title.clone()),
            ]),
            Line::from(vec![
                Span::styled("Author:  ", theme::header()),
                Span::raw(pr.author.login.clone()),
            ]),
            Line::from(vec![
                Span::styled("Repo:    ", theme::header()),
                Span::raw(pr.repository.name_with_owner.clone()),
            ]),
            Line::from(vec![
                Span::styled("PR:      ", theme::header()),
                Span::styled(format!("#{}", pr.number), theme::dim()),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Age:     ", theme::header()),
                Span::raw(rel_age(&pr.created_at)),
            ]),
            Line::from(vec![
                Span::styled("Activity:", theme::header()),
                Span::raw(format!(" {} ago", rel_age(&pr.updated_at))),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Comments:", theme::header()),
                Span::raw(pr.comments_count.to_string()),
            ]),
        ];

        if let Some(d) = details {
            lines.push(Line::from(vec![
                Span::styled("Diff:    ", theme::header()),
                Span::styled(format!("+{}", d.additions), theme::additions()),
                Span::raw(" / "),
                Span::styled(format!("-{}", d.deletions), theme::deletions()),
            ]));

            lines.push(Line::raw(""));

            let re_requested: std::collections::HashSet<_> =
                d.requested_reviewers.iter().collect();
            let active_changes = d.reviews.iter()
                .filter(|r| r.state == "CHANGES_REQUESTED")
                .any(|r| !re_requested.contains(&r.author.login));

            let decision_label = match d.review_decision.as_deref() {
                Some("APPROVED") => Span::styled(format!("{} Approved", icons::CHECK), theme::ci_pass()),
                Some("CHANGES_REQUESTED") if !active_changes => {
                    Span::styled(format!("{} Re-review requested", icons::CLOCK), theme::ci_pending())
                }
                Some("CHANGES_REQUESTED") => {
                    Span::styled(format!("{} Changes requested", icons::CROSS), theme::ci_fail())
                }
                Some("REVIEW_REQUIRED") => Span::styled(format!("{} Review required", icons::CLOCK), theme::ci_pending()),
                Some(other) => Span::styled(other.to_string(), theme::dim()),
                None => Span::styled(icons::DASH, theme::dim()),
            };
            lines.push(Line::from(vec![
                Span::styled("Decision:", theme::header()),
                Span::raw(" "),
                decision_label,
            ]));

            if !d.requested_reviewers.is_empty() {
                lines.push(Line::raw(""));
                lines.push(Line::from(Span::styled("Pending: ", theme::header())));
                for login in &d.requested_reviewers {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", icons::CLOCK), theme::ci_pending()),
                        Span::raw(login.clone()),
                    ]));
                }
            }

            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("Reviews:", theme::header())));
            render_reviews_lines(d, &mut lines);
        } else if self.tab.loading_detail {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("Loading…", theme::dim())));
        }

        if pr.is_draft {
            lines.push(Line::from(Span::styled(icons::DRAFT, theme::ci_pending())));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .render(area, buf);
    }
}

fn render_reviews_lines(details: &PrDetails, lines: &mut Vec<Line>) {
    if details.reviews.is_empty() {
        lines.push(Line::from(Span::styled("  no reviews yet", theme::dim())));
        return;
    }

    let re_requested: std::collections::HashSet<_> =
        details.requested_reviewers.iter().collect();

    let mut last_by_author: std::collections::HashMap<&str, &crate::gh::Review> =
        std::collections::HashMap::new();
    for r in &details.reviews {
        last_by_author.insert(r.author.login.as_str(), r);
    }

    let mut entries: Vec<_> = last_by_author.values().collect();
    entries.sort_by_key(|r| r.submitted_at);

    for review in entries {
        let is_re_requested = re_requested.contains(&review.author.login);
        let (symbol, style) = if is_re_requested && review.state == "CHANGES_REQUESTED" {
            (icons::CLOCK, theme::ci_pending())
        } else {
            match review.state.as_str() {
                "APPROVED" => (icons::CHECK, theme::ci_pass()),
                "CHANGES_REQUESTED" => (icons::CROSS, theme::ci_fail()),
                "COMMENTED" => (icons::COMMENT, theme::dim()),
                "DISMISSED" => (icons::SLASH, theme::dim()),
                _ => (icons::DOT, theme::dim()),
            }
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", symbol), style),
            Span::raw(review.author.login.clone()),
        ]));
    }
}
