use crate::gh::{CiStatus, PrDetails, PullRequest};
use chrono::Utc;

pub fn compute_priority(pr: &PullRequest, details: Option<&PrDetails>) -> u32 {
    let mut score: u32 = 0;

    let age_secs = (Utc::now() - pr.created_at).num_seconds().unsigned_abs();
    score += match age_secs {
        s if s > 7 * 86400 => 25,
        s if s > 3 * 86400 => 15,
        s if s > 86400 => 8,
        _ => 0,
    };

    if !pr.is_draft {
        score += 10;
    }

    let activity_secs = (Utc::now() - pr.updated_at).num_seconds().unsigned_abs();
    score += match activity_secs {
        s if s < 3600 => 5,
        s if s < 86400 => 3,
        _ => 0,
    };

    if let Some(d) = details {
        let total_lines = d.additions + d.deletions;
        score += match total_lines {
            s if s > 1000 => 15,
            s if s > 500 => 10,
            s if s > 100 => 5,
            _ => 0,
        };

        score += match d.ci_status {
            CiStatus::Pass => 20,
            CiStatus::Pending => 10,
            CiStatus::Fail | CiStatus::None => 0,
        };

        score += match d.review_decision.as_deref() {
            Some("APPROVED") => 25,
            Some("CHANGES_REQUESTED") => 15,
            Some("REVIEW_REQUIRED") => 10,
            _ => 5,
        };
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::{Author, CheckRun, Repository};
    use chrono::Duration;

    fn make_pr(age_days: i64, is_draft: bool, updated_hours_ago: i64) -> PullRequest {
        let now = Utc::now();
        PullRequest {
            number: 1,
            title: "test".to_string(),
            url: "https://example.com".to_string(),
            author: Author { login: "user".to_string() },
            repository: Repository { name_with_owner: "org/repo".to_string() },
            created_at: now - Duration::days(age_days),
            updated_at: now - Duration::hours(updated_hours_ago),
            is_draft,
            comments_count: 0,
        }
    }

    fn make_details(ci: CiStatus, decision: Option<&str>, lines: u32) -> PrDetails {
        PrDetails {
            reviews: vec![],
            additions: lines,
            deletions: 0,
            review_decision: decision.map(String::from),
            requested_reviewers: vec![],
            checks: vec![],
            ci_status: ci,
        }
    }

    #[test]
    fn draft_pr_scores_lower() {
        let draft = make_pr(1, true, 2);
        let non_draft = make_pr(1, false, 2);
        assert!(compute_priority(&draft, None) < compute_priority(&non_draft, None));
    }

    #[test]
    fn old_pr_scores_higher() {
        let fresh = make_pr(0, false, 0);
        let old = make_pr(8, false, 0);
        assert!(compute_priority(&fresh, None) < compute_priority(&old, None));
    }

    #[test]
    fn ci_pass_scores_higher_than_fail() {
        let pr = make_pr(1, false, 2);
        let pass = make_details(CiStatus::Pass, None, 50);
        let fail = make_details(CiStatus::Fail, None, 50);
        assert!(compute_priority(&pr, Some(&pass)) > compute_priority(&pr, Some(&fail)));
    }

    #[test]
    fn approved_scores_highest_review() {
        let pr = make_pr(1, false, 2);
        let approved = make_details(CiStatus::Pass, Some("APPROVED"), 50);
        let pending = make_details(CiStatus::Pass, Some("REVIEW_REQUIRED"), 50);
        assert!(compute_priority(&pr, Some(&approved)) > compute_priority(&pr, Some(&pending)));
    }

    #[test]
    fn large_pr_scores_higher() {
        let pr = make_pr(1, false, 2);
        let small = make_details(CiStatus::Pass, None, 50);
        let large = make_details(CiStatus::Pass, None, 1500);
        assert!(compute_priority(&pr, Some(&small)) < compute_priority(&pr, Some(&large)));
    }
}
