use crate::app::{FilterPreset, SortState, TabState};
use crate::review::{analyze_reviewers, has_active_changes, ReviewStatus};
use crate::state::LocalState;
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
    pub filter: FilterPreset,
    pub local: &'a LocalState,
}

impl Widget for DetailPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).title("Detail");

        let Some(pr) = self.tab.selected_pr(self.query, self.sort, self.filter, self.local) else {
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

            let ci_line = render_ci_line(d);
            lines.push(ci_line);

            lines.push(Line::raw(""));

            let reviewers = analyze_reviewers(d, &pr.author.login);
            let active_changes = has_active_changes(&reviewers);
            let has_pending = reviewers.iter().any(|e| e.status == ReviewStatus::Pending);
            let n_approved = reviewers
                .iter()
                .filter(|e| e.status == ReviewStatus::Approved)
                .count();
            let fully_approved =
                d.review_decision.as_deref() == Some("APPROVED") && !has_pending;

            let decision_label = if (fully_approved || n_approved >= 2) && !active_changes {
                Span::styled(format!("{} Approved ({})", icons::CHECK, n_approved), theme::ci_pass())
            } else if active_changes {
                Span::styled(format!("{} Changes requested", icons::CROSS), theme::ci_fail())
            } else {
                match d.review_decision.as_deref() {
                    Some("APPROVED") if has_pending => {
                        Span::styled(format!("{} Review required", icons::CLOCK), theme::ci_pending())
                    }
                    Some("CHANGES_REQUESTED") => {
                        Span::styled(format!("{} Re-review requested", icons::CLOCK), theme::ci_pending())
                    }
                    Some("REVIEW_REQUIRED") if n_approved > 0 => {
                        Span::styled(format!("{} Partial ({}/2)", icons::CLOCK, n_approved), theme::ci_pending())
                    }
                    Some("REVIEW_REQUIRED") => {
                        Span::styled(format!("{} Review required", icons::CLOCK), theme::ci_pending())
                    }
                    Some(other) => Span::styled(other.to_string(), theme::dim()),
                    None => Span::styled(icons::DASH, theme::dim()),
                }
            };
            lines.push(Line::from(vec![
                Span::styled("Decision:", theme::header()),
                Span::raw(" "),
                decision_label,
            ]));

            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled("Reviewers:", theme::header())));
            render_reviewers_lines(&reviewers, &mut lines);

            render_failed_checks(d, &mut lines);

            let score = crate::score::compute_priority(pr, Some(d));
            let score_style = if score > 70 {
                theme::ci_pass()
            } else if score > 40 {
                theme::ci_pending()
            } else {
                theme::ci_fail()
            };
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("Priority:", theme::header()),
                Span::raw(" "),
                Span::styled(format!("{}/100", score), score_style),
            ]));
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

fn render_ci_line<'a>(d: &crate::gh::PrDetails) -> Line<'a> {
    use crate::gh::CiStatus;
    if d.checks.is_empty() {
        return Line::from(vec![
            Span::styled("CI:      ", theme::header()),
            Span::styled(icons::DASH, theme::dim()),
        ]);
    }
    let passed = d.checks.iter().filter(|c| {
        matches!(c.conclusion.to_uppercase().as_str(), "SUCCESS" | "NEUTRAL" | "SKIPPED")
    }).count();
    let failed = d.checks.iter().filter(|c| {
        matches!(c.conclusion.to_uppercase().as_str(), "FAILURE" | "CANCELLED")
    }).count();
    let total = d.checks.len();
    let (icon, style, text) = match d.ci_status {
        CiStatus::Pass => (icons::CHECK, theme::ci_pass(), format!("{}/{} passed", passed, total)),
        CiStatus::Fail => (icons::CROSS, theme::ci_fail(), format!("{} failed, {} passed", failed, passed)),
        CiStatus::Pending => (icons::CLOCK, theme::ci_pending(), format!("{}/{} passed, rest pending", passed, total)),
        CiStatus::None => (icons::DASH, theme::dim(), "none".to_string()),
    };
    Line::from(vec![
        Span::styled("CI:      ", theme::header()),
        Span::styled(format!("{} {}", icon, text), style),
    ])
}

fn render_failed_checks(d: &crate::gh::PrDetails, lines: &mut Vec<Line>) {
    let failed: Vec<&crate::gh::CheckRun> = d.checks.iter().filter(|c| {
        matches!(c.conclusion.to_uppercase().as_str(), "FAILURE" | "CANCELLED")
    }).collect();
    if failed.is_empty() {
        return;
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("Failed checks:", theme::header())));
    for check in failed {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", icons::CROSS), theme::ci_fail()),
            Span::styled(check.name.clone(), theme::ci_fail()),
        ]));
    }
}

fn render_reviewers_lines(reviewers: &[crate::review::ReviewerEntry], lines: &mut Vec<Line>) {
    if reviewers.is_empty() {
        lines.push(Line::from(Span::styled("  no reviewers", theme::dim())));
        return;
    }
    for entry in reviewers {
        let (symbol, style) = match entry.status {
            ReviewStatus::Approved => (icons::CHECK, theme::ci_pass()),
            ReviewStatus::ChangesRequested => (icons::CROSS, theme::ci_fail()),
            ReviewStatus::Pending => (icons::COMMENT, theme::dim()),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", symbol), style),
            Span::raw(entry.login.clone()),
        ]));
    }
}
