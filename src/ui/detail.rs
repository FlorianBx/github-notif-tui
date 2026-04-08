use crate::app::TabState;
use crate::gh::Review;
use crate::ui::theme;
use chrono::Utc;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

fn rel_age(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let secs = (Utc::now() - *dt).num_seconds().unsigned_abs();
    let duration = std::time::Duration::from_secs(secs);
    humantime::format_duration(duration)
        .to_string()
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn bar(filled: usize, total: usize, width: usize) -> String {
    let n = if total == 0 { 0 } else { (filled * width / total).min(width) };
    format!("{}{}", "█".repeat(n), "░".repeat(width - n))
}

fn age_bar(secs: u64) -> (String, Style) {
    let (filled, style) = if secs < 86_400 {
        (2, theme::age_fresh())
    } else if secs < 3 * 86_400 {
        (5, theme::age_fresh())
    } else if secs < 7 * 86_400 {
        (9, theme::age_medium())
    } else if secs < 14 * 86_400 {
        (13, theme::age_old())
    } else {
        (16, theme::age_old())
    };
    (bar(filled, 16, 16), style)
}

fn size_bar(total_lines: u32) -> (String, Style) {
    let (filled, style) = if total_lines < 50 {
        (2, theme::age_fresh())
    } else if total_lines < 200 {
        (5, theme::age_fresh())
    } else if total_lines < 500 {
        (9, theme::age_medium())
    } else if total_lines < 1000 {
        (13, theme::age_old())
    } else {
        (16, theme::age_old())
    };
    (bar(filled, 16, 16), style)
}

fn decision_spans(decision: Option<&str>) -> Vec<Span<'static>> {
    match decision {
        Some("APPROVED") => vec![Span::styled("✓ APPROVED", theme::ci_pass())],
        Some("CHANGES_REQUESTED") => vec![Span::styled("✗ CHANGES REQUESTED", theme::ci_fail())],
        _ => vec![Span::styled("⧗ REVIEW REQUIRED", theme::ci_pending())],
    }
}

fn mergeable_spans(state: Option<&str>) -> Vec<Span<'static>> {
    match state {
        Some("clean") => vec![
            Span::styled("  ·  ", theme::dim()),
            Span::styled("✓ ready to merge", theme::ci_pass()),
        ],
        Some("dirty") => vec![
            Span::styled("  ·  ", theme::dim()),
            Span::styled("✗ conflicts", theme::ci_fail()),
        ],
        Some("blocked") => vec![
            Span::styled("  ·  ", theme::dim()),
            Span::styled("⊘ blocked", theme::ci_pending()),
        ],
        Some("unstable") => vec![
            Span::styled("  ·  ", theme::dim()),
            Span::styled("⚠ CI failing", theme::ci_fail()),
        ],
        Some("draft") => vec![
            Span::styled("  ·  ", theme::dim()),
            Span::styled("[draft]", theme::dim()),
        ],
        _ => vec![],
    }
}

fn last_review_for<'a>(login: &str, reviews: &'a [Review]) -> Option<&'a Review> {
    reviews
        .iter()
        .filter(|r| r.author.login == login)
        .filter(|r| r.state != "COMMENTED" && r.state != "DISMISSED")
        .last()
}

pub struct DetailPanel<'a> {
    pub tab: &'a TabState,
    pub query: &'a str,
}

