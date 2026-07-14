// Git backend via libgit2 (the `git2` crate) - in-process, no subprocess.
//
// Migrated from shelling out to the `git` CLI (issue #1). Reads run against
// libgit2 directly, which is faster (no per-call process spawn), removes the
// hard dependency on a `git` binary on PATH, and gives us a real handle to the
// object database for native push/pull with credentials next (issue #2).
//
// The public surface - `RepoStatus`, `FileChange`, `get_status`, `commit` - is
// unchanged, so the Tauri commands and the frontend are untouched.

use git2::build::CheckoutBuilder;
use git2::{
    Commit, Cred, CredentialType, Diff, DiffOptions, FetchOptions, Patch, PushOptions,
    RemoteCallbacks, Repository, Status, StatusOptions,
};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

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

#[derive(Serialize)]
pub struct DiffLine {
    /// "ctx" | "add" | "del"
    pub kind: String,
    pub old_ln: Option<u32>,
    pub new_ln: Option<u32>,
    pub content: String,
}

#[derive(Serialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize)]
pub struct FileDiff {
    pub file: String,
    pub binary: bool,
    pub hunks: Vec<DiffHunk>,
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
/// is no HEAD or no tracking branch - callers default to (0, 0).
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
/// unstaged (index→workdir) diffs - the equivalent of `git diff --numstat`
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

// ---------------------------------------------------------------------------
// Networked operations (issue #2): fetch / pull / push over the `origin` remote.
// ---------------------------------------------------------------------------

fn find_origin(repo: &Repository) -> Result<git2::Remote<'_>, String> {
    repo.find_remote("origin")
        .map_err(|_| "no `origin` remote is configured for this repository".to_string())
}

/// Remote callbacks with a credential resolver that covers the common cases:
/// SSH via the running agent, then the platform credential helper (macOS
/// Keychain / `git credential`) for HTTPS. An attempt counter guards against
/// libgit2's retry loop when every method is refused.
fn remote_callbacks() -> RemoteCallbacks<'static> {
    #[cfg(not(feature = "appstore"))]
    let config = git2::Config::open_default().and_then(|mut c| c.snapshot()).ok();
    let mut cb = RemoteCallbacks::new();
    let mut attempts = 0usize;
    cb.credentials(move |_url, username, allowed| {
        attempts += 1;
        if attempts > 5 {
            return Err(git2::Error::from_str(
                "authentication failed (add a GitHub token in Settings, or check your SSH agent)",
            ));
        }
        // libgit2 asks for the username first on SSH URLs.
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username.unwrap_or("git"));
        }
        // HTTPS with a token stored in the Keychain. This is the only path that
        // works inside the Mac App Store sandbox, and a fine fallback elsewhere.
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let Some(tok) = crate::github::stored_github_token() {
                return Cred::userpass_plaintext("x-access-token", &tok);
            }
        }
        // SSH agent and the platform credential helper both need to reach
        // outside the app bundle, so they are unavailable in the sandboxed
        // App Store build.
        #[cfg(not(feature = "appstore"))]
        {
            if allowed.contains(CredentialType::SSH_KEY) {
                return Cred::ssh_key_from_agent(username.unwrap_or("git"));
            }
            if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
                if let Some(cfg) = &config {
                    return Cred::credential_helper(cfg, _url, username);
                }
            }
            if allowed.contains(CredentialType::DEFAULT) {
                return Cred::default();
            }
        }
        Err(git2::Error::from_str(
            "no supported authentication method (add a GitHub token in Settings)",
        ))
    });
    cb
}

/// Fetch `origin` (updates remote-tracking refs) and return refreshed status -
/// the ahead/behind counts now reflect the remote without touching the tree.
pub fn fetch(path: &str) -> Result<RepoStatus, String> {
    let repo = open(path)?;
    {
        let mut remote = find_origin(&repo)?;
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(remote_callbacks());
        let no_refspecs: [&str; 0] = [];
        remote.fetch(&no_refspecs, Some(&mut fo), None).map_err(err)?;
    }
    get_status(path)
}

