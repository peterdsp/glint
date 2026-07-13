// Git backend via libgit2 (the `git2` crate) — in-process, no subprocess.
//
// Migrated from shelling out to the `git` CLI (issue #1). Reads run against
// libgit2 directly, which is faster (no per-call process spawn), removes the
// hard dependency on a `git` binary on PATH, and gives us a real handle to the
// object database for native push/pull with credentials next (issue #2).
//
// The public surface — `RepoStatus`, `FileChange`, `get_status`, `commit` — is
// unchanged, so the Tauri commands and the frontend are untouched.

use git2::{Commit, Diff, DiffOptions, Patch, Repository, Status, StatusOptions};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize)]
pub struct FileChange {
    pub path: String,
    /// "modified" | "added" | "deleted" | "renamed" | "untracked"
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

/// libgit2 errors carry a rich message; surface just that to the frontend.
fn err(e: git2::Error) -> String {
    e.message().to_string()
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(err)
}

pub fn get_status(path: &str) -> Result<RepoStatus, String> {
    let repo = open(path)?;
    let branch = current_branch(&repo);
    let (ahead, behind) = ahead_behind(&repo).unwrap_or((0, 0));
    let deltas = line_deltas(&repo);
    let files = collect_files(&repo, &deltas)?;
    Ok(RepoStatus { branch, ahead, behind, files })
}

/// Branch shorthand, resolving the unborn case (a fresh repo with no commits,
/// where `HEAD` is a symbolic ref to a branch that doesn't exist yet).
fn current_branch(repo: &Repository) -> String {
    if let Ok(head) = repo.head() {
        return head.shorthand().unwrap_or("HEAD").to_string();
    }
    repo.find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_string))
        .and_then(|t| t.strip_prefix("refs/heads/").map(str::to_string))
        .unwrap_or_else(|| "HEAD".to_string())
}

/// Commits ahead of / behind the configured upstream branch. `None` when there
/// is no HEAD or no tracking branch — callers default to (0, 0).
fn ahead_behind(repo: &Repository) -> Option<(u32, u32)> {
    let head = repo.head().ok()?;
    let local = head.target()?;
    let upstream_name = repo.branch_upstream_name(head.name()?).ok()?;
    let upstream_ref = repo.find_reference(upstream_name.as_str()?).ok()?;
    let upstream = upstream_ref.target()?;
    let (ahead, behind) = repo.graph_ahead_behind(local, upstream).ok()?;
    Some((ahead as u32, behind as u32))
}

/// Per-path (added, removed) line counts, summing the staged (HEAD→index) and
/// unstaged (index→workdir) diffs — the equivalent of `git diff --numstat`
/// plus `--cached`. Untracked files are excluded (they have no diff), matching
/// the CLI's behaviour.
fn line_deltas(repo: &Repository) -> HashMap<String, (u32, u32)> {
    let mut map = HashMap::new();
    let index = repo.index().ok();

    // Unstaged: index → working directory.
    if let Some(index) = &index {
        if let Ok(diff) = repo.diff_index_to_workdir(Some(index), Some(&mut DiffOptions::new())) {
            accumulate(&diff, &mut map);
        }
    }
    // Staged: HEAD tree → index (tree is None on an unborn branch).
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
    if let Some(index) = &index {
        if let Ok(diff) =
            repo.diff_tree_to_index(head_tree.as_ref(), Some(index), Some(&mut DiffOptions::new()))
        {
            accumulate(&diff, &mut map);
        }
    }
    map
}

fn accumulate(diff: &Diff, map: &mut HashMap<String, (u32, u32)>) {
    for i in 0..diff.deltas().len() {
        // line_stats() is (context, additions, deletions).
        let (add, del) = match Patch::from_diff(diff, i) {
            Ok(Some(patch)) => match patch.line_stats() {
                Ok((_, a, d)) => (a as u32, d as u32),
                Err(_) => continue,
            },
            _ => continue,
        };
        if add == 0 && del == 0 {
            continue;
        }
        if let Some(path) = diff
            .get_delta(i)
            .and_then(|d| d.new_file().path().or_else(|| d.old_file().path()))
            .and_then(Path::to_str)
        {
            let e = map.entry(path.to_string()).or_insert((0, 0));
            e.0 += add;
            e.1 += del;
        }
    }
}

