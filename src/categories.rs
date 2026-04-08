use crate::gh::{self, PullRequest};
use color_eyre::Result;

pub async fn fetch_personal() -> Result<Vec<PullRequest>> {
    gh::search_prs("user-review-requested:@me").await
}

pub async fn fetch_team() -> Result<Vec<PullRequest>> {
    let all = gh::search_prs("review-requested:@me").await?;
    let personal = gh::search_prs("user-review-requested:@me").await?;
    let personal_ids: std::collections::HashSet<u64> =
        personal.iter().map(|pr| pr.number).collect();
    Ok(all
        .into_iter()
        .filter(|pr| !personal_ids.contains(&pr.number))
        .collect())
}

pub async fn fetch_mentioned() -> Result<Vec<PullRequest>> {
    gh::search_prs("mentions:@me").await
}

pub async fn fetch_assigned() -> Result<Vec<PullRequest>> {
    gh::search_prs("assignee:@me").await
}

pub async fn fetch_mine() -> Result<Vec<PullRequest>> {
    gh::search_authored_prs().await
}