/// Fetch, then fast-forward the current branch to its upstream. Anything that
/// would need a merge or rebase is refused with a clear message (that lives in
/// the pop-out window, issue #4).
pub fn pull(path: &str) -> Result<RepoStatus, String> {
    let repo = open(path)?;
    {
        let mut remote = find_origin(&repo)?;
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(remote_callbacks());
        let no_refspecs: [&str; 0] = [];
        remote.fetch(&no_refspecs, Some(&mut fo), None).map_err(err)?;
    }

    let branch_ref = repo
        .head()
        .map_err(err)?
        .name()
        .ok_or("detached HEAD - cannot pull")?
        .to_string();
    let upstream_name = repo
        .branch_upstream_name(&branch_ref)
        .map_err(|_| "no upstream branch is configured for the current branch".to_string())?;
    let upstream_name = upstream_name.as_str().ok_or("invalid upstream ref name")?.to_string();

    let fetched = {
        let up = repo.find_reference(&upstream_name).map_err(err)?;
        repo.reference_to_annotated_commit(&up).map_err(err)?
    };
    let (analysis, _) = repo.merge_analysis(&[&fetched]).map_err(err)?;

    if analysis.is_up_to_date() {
        return get_status(path);
    }
    if !analysis.is_fast_forward() {
        return Err(
            "pull needs a merge or rebase - Glint does fast-forward only for now".into(),
        );
    }

    // A fast-forward force-checkouts HEAD; refuse if the tree is dirty so we can
    // never discard uncommitted work.
    if !get_status(path)?.files.is_empty() {
        return Err("commit or stash your local changes before pulling".into());
    }

    let mut branch = repo.find_reference(&branch_ref).map_err(err)?;
    branch.set_target(fetched.id(), "glint: fast-forward pull").map_err(err)?;
    repo.set_head(&branch_ref).map_err(err)?;
    repo.checkout_head(Some(CheckoutBuilder::new().force())).map_err(err)?;
    get_status(path)
}

/// Push the current branch to the same-named branch on `origin`. Per-ref
/// rejections (e.g. non-fast-forward) are surfaced as errors rather than
/// silently succeeding.
pub fn push(path: &str) -> Result<RepoStatus, String> {
    let repo = open(path)?;
    let branch_ref = repo
        .head()
        .map_err(err)?
        .name()
        .ok_or("detached HEAD - nothing to push")?
        .to_string();

    let rejected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    {
        let mut remote = find_origin(&repo)?;
        let mut cb = remote_callbacks();
        let rej = rejected.clone();
        cb.push_update_reference(move |refname, status| {
            if let Some(msg) = status {
                *rej.borrow_mut() = Some(format!("{refname}: {msg}"));
            }
            Ok(())
        });
        let mut po = PushOptions::new();
        po.remote_callbacks(cb);
        let refspec = format!("{branch_ref}:{branch_ref}");
        remote.push(&[refspec.as_str()], Some(&mut po)).map_err(err)?;
    }

    if let Some(msg) = rejected.borrow().clone() {
        return Err(format!("push rejected - {msg} (pull first, then push)"));
    }
    get_status(path)
}

// ---------------------------------------------------------------------------
// Diff for the pop-out window (issue #4).
// ---------------------------------------------------------------------------

fn strip_eol(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string()
}