fn collect_files(
    repo: &Repository,
    deltas: &HashMap<String, (u32, u32)>,
) -> Result<Vec<FileChange>, String> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .exclude_submodules(true);

    let statuses = repo.statuses(Some(&mut opts)).map_err(err)?;
    let mut files = Vec::new();
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_ignored() {
            continue;
        }
        let path = match entry.path() {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => continue,
        };
        let staged = s.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        );
        let (added, removed) = deltas.get(&path).copied().unwrap_or((0, 0));
        files.push(FileChange {
            path,
            status: status_label(s),
            staged,
            added,
            removed,
        });
    }
    Ok(files)
}

fn status_label(s: Status) -> String {
    let index_change = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE;
    if s.contains(Status::WT_NEW) && !s.intersects(index_change) {
        "untracked"
    } else if s.intersects(Status::INDEX_DELETED | Status::WT_DELETED) {
        "deleted"
    } else if s.intersects(Status::INDEX_RENAMED | Status::WT_RENAMED) {
        "renamed"
    } else if s.contains(Status::INDEX_NEW) && !s.contains(Status::WT_MODIFIED) {
        "added"
    } else {
        "modified"
    }
    .to_string()
}

/// Stage `files` and create a commit authored/committed by the repo's
/// configured identity. `description` is an optional second message paragraph.
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

    let repo = open(path)?;
    let workdir = repo
        .workdir()
        .ok_or("bare repositories cannot be committed to from Glint")?
        .to_path_buf();
    let mut index = repo.index().map_err(err)?;

    for f in files {
        let rel = Path::new(f);
        // A file present on disk is added/updated; a missing one was deleted.
        if workdir.join(rel).exists() {
            index.add_path(rel).map_err(err)?;
        } else {
            index.remove_path(rel).map_err(err)?;
        }
    }
    index.write().map_err(err)?;

    let tree = repo
        .find_tree(index.write_tree().map_err(err)?)
        .map_err(err)?;
    let sig = repo
        .signature()
        .map_err(|e| format!("no git identity configured (user.name / user.email): {}", e.message()))?;

    let message = if description.trim().is_empty() {
        summary.trim().to_string()
    } else {
        format!("{}\n\n{}", summary.trim(), description.trim())
    };

    // Parent is the current HEAD commit, or none for the first commit.
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&Commit> = parent.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &parents)
        .map_err(err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Setup helper: drive real git via the CLI so the test asserts our libgit2
    // reads against a repo built the ordinary way.
    fn sh(dir: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .expect("spawn git")
            .success();
        assert!(ok, "git {args:?} failed");
    }

    #[test]
    fn status_deltas_and_commit_roundtrip() {
        let dir = std::env::temp_dir().join(format!("glint-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_str().unwrap();

        sh(&dir, &["init", "-q", "-b", "main"]);
        sh(&dir, &["config", "user.email", "t@example.com"]);
        sh(&dir, &["config", "user.name", "Tester"]);

        // Unborn branch is still reported as "main"; the new file is untracked.
        std::fs::write(dir.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        let st = get_status(d).unwrap();
        assert_eq!(st.branch, "main");
        let f = st.files.iter().find(|f| f.path == "a.txt").expect("a.txt present");
        assert!(!f.staged);
        assert_eq!(f.status, "untracked");

        // Commit it via libgit2; the working tree should then be clean.
        commit(d, &["a.txt".to_string()], "add a", "").unwrap();
        let st2 = get_status(d).unwrap();
        assert!(st2.files.is_empty(), "clean tree after commit, got {:?}", st2.files.len());
        assert_eq!(st2.branch, "main");

        // Modify: numstat-equivalent should report added/removed counts.
        std::fs::write(dir.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        let st3 = get_status(d).unwrap();
        let f = st3.files.iter().find(|f| f.path == "a.txt").unwrap();
        assert!(f.added >= 1 && f.removed >= 1, "got +{} -{}", f.added, f.removed);
        assert_eq!(f.status, "modified");

        // A second commit with a description parent-links correctly.
        commit(d, &["a.txt".to_string()], "edit a", "more detail").unwrap();
        let st4 = get_status(d).unwrap();
        assert!(st4.files.is_empty());

        // Empty summary is rejected.
        std::fs::write(dir.join("a.txt"), "changed\n").unwrap();
        assert!(commit(d, &["a.txt".to_string()], "  ", "").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
