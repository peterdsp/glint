// PR + CI status via the GitHub API (issue #3).
//
// The token is borrowed from the GitHub CLI: GH_TOKEN/GITHUB_TOKEN in the
// environment first, then `gh auth token` (which also covers keychain-stored
// tokens). Glint stores nothing of its own. If no token is available, every
// entry point degrades to `Ok(None)` - the panel simply shows no PR badge.

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct PrStatus {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub draft: bool,
    /// "success" | "failure" | "pending" | "none"
    pub checks: String,
}

/// Resolve a GitHub token the way the gh CLI would see it.
pub fn resolve_token() -> Option<String> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(v) = std::env::var(var) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    let out = std::process::Command::new("gh")
        .args(["auth", "token", "--hostname", "github.com"])
        .output()
        .ok()?;
    if out.status.success() {
        let t = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }
    None
}

/// Parse `owner/repo` from a GitHub remote URL (SSH or HTTPS forms).
pub fn parse_owner_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let rest = if let Some(r) = url.strip_prefix("git@github.com:") {
        r.to_string()
    } else if let Some(r) = url.strip_prefix("ssh://git@github.com/") {
        r.to_string()
    } else if let Some(r) = url.strip_prefix("https://github.com/") {
        r.to_string()
    } else if let Some(r) = url.strip_prefix("http://github.com/") {
        r.to_string()
    } else if let Some(idx) = url.find("github.com") {
        url[idx + "github.com".len()..]
            .trim_start_matches([':', '/'])
            .to_string()
    } else {
        return None;
    };

    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let mut parts = rest.splitn(2, '/');
    let owner = parts.next()?.trim().to_string();
    let repo = parts.next()?.trim().to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

fn origin_url(path: &str) -> Result<String, String> {
    let repo = git2::Repository::open(path).map_err(|e| e.message().to_string())?;
    let remote = repo
        .find_remote("origin")
        .map_err(|_| "no `origin` remote".to_string())?;
    remote
        .url()
        .map(str::to_string)
        .ok_or_else(|| "origin has no URL".to_string())
}

fn branch_name(path: &str) -> Option<String> {
    let repo = git2::Repository::open(path).ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(str::to_string)
}

#[derive(Deserialize)]
struct CheckRunsResp {
    check_runs: Vec<CheckRun>,
}
#[derive(Deserialize)]
struct CheckRun {
    status: String,
    conclusion: Option<String>,
}

/// Aggregate check-run conclusions for a commit into one word. GitHub Actions
/// report via check-runs (not the older commit-status API), so this is what
/// drives the CI dot.
async fn check_conclusion(oc: &octocrab::Octocrab, owner: &str, repo: &str, sha: &str) -> String {
    let route = format!("/repos/{owner}/{repo}/commits/{sha}/check-runs");
    let resp: CheckRunsResp = match oc.get(route, None::<&()>).await {
        Ok(r) => r,
        Err(_) => return "none".to_string(),
    };
    if resp.check_runs.is_empty() {
        return "none".to_string();
    }
    let mut pending = false;
    let mut failed = false;
    for run in &resp.check_runs {
        if run.status != "completed" {
            pending = true;
        } else if let Some(c) = run.conclusion.as_deref() {
            if matches!(
                c,
                "failure" | "timed_out" | "cancelled" | "action_required" | "stale"
            ) {
                failed = true;
            }
        }
    }
    if failed {
        "failure".to_string()
    } else if pending {
        "pending".to_string()
    } else {
        "success".to_string()
    }
}

/// The open PR whose head is the current branch, with its CI conclusion.
pub async fn pr_status(path: &str) -> Result<Option<PrStatus>, String> {
    let Some(token) = resolve_token() else {
        return Ok(None);
    };
    let Some((owner, repo)) = parse_owner_repo(&origin_url(path)?) else {
        return Ok(None);
    };
    let Some(branch) = branch_name(path) else {
        return Ok(None);
    };

    let oc = octocrab::Octocrab::builder()
        .personal_token(token)
        .build()
        .map_err(|e| e.to_string())?;

    let page = oc
        .pulls(&owner, &repo)
        .list()
        .state(octocrab::params::State::Open)
        .head(format!("{owner}:{branch}"))
        .per_page(1)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let Some(pr) = page.items.into_iter().next() else {
        return Ok(None);
    };

    let sha = pr.head.sha;
    let checks = if sha.is_empty() {
        "none".to_string()
    } else {
        check_conclusion(&oc, &owner, &repo, &sha).await
    };

    Ok(Some(PrStatus {
        number: pr.number,
        title: pr.title.unwrap_or_default(),
        url: pr.html_url.map(|u| u.to_string()).unwrap_or_default(),
        draft: pr.draft.unwrap_or(false),
        checks,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ssh_and_https_remotes() {
        let want = Some(("peterdsp".to_string(), "glint".to_string()));
        assert_eq!(parse_owner_repo("git@github.com:peterdsp/glint.git"), want);
        assert_eq!(parse_owner_repo("https://github.com/peterdsp/glint.git"), want);
        assert_eq!(parse_owner_repo("https://github.com/peterdsp/glint"), want);
        assert_eq!(parse_owner_repo("ssh://git@github.com/peterdsp/glint.git"), want);
        assert_eq!(parse_owner_repo("git@github.com:peterdsp/glint/"), want);
    }

    #[test]
    fn rejects_non_github_and_malformed() {
        assert_eq!(parse_owner_repo("git@gitlab.com:x/y.git"), None);
        assert_eq!(parse_owner_repo("https://github.com/onlyowner"), None);
        assert_eq!(parse_owner_repo("not a url"), None);
    }
}