impl Widget for DetailPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title("Detail");

        let Some(pr) = self.tab.selected_pr(self.query) else {
            Paragraph::new("No PR selected").block(block).render(area, buf);
            return;
        };

        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        let details = self.tab.details_cache.get(&pr_id);

        let age_secs = (Utc::now() - pr.created_at).num_seconds().unsigned_abs();

        let mut lines: Vec<Line> = vec![
            Line::from(Span::styled(pr.title.clone(), theme::header())),
            Line::from(vec![
                Span::styled("by ", theme::dim()),
                Span::raw(pr.author.login.clone()),
                Span::styled("  ·  ", theme::dim()),
                Span::raw(pr.repository.name_with_owner.clone()),
                Span::styled("  ·  ", theme::dim()),
                Span::styled(format!("#{}", pr.number), theme::dim()),
            ]),
            Line::raw(""),
        ];

        // Age bar — always visible
        let (age_b, age_style) = age_bar(age_secs);
        lines.push(Line::from(vec![
            Span::styled("Age   ", theme::header()),
            Span::styled(age_b, age_style),
            Span::styled(format!("  {}", rel_age(&pr.created_at)), age_style),
        ]));

        // Comments — always visible
        let comment_str = if pr.comments_count == 0 {
            "💬 no comments".to_string()
        } else {
            format!("💬 {} comment{}", pr.comments_count, if pr.comments_count > 1 { "s" } else { "" })
        };
        lines.push(Line::from(Span::styled(comment_str, theme::dim())));

        if let Some(d) = details {
            lines.push(Line::raw(""));

            // Decision + merge state badges
            let mut badge_spans = decision_spans(d.review_decision.as_deref());
            badge_spans.extend(mergeable_spans(d.mergeable_state.as_deref()));
            lines.push(Line::from(badge_spans));
            lines.push(Line::raw(""));

            // Review progress bar
            let approved_count = {
                let mut last: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
                for r in &d.reviews {
                    if r.state != "DISMISSED" {
                        last.insert(r.author.login.as_str(), r.state.as_str());
                    }
                }
                last.values().filter(|&&s| s == "APPROVED").count()
            };

            let total_reviewers = (d.requested_reviewers.len() + d.requested_teams.len())
                .max(approved_count);

            let bar_style = if approved_count > 0 && approved_count >= total_reviewers {
                theme::ci_pass()
            } else {
                theme::ci_pending()
            };

            let review_bar = if total_reviewers > 0 {
                bar(approved_count, total_reviewers, 16)
            } else {
                "░".repeat(16)
            };

            lines.push(Line::from(vec![
                Span::styled("Reviews  ", theme::header()),
                Span::styled(review_bar, bar_style),
                Span::styled(format!("  {} / {}", approved_count, total_reviewers), theme::dim()),
            ]));

            for login in &d.requested_reviewers {
                let (symbol, label, style) = match last_review_for(login, &d.reviews) {
                    Some(r) if r.state == "APPROVED" => {
                        let age = r.submitted_at.map(|t| format!("  ·  {} ago", rel_age(&t))).unwrap_or_default();
                        ("✓", format!(" {:<16} approved{}", login, age), theme::ci_pass())
                    }
                    Some(r) if r.state == "CHANGES_REQUESTED" => {
                        let age = r.submitted_at.map(|t| format!("  ·  {} ago", rel_age(&t))).unwrap_or_default();
                        ("✗", format!(" {:<16} changes{}", login, age), theme::ci_fail())
                    }
                    _ => ("·", format!(" {:<16} pending", login), theme::dim()),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", symbol), style),
                    Span::styled(label, theme::dim()),
                ]));
            }

            for team in &d.requested_teams {
                lines.push(Line::from(vec![
                    Span::styled("  · ", theme::dim()),
                    Span::styled(format!(" {:<16} pending (team)", team), theme::dim()),
                ]));
            }

            lines.push(Line::raw(""));

            // Size bar
            let total_lines = d.additions + d.deletions;
            let (size_b, size_style) = size_bar(total_lines);
            lines.push(Line::from(vec![
                Span::styled("Size  ", theme::header()),
                Span::styled(size_b, size_style),
                Span::styled(format!("  +{} / -{}", d.additions, d.deletions), size_style),
            ]));
        } else if self.tab.failed_details.contains(&pr_id) {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("⚠ details unavailable  (r to retry)", theme::ci_pending())));
        } else {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("Loading details…", theme::dim())));
        }

        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
