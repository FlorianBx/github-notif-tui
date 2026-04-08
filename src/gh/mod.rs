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
    let reviews_path = format!("repos/{repo}/pulls/{number}/reviews");
    let pr_path = format!("repos/{repo}/pulls/{number}");
    let reviews_args: Vec<&str> = vec!["api", &reviews_path];
    let pr_args: Vec<&str> = vec!["api", &pr_path];
    let (reviews_json, pr_json) = tokio::join!(
        run_gh(&reviews_args),
        run_gh(&pr_args),
    );

    let reviews: Vec<Review> = serde_json::from_str(&reviews_json?)?;
    let pr_data: serde_json::Value = serde_json::from_str(&pr_json?)?;

    let additions = pr_data["additions"].as_u64().unwrap_or(0) as u32;
    let deletions = pr_data["deletions"].as_u64().unwrap_or(0) as u32;
    let review_decision = pr_data["review_decision"]
        .as_str()
        .map(|s| s.to_uppercase());
    let mergeable_state = pr_data["mergeable_state"]
        .as_str()
        .map(|s| s.to_lowercase());

    let requested_reviewers = pr_data["requested_reviewers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| r["login"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let requested_teams = pr_data["requested_teams"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(PrDetails {
        reviews,
        additions,
        deletions,
        review_decision,
        requested_reviewers,
        requested_teams,
        mergeable_state,
    })
}
