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

            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("Reviewers:", theme::header())));
            render_reviewers_lines(d, &pr.author.login, &mut lines);
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

fn render_reviewers_lines(details: &PrDetails, pr_author: &str, lines: &mut Vec<Line>) {
    let mut effective_state: std::collections::HashMap<String, &str> =
        std::collections::HashMap::new();

    for r in &details.reviews {
        if r.author.login == pr_author {
            continue;
        }
        match r.state.as_str() {
            "APPROVED" | "CHANGES_REQUESTED" | "DISMISSED" => {
                effective_state.insert(r.author.login.clone(), r.state.as_str());
            }
            _ => {
                effective_state.entry(r.author.login.clone()).or_insert(r.state.as_str());
            }
        }
    }

    for login in &details.requested_reviewers {
        if login != pr_author {
            effective_state.entry(login.clone()).or_insert("PENDING");
        }
    }

    if effective_state.is_empty() {
        lines.push(Line::from(Span::styled("  no reviewers", theme::dim())));
        return;
    }

    let mut entries: Vec<_> = effective_state.iter().collect();
    entries.sort_by_key(|(login, _)| login.as_str());

    for (login, state) in entries {
        let (symbol, style) = if *state == "APPROVED" {
            (icons::CHECK, theme::ci_pass())
        } else {
            (icons::COMMENT, theme::dim())
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", symbol), style),
            Span::raw(login.clone()),
        ]));
    }
}