/// Unified diff for a single file - all uncommitted changes (staged and
/// unstaged) against HEAD, as structured hunks the pop-out window renders.
/// Untracked files show up as all-additions.
pub fn diff(path: &str, file: &str) -> Result<FileDiff, String> {
    let repo = open(path)?;
    let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

    let mut opts = DiffOptions::new();
    opts.pathspec(file)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .context_lines(3);

    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut opts))
        .map_err(err)?;

    let mut out = FileDiff {
        file: file.to_string(),
        binary: false,
        hunks: Vec::new(),
    };

    for i in 0..diff.deltas().len() {
        let delta_path = diff
            .get_delta(i)
            .and_then(|d| d.new_file().path().or_else(|| d.old_file().path()))
            .and_then(Path::to_str);
        if delta_path != Some(file) {
            continue;
        }

        let patch = match Patch::from_diff(&diff, i).map_err(err)? {
            Some(p) => p,
            None => {
                out.binary = true;
                break;
            }
        };
        for h in 0..patch.num_hunks() {
            let (hunk, count) = patch.hunk(h).map_err(err)?;
            let mut lines = Vec::new();
            for l in 0..count {
                let line = patch.line_in_hunk(h, l).map_err(err)?;
                let kind = match line.origin() {
                    '+' => "add",
                    '-' => "del",
                    _ => "ctx",
                };
                lines.push(DiffLine {
                    kind: kind.to_string(),
                    old_ln: line.old_lineno(),
                    new_ln: line.new_lineno(),
                    content: strip_eol(line.content()),
                });
            }
            out.hunks.push(DiffHunk {
                header: strip_eol(hunk.header()),
                lines,
            });
        }
        break;
    }
    Ok(out)
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

    // Run git with an explicit working directory (for `init --bare`/`clone`
    // whose paths are positional arguments, not `-C` targets).
    fn run_in(cwd: &Path, args: &[&str]) {
        let ok = std::process::Command::new("git")
            .current_dir(cwd)
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

    // push / fetch / pull against a local bare "remote" - exercises the real
    // libgit2 transport end-to-end without network or credentials (local
    // transport needs no auth).
    #[test]
    fn push_and_pull_local_remote() {
        let base = std::env::temp_dir().join(format!("glint-net-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let remote = base.join("remote.git");
        let a = base.join("a");
        let b = base.join("b");

        // Bare remote + producer repo `a` wired to it.
        run_in(&base, &["init", "-q", "--bare", "-b", "main", remote.to_str().unwrap()]);
        std::fs::create_dir_all(&a).unwrap();
        sh(&a, &["init", "-q", "-b", "main"]);
        sh(&a, &["config", "user.email", "t@example.com"]);
        sh(&a, &["config", "user.name", "Tester"]);
        sh(&a, &["remote", "add", "origin", remote.to_str().unwrap()]);
        let ap = a.to_str().unwrap();

        std::fs::write(a.join("f.txt"), "v1\n").unwrap();
        commit(ap, &["f.txt".to_string()], "first", "").unwrap();
        push(ap).unwrap(); // local branch main -> origin/main

        // Consumer repo `b` clones the remote, then `a` pushes a second commit.
        run_in(&base, &["clone", "-q", remote.to_str().unwrap(), b.to_str().unwrap()]);
        sh(&b, &["config", "user.email", "t@example.com"]);
        sh(&b, &["config", "user.name", "Tester"]);
        let bp = b.to_str().unwrap();

        std::fs::write(a.join("f.txt"), "v1\nv2\n").unwrap();
        commit(ap, &["f.txt".to_string()], "second", "").unwrap();
        push(ap).unwrap();

        // Before pulling, a fetch should show `b` one commit behind.
        let behind = fetch(bp).unwrap();
        assert_eq!(behind.behind, 1, "b sees one commit to pull");
        assert_eq!(behind.ahead, 0);

        // Fast-forward pull brings the second commit into b's working tree.
        let after = pull(bp).unwrap();
        assert_eq!(after.behind, 0, "up to date after pull");
        assert!(after.files.is_empty(), "clean tree after ff pull");
        let content = std::fs::read_to_string(b.join("f.txt")).unwrap();
        assert!(content.contains("v2"), "pulled second commit, got {content:?}");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn diff_reports_added_and_removed_lines() {
        let dir = std::env::temp_dir().join(format!("glint-diff-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.to_str().unwrap();

        sh(&dir, &["init", "-q", "-b", "main"]);
        sh(&dir, &["config", "user.email", "t@example.com"]);
        sh(&dir, &["config", "user.name", "Tester"]);

        std::fs::write(dir.join("x.txt"), "line1\nline2\nline3\n").unwrap();
        commit(d, &["x.txt".to_string()], "seed", "").unwrap();

        // Change a line and append one.
        std::fs::write(dir.join("x.txt"), "line1\nCHANGED\nline3\nline4\n").unwrap();
        let diffed = diff(d, "x.txt").unwrap();
        assert!(!diffed.binary);
        assert!(!diffed.hunks.is_empty(), "expected at least one hunk");

        let all: Vec<&DiffLine> = diffed.hunks.iter().flat_map(|h| &h.lines).collect();
        assert!(all.iter().any(|l| l.kind == "add"), "an addition");
        assert!(all.iter().any(|l| l.kind == "del"), "a deletion");
        assert!(all.iter().any(|l| l.kind == "ctx"), "context lines");
        assert!(
            all.iter().any(|l| l.kind == "add" && l.content.contains("CHANGED")),
            "the changed line is an addition"
        );
        // Context lines carry both line numbers; additions only the new one.
        let add = all.iter().find(|l| l.kind == "add").unwrap();
        assert!(add.new_ln.is_some() && add.old_ln.is_none());

        // An untracked file diffs as all-additions.
        std::fs::write(dir.join("new.txt"), "fresh\n").unwrap();
        let nd = diff(d, "new.txt").unwrap();
        assert!(nd.hunks.iter().flat_map(|h| &h.lines).all(|l| l.kind == "add"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
