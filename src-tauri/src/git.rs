// Git backend via the `git` CLI (porcelain v2 + numstat).
//
// Scaffold stub: shells out to `git` so the build is fast and dependency-free.
// Planned upgrade (issue #1): migrate to `git2` (libgit2) for in-process reads,
// then native push/pull with credentials (issue #2).

use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

#[derive(Serialize)]
pub struct FileChange {
    pub path: String,
    /// "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked"
    pub status: String,
    pub staged: bool,
    pub added: u32,
    pub removed: u32,
}

#[derive(Serialize)]
pub struct RepoStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

fn git(path: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse `git diff [--cached] --numstat` into path -> (added, removed).
fn numstat(path: &str, cached: bool) -> HashMap<String, (u32, u32)> {
    let mut args = vec!["diff", "--numstat"];
    if cached {
        args.insert(1, "--cached");
    }
    let mut map = HashMap::new();
    if let Ok(text) = git(path, &args) {
        for line in text.lines() {
            let mut cols = line.split('\t');
            let a = cols.next().unwrap_or("0");
            let r = cols.next().unwrap_or("0");
            if let Some(p) = cols.next() {
                // "-" marks binary files; treat as 0.
                let add = a.parse().unwrap_or(0);
                let rem = r.parse().unwrap_or(0);
                let e = map.entry(p.trim().to_string()).or_insert((0, 0));
                e.0 += add;
                e.1 += rem;
            }
        }
    }
    map
}

pub fn get_status(path: &str) -> Result<RepoStatus, String> {
    let text = git(path, &["status", "--porcelain=2", "--branch"])?;

    // Line counts: staged (cached) + unstaged, summed per path.
    let mut deltas = numstat(path, false);
    for (p, (a, r)) in numstat(path, true) {
        let e = deltas.entry(p).or_insert((0, 0));
        e.0 += a;
        e.1 += r;
    }

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
            let xy = rest.get(..2).unwrap_or("..");
            let path = rest.rsplit(' ').next().unwrap_or("").to_string();
            files.push(make(xy, path, &deltas));
        } else if let Some(rest) = line.strip_prefix("2 ") {
            let xy = rest.get(..2).unwrap_or("..");
            let head = rest.split('\t').next().unwrap_or("");
            let path = head.rsplit(' ').next().unwrap_or("").to_string();
            files.push(make(xy, path, &deltas));
        } else if let Some(rest) = line.strip_prefix("? ") {
            let path = rest.trim().to_string();
            let (a, r) = deltas.get(&path).copied().unwrap_or((0, 0));
            files.push(FileChange {
                path,
                status: "untracked".into(),
                staged: false,
                added: a,
                removed: r,
            });
        }
    }

    Ok(RepoStatus { branch, ahead, behind, files })
}

fn make(xy: &str, path: String, deltas: &HashMap<String, (u32, u32)>) -> FileChange {
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
    let (added, removed) = deltas.get(&path).copied().unwrap_or((0, 0));
    FileChange { path, status, staged, added, removed }
}

/// Stage `files` and create a commit. `description` is an optional second
/// message paragraph. Returns Err with git's stderr on failure.
pub fn commit(
    path: &str,
    files: &[String],
    summary: &str,
    description: &str,
) -> Result<(), String> {
    if summary.trim().is_empty() {
        return Err("commit summary is empty".into());
    }
    if files.is_empty() {
        return Err("no files staged for commit".into());
    }

    let mut add = vec!["add", "--"];
    for f in files {
        add.push(f.as_str());
    }
    git(path, &add)?;

    let mut args = vec!["commit", "-m", summary];
    if !description.trim().is_empty() {
        args.push("-m");
        args.push(description);
    }
    git(path, &args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(dir: &str, args: &[&str]) {
        git(dir, args).unwrap_or_else(|e| panic!("git {args:?} failed: {e}"));
    }

    #[test]
    fn status_deltas_and_commit_roundtrip() {
        let dir = std::env::temp_dir().join(format!("glint-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_str().unwrap();

        run(d, &["init", "-q", "-b", "main"]);
        run(d, &["config", "user.email", "t@example.com"]);
        run(d, &["config", "user.name", "Tester"]);

        // Untracked file with 3 lines.
        std::fs::write(dir.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        let st = get_status(d).unwrap();
        assert_eq!(st.branch, "main");
        let f = st.files.iter().find(|f| f.path == "a.txt").expect("a.txt present");
        assert!(!f.staged);

        // Commit it, then the working tree should be clean.
        commit(d, &["a.txt".to_string()], "add a", "").unwrap();
        let st2 = get_status(d).unwrap();
        assert!(st2.files.is_empty(), "clean tree after commit");

        // Modify: numstat should report added/removed line counts.
        std::fs::write(dir.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        let st3 = get_status(d).unwrap();
        let f = st3.files.iter().find(|f| f.path == "a.txt").unwrap();
        assert!(f.added >= 1 && f.removed >= 1, "got +{} -{}", f.added, f.removed);

        // Empty summary is rejected.
        assert!(commit(d, &["a.txt".to_string()], "  ", "").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
