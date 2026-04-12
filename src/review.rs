use crate::gh::PrDetails;

#[derive(Debug, PartialEq, Clone)]
pub enum ReviewStatus {
    Approved,
    ChangesRequested,
    Pending,
}

#[derive(Debug, Clone)]
pub struct ReviewerEntry {
    pub login: String,
    pub status: ReviewStatus,
}

pub fn analyze_reviewers(details: &PrDetails, pr_author: &str) -> Vec<ReviewerEntry> {
    let re_requested: std::collections::HashSet<&str> =
        details.requested_reviewers.iter().map(String::as_str).collect();

    let mut effective: std::collections::HashMap<String, ReviewStatus> =
        std::collections::HashMap::new();

    for r in &details.reviews {
        if r.author.login == pr_author {
            continue;
        }
        let status = match r.state.as_str() {
            "APPROVED" => Some(ReviewStatus::Approved),
            "CHANGES_REQUESTED" => {
                if re_requested.contains(r.author.login.as_str()) {
                    Some(ReviewStatus::Pending)
                } else {
                    Some(ReviewStatus::ChangesRequested)
                }
            }
            "DISMISSED" => Some(ReviewStatus::Pending),
            _ => None,
        };
        if let Some(s) = status {
            effective.insert(r.author.login.clone(), s);
        } else {
            effective.entry(r.author.login.clone()).or_insert(ReviewStatus::Pending);
        }
    }

    for login in &details.requested_reviewers {
        if login != pr_author {
            effective.entry(login.clone()).or_insert(ReviewStatus::Pending);
        }
    }

    let mut entries: Vec<ReviewerEntry> = effective
        .into_iter()
        .map(|(login, status)| ReviewerEntry { login, status })
        .collect();
    entries.sort_by(|a, b| a.login.cmp(&b.login));
    entries
}

pub fn approved_count(entries: &[ReviewerEntry]) -> usize {
    entries.iter().filter(|e| e.status == ReviewStatus::Approved).count()
}

pub fn has_active_changes(entries: &[ReviewerEntry]) -> bool {
    entries.iter().any(|e| e.status == ReviewStatus::ChangesRequested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh::{Author, PrDetails, Review};

    fn make_review(login: &str, state: &str) -> Review {
        Review {
            state: state.to_string(),
            author: Author { login: login.to_string() },
            submitted_at: None,
        }
    }

    fn make_details(reviews: Vec<Review>, requested: Vec<&str>) -> PrDetails {
        PrDetails {
            reviews,
            additions: 0,
            deletions: 0,
            review_decision: None,
            requested_reviewers: requested.into_iter().map(String::from).collect(),
            checks: vec![],
            ci_status: crate::gh::CiStatus::None,
        }
    }

    #[test]
    fn approved_reviewer_shows_approved() {
        let d = make_details(vec![make_review("alice", "APPROVED")], vec![]);
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, ReviewStatus::Approved);
    }

    #[test]
    fn changes_requested_shows_changes_requested() {
        let d = make_details(vec![make_review("alice", "CHANGES_REQUESTED")], vec![]);
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries[0].status, ReviewStatus::ChangesRequested);
    }

    #[test]
    fn re_requested_after_changes_shows_pending() {
        let d = make_details(
            vec![make_review("alice", "CHANGES_REQUESTED")],
            vec!["alice"],
        );
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries[0].status, ReviewStatus::Pending);
    }

    #[test]
    fn comment_does_not_override_approval() {
        let d = make_details(
            vec![
                make_review("alice", "APPROVED"),
                make_review("alice", "COMMENTED"),
            ],
            vec![],
        );
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, ReviewStatus::Approved);
    }

    #[test]
    fn pr_author_excluded_from_reviewers() {
        let d = make_details(vec![make_review("author", "APPROVED")], vec![]);
        let entries = analyze_reviewers(&d, "author");
        assert!(entries.is_empty());
    }

    #[test]
    fn pending_reviewer_not_in_reviews() {
        let d = make_details(vec![], vec!["bob"]);
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries[0].status, ReviewStatus::Pending);
    }

    #[test]
    fn no_duplicate_between_reviews_and_requested() {
        let d = make_details(
            vec![make_review("alice", "APPROVED")],
            vec!["alice"],
        );
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn approved_count_correct() {
        let d = make_details(
            vec![
                make_review("alice", "APPROVED"),
                make_review("bob", "CHANGES_REQUESTED"),
            ],
            vec![],
        );
        let entries = analyze_reviewers(&d, "author");
        assert_eq!(approved_count(&entries), 1);
        assert!(has_active_changes(&entries));
    }
}
