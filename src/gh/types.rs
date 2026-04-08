use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub author: Author,
    pub repository: Repository,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "isDraft", default)]
    pub is_draft: bool,
    #[serde(rename = "commentsCount", default)]
    pub comments_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Author {
    pub login: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Repository {
    #[serde(rename = "nameWithOwner")]
    pub name_with_owner: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Review {
    pub state: String,
    pub author: Author,
    #[serde(rename = "submittedAt")]
    #[allow(dead_code)]
    pub submitted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct PrDetails {
    pub reviews: Vec<Review>,
    pub additions: u32,
    pub deletions: u32,
    pub review_decision: Option<String>,
    pub requested_reviewers: Vec<String>,
}

pub type PrId = (String, u64);
