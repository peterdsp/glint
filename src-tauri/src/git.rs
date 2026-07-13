// Git status via the `git` CLI (porcelain v2).
//
// This is the scaffold stub: it shells out to `git` so the first build is fast
// and dependency-free. Planned upgrade: migrate to `git2` (libgit2 bindings)
// for in-process reads, then to native push/pull with credential handling.

use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct FileChange {
    pub path: String,
    /// "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked"
    pub status: String,
    pub staged: bool,
}

#[derive(Serialize)]
pub struct RepoStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

pub fn get_status(path: &str) -> Result<RepoStatus, String> {
    let out = Command::new("git")
        .args(["-C", path, "status", "--porcelain=2", "--branch"])
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut branch = "HEAD".to_string();
    let mut ahead = 0;
    let mut behind = 0;
    let mut files = Vec::new();

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for tok in rest.split_whitespace() {
                if let Some(a) = tok.strip_prefix('+') {
                    ahead = a.parse().unwrap_or(0);
                } else if let Some(b) = tok.strip_prefix('-') {
                    behind = b.parse().unwrap_or(0);
                }
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // ordinary change: <xy> <sub> <mH> <mI> <mW> <hH> <hI> <path>
            let xy = &rest.get(..2).unwrap_or("..");
            let path = rest.rsplit(' ').next().unwrap_or("").to_string();
            files.push(classify(xy, path));
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // renamed/copied: fields... <path>\t<origPath>
            let xy = &rest.get(..2).unwrap_or("..");
            let head = rest.split('\t').next().unwrap_or("");
            let path = head.rsplit(' ').next().unwrap_or("").to_string();
            files.push(classify(xy, path));
        } else if let Some(rest) = line.strip_prefix("? ") {
            files.push(FileChange {
                path: rest.trim().to_string(),
                status: "untracked".into(),
                staged: false,
            });
        }
    }

    Ok(RepoStatus { branch, ahead, behind, files })
}

fn classify(xy: &str, path: String) -> FileChange {
    let x = xy.chars().next().unwrap_or('.');
    let y = xy.chars().nth(1).unwrap_or('.');
    let staged = x != '.' && x != '?';
    let code = if y != '.' { y } else { x };
    let status = match code {
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        _ => "modified",
    }
    .to_string();
    FileChange { path, status, staged }
}
