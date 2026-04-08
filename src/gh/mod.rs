pub mod types;

use color_eyre::Result;
use tokio::process::Command;

pub use types::*;

const PR_FIELDS: &str =
    "number,title,url,author,repository,createdAt,updatedAt,isDraft,commentsCount";

async fn run_gh(args: &[&str]) -> Result<String> {
    let output = Command::new("gh").args(args).output().await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!("gh error: {}", stderr.trim()));
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub async fn search_prs(query: &str) -> Result<Vec<PullRequest>> {
    let json = run_gh(&[
        "search",
        "prs",
        "--state=open",
        "--limit=100",
        &format!("--json={PR_FIELDS}"),
        "--",
        query,
    ])
    .await?;
    Ok(serde_json::from_str(&json)?)
}

pub async fn fetch_pr_details(repo: &str, number: u64) -> Result<PrDetails> {
    let number_str = number.to_string();
    let json = run_gh(&[
        "pr", "view", &number_str,
        "--repo", repo,
        "--json", "additions,deletions,reviewDecision,reviews,reviewRequests",
    ]).await?;

    let data: serde_json::Value = serde_json::from_str(&json)?;

    let additions = data["additions"].as_u64().unwrap_or(0) as u32;
    let deletions = data["deletions"].as_u64().unwrap_or(0) as u32;
    let review_decision = data["reviewDecision"].as_str().map(|s| s.to_uppercase());
    let reviews: Vec<Review> = serde_json::from_value(data["reviews"].clone()).unwrap_or_default();
    let requested_reviewers = data["reviewRequests"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v["login"].as_str().or_else(|| v["slug"].as_str()).map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(PrDetails { reviews, additions, deletions, review_decision, requested_reviewers })
}
