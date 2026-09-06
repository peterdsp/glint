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
    Commit, Cred, CredentialType, Diff, DiffOptions, ErrorCode, FetchOptions, IndexEntry,
    IndexTime, Patch, PushOptions, RemoteCallbacks, Repository, Status, StatusOptions, Tree,
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
    /// The file has BOTH staged and unstaged changes. Selecting it commits the
    /// whole working-tree file (staged and unstaged together), so the UI can warn
    /// before that collapses the split. See `commit`'s doc comment.
    pub partially_staged: bool,
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
        // Both staged and unstaged edits on the same file: committing it takes the
        // whole working-tree copy, so the frontend flags this before it happens.
        let unstaged = s.intersects(
            Status::WT_MODIFIED
                | Status::WT_TYPECHANGE
                | Status::WT_DELETED
                | Status::WT_RENAMED,
        );
        let (added, removed) = deltas.get(&path).copied().unwrap_or((0, 0));
        files.push(FileChange {
            path,
            status: status_label(s),
            staged,
            partially_staged: staged && unstaged,
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

/// One selected file's effect on the commit tree.
struct PlannedChange {
    /// The HEAD-side path: what the UI lists and the user checked. For a rename
    /// this is the old name (matching `git2`'s status path, which is old-side).
    old_path: String,
    /// The working-tree path to commit, or `None` for a deletion. Differs from
    /// `old_path` only for a rename (then it is the new name).
    target: Option<String>,
}

/// What committing does with the files the user checked in Glint.
///
/// Glint is a whole-file client: checking a file means "commit this file's
/// current working-tree content, in full". The commit is built from the current
/// HEAD tree with ONLY the selected files applied on top, so:
///
/// - A file staged outside Glint (e.g. `git add other.txt`) that is left
///   unchecked never enters the commit, and its staged content stays in the
///   index afterwards.
/// - Unrelated worktree edits are never touched.
/// - A partially staged file that IS selected is committed in full from the
///   working tree (staged hunks plus any further unstaged edits). A whole-file
///   commit cannot preserve a within-file staged/unstaged split, so afterwards
///   that file is fully committed and shows as clean; nothing on disk is lost.
///   Leave the file unchecked to keep its staged/worktree split intact.
///
/// After the commit the real index is reconciled with the new HEAD for exactly
/// the committed paths (like `git commit -- <paths>`): those paths show as
/// clean, and every other index entry is left as it was.
///
/// Refuses an empty selection, a selection with no actual changes, a bare repo,
/// unresolved merge conflicts, and any in-progress operation (merge, rebase,
/// cherry-pick, revert, bisect, patch apply) with an actionable message.
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
        return Err("no files are selected to commit".into());
    }

    let repo = open(path)?;
    let workdir = repo
        .workdir()
        .ok_or("bare repositories cannot be committed to from Glint")?
        .to_path_buf();

    ensure_committable(&repo)?;

    // Resolve each checked path to a concrete change (add / modify / delete /
    // rename) using the same status scan the UI is built from, so a checked path
    // maps 1:1 to a change. Unknown or unchanged paths are dropped here and
    // caught by the "no changes" guard below.
    let plan = commit_plan(&repo, &workdir, files)?;

    // Capture the ref we are building on (its name and tip) up front, so the
    // commit can be attached with a compare-and-swap that refuses to clobber a
    // concurrent move rather than silently overwriting it.
    let head = repo.head().ok();
    let head_commit = head.as_ref().and_then(|h| h.peel_to_commit().ok());
    let head_ref_name = head.as_ref().and_then(|h| h.name()).map(str::to_string);
    let head_tree = head_commit.as_ref().and_then(|c| c.tree().ok());

    // Build the tree to commit: HEAD (empty on an unborn branch) plus only the
    // selected changes. The repo index is used as scratch because libgit2 needs
    // a repo-backed index to hash working-tree blobs; seeding it from HEAD keeps
    // anything staged outside the selection out of the tree. `write_tree` writes
    // the tree object but NOT the on-disk index file, so the real index is still
    // intact on disk for the reconciliation step.
    let mut index = repo.index().map_err(err)?;
    match &head_tree {
        Some(t) => index.read_tree(t).map_err(err)?,
        None => index.clear().map_err(err)?,
    }
    apply_plan(&mut index, &plan)?;
    let tree = repo
        .find_tree(index.write_tree().map_err(err)?)
        .map_err(err)?;

    // Nothing to commit if the selection does not change the HEAD tree.
    let no_change = match &head_tree {
        Some(t) => t.id() == tree.id(),
        None => tree.is_empty(),
    };
    if no_change {
        // The scratch mutations were in-memory only; drop them so the on-disk
        // index is left exactly as we found it.
        let _ = index.read(true);
        return Err("the selected files have no changes to commit".into());
    }

    let sig = repo.signature().map_err(|e| {
        format!("no git identity configured (user.name / user.email): {}", e.message())
    })?;
    let message = if description.trim().is_empty() {
        summary.trim().to_string()
    } else {
        format!("{}\n\n{}", summary.trim(), description.trim())
    };
    let parents: Vec<&Commit> = head_commit.iter().collect();

    // Create the commit OBJECT without moving any ref yet. Attaching the branch
    // as a separate compare-and-swap lets us refuse to clobber a commit another
    // Git process made since we read HEAD, instead of overwriting it silently.
    let new_oid = repo
        .commit(None, &sig, &sig, &message, &tree, &parents)
        .map_err(err)?;

    // Point the branch (or a detached HEAD) at the new commit, but only if it is
    // still where we built on. If it moved, nothing was written to the branch
    // (the new commit is just an unreferenced object), so returning an error here
    // is safe to retry - unlike a failure AFTER this point, handled below.
    attach_commit(&repo, &head_ref_name, head_commit.as_ref(), new_oid, summary)?;

    // The commit is now durable. Reconciling the index to it is best effort: a
    // failure here must NOT surface as a commit failure, which would tempt the
    // user into re-committing what was just committed. The reconciliation copies
    // the committed tree (not the working tree), so a file changed after the tree
    // was built stays an unstaged edit rather than silently becoming staged.
    if let Err(e) = finish_reconcile(&mut index, &tree, &plan) {
        eprintln!("glint: commit {new_oid} succeeded but the index was not reconciled: {e}");
    }
    Ok(())
}

/// Attach `new_oid` to the branch (or detached HEAD) with a compare-and-swap:
/// update only if the ref still points where we built on, so a commit made by
/// another Git process since we read HEAD is never silently overwritten. On an
/// unborn branch, create the branch only if nothing created it first. A failure
/// means nothing landed on the branch, so the caller may safely retry.
fn attach_commit(
    repo: &Repository,
    head_ref_name: &Option<String>,
    parent: Option<&Commit>,
    new_oid: git2::Oid,
    summary: &str,
) -> Result<(), String> {
    match parent {
        Some(parent) => {
            let name = head_ref_name
                .clone()
                .ok_or("HEAD has no reference name to update")?;
            let reflog = format!("commit: {}", summary.trim());
            repo.reference_matching(&name, new_oid, true, parent.id(), &reflog)
                .map_err(branch_moved)?;
        }
        None => {
            let name = repo
                .find_reference("HEAD")
                .ok()
                .and_then(|h| h.symbolic_target().map(str::to_string))
                .unwrap_or_else(|| "refs/heads/main".to_string());
            let reflog = format!("commit (initial): {}", summary.trim());
            repo.reference(&name, new_oid, false, &reflog)
                .map_err(branch_moved)?;
        }
    }
    Ok(())
}

/// Turn a failed branch update into an actionable message. A compare-and-swap
/// miss (the ref moved) or a lost create race means nothing landed on the branch,
/// so the user can safely retry.
fn branch_moved(e: git2::Error) -> String {
    match e.code() {
        ErrorCode::Modified | ErrorCode::Exists => "the branch changed in another Git \
            process while committing; nothing was committed, so refresh and try again"
            .to_string(),
        _ => format!("could not update the branch: {}", e.message()),
    }
}

/// Reload the real on-disk index (the scratch tree-build never wrote it, so it
/// still holds unrelated staged changes) and reconcile the committed paths to the
/// new commit.
fn finish_reconcile(
    index: &mut git2::Index,
    committed_tree: &Tree,
    plan: &[PlannedChange],
) -> Result<(), String> {
    index.read(true).map_err(err)?;
    reconcile_index(index, committed_tree, plan)
}

/// Refuse commits Glint cannot safely build: a mid-operation repository or one
/// with unresolved conflicts. The message tells the user how to proceed.
fn ensure_committable(repo: &Repository) -> Result<(), String> {
    use git2::RepositoryState::*;
    let op = match repo.state() {
        Clean => None,
        Merge => Some("a merge"),
        Revert | RevertSequence => Some("a revert"),
        CherryPick | CherryPickSequence => Some("a cherry-pick"),
        Bisect => Some("a bisect"),
        Rebase | RebaseInteractive | RebaseMerge => Some("a rebase"),
        ApplyMailbox | ApplyMailboxOrRebase => Some("a patch apply"),
    };
    if let Some(op) = op {
        return Err(format!(
            "{op} is in progress in this repository; finish or abort it in your terminal before committing with Glint"
        ));
    }
    if repo.index().map_err(err)?.has_conflicts() {
        return Err(
            "this repository has unresolved merge conflicts; resolve them before committing".into(),
        );
    }
    Ok(())
}

/// Map each checked path to a concrete change using the same status scan that
/// feeds the UI, so a checked path (the HEAD-side name, including for renames)
/// resolves to exactly one change. Paths with no matching change are skipped.
fn commit_plan(
    repo: &Repository,
    workdir: &Path,
    files: &[String],
) -> Result<Vec<PlannedChange>, String> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(err)?;

    // old_path -> target (the working-tree path to commit, or None for a delete).
    let mut changes: HashMap<String, Option<String>> = HashMap::new();
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_ignored() {
            continue;
        }
        let old_path = match entry.path() {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => continue,
        };
        // The file's current working-tree path (the new name for a rename),
        // preferring the unstaged delta since it reflects the actual worktree.
        let wt = entry
            .index_to_workdir()
            .and_then(|d| d.new_file().path().map(Path::to_path_buf))
            .or_else(|| {
                entry
                    .head_to_index()
                    .and_then(|d| d.new_file().path().map(Path::to_path_buf))
            })
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| old_path.clone());
        // The working tree is the source of truth for a whole-file commit: if
        // the file is on disk we commit it, otherwise the change is a deletion.
        let target = if workdir.join(&wt).exists() { Some(wt) } else { None };
        changes.insert(old_path, target);
    }

    let mut plan = Vec::new();
    for f in files {
        if let Some(target) = changes.get(f) {
            plan.push(PlannedChange {
                old_path: f.clone(),
                target: target.clone(),
            });
        }
    }
    Ok(plan)
}

/// Apply the planned changes to `index` (already seeded from HEAD): stage the
/// working-tree version of each file, drop the old name of a rename, and remove
/// deletions. Dropping a name is best-effort: a file that was staged as new and
/// then renamed or deleted has no HEAD entry to remove, and its absence from the
/// tree is already the correct result.
fn apply_plan(index: &mut git2::Index, plan: &[PlannedChange]) -> Result<(), String> {
    for change in plan {
        match &change.target {
            Some(w) => {
                index.add_path(Path::new(w)).map_err(err)?;
                if *w != change.old_path {
                    let _ = index.remove_path(Path::new(&change.old_path));
                }
            }
            None => {
                let _ = index.remove_path(Path::new(&change.old_path));
            }
        }
    }
    Ok(())
}

/// Bring the real (on-disk) index into line with the just-created commit for the
/// committed paths only, copying each file's EXACT committed blob from the tree
/// rather than re-reading the working tree. That distinction matters: if a file
/// is edited between building the tree and here, its newer content must stay an
/// unstaged edit, not silently become staged. A rename's old name and deletions
/// are dropped (best effort: a path staged as new and then renamed or deleted has
/// no entry to remove). Unrelated index entries are never mentioned, so they are
/// preserved exactly, and paths are matched literally, so a filename with glob
/// characters can never catch a sibling.
fn reconcile_index(
    index: &mut git2::Index,
    committed_tree: &Tree,
    plan: &[PlannedChange],
) -> Result<(), String> {
    for change in plan {
        match &change.target {
            Some(w) => {
                if let Ok(entry) = committed_tree.get_path(Path::new(w)) {
                    // Stat fields are zeroed, exactly as `git reset` leaves them;
                    // `git status` then re-hashes the file to decide clean vs. an
                    // unstaged edit, so a concurrent change is reported correctly.
                    index
                        .add(&IndexEntry {
                            ctime: IndexTime::new(0, 0),
                            mtime: IndexTime::new(0, 0),
                            dev: 0,
                            ino: 0,
                            mode: entry.filemode() as u32,
                            uid: 0,
                            gid: 0,
                            file_size: 0,
                            id: entry.id(),
                            flags: 0,
                            flags_extended: 0,
                            path: w.as_bytes().to_vec(),
                        })
                        .map_err(err)?;
                }
                if *w != change.old_path {
                    let _ = index.remove_path(Path::new(&change.old_path));
                }
            }
            None => {
                let _ = index.remove_path(Path::new(&change.old_path));
            }
        }
    }
    index.write().map_err(err)
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

    use git2::Repository;

    // A throwaway repo dir under the temp dir, unique per test prefix. Each test
    // uses a distinct prefix so parallel runs (same pid) never share a path.
    fn scratch(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_repo(dir: &Path) {
        sh(dir, &["init", "-q", "-b", "main"]);
        sh(dir, &["config", "user.email", "t@example.com"]);
        sh(dir, &["config", "user.name", "Tester"]);
    }

    fn write(dir: &Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn read(dir: &Path, name: &str) -> String {
        std::fs::read_to_string(dir.join(name)).unwrap()
    }

    // True if the current HEAD commit's tree contains `path`.
    fn head_has(dir: &Path, path: &str) -> bool {
        let repo = Repository::open(dir).unwrap();
        let tree = repo.head().unwrap().peel_to_tree().unwrap();
        tree.get_path(Path::new(path)).is_ok()
    }

    // Content of `path` as recorded in the HEAD commit.
    fn head_blob(dir: &Path, path: &str) -> String {
        let repo = Repository::open(dir).unwrap();
        let tree = repo.head().unwrap().peel_to_tree().unwrap();
        let entry = tree.get_path(Path::new(path)).unwrap();
        let blob = repo.find_blob(entry.id()).unwrap();
        String::from_utf8_lossy(blob.content()).into_owned()
    }

    // Content staged in the index for `path`, or None if it isn't staged.
    fn staged_blob(dir: &Path, path: &str) -> Option<String> {
        let repo = Repository::open(dir).unwrap();
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new(path), 0)?;
        let blob = repo.find_blob(entry.id).unwrap();
        Some(String::from_utf8_lossy(blob.content()).into_owned())
    }

    // Find a file entry in a status listing.
    fn file<'a>(st: &'a RepoStatus, path: &str) -> Option<&'a FileChange> {
        st.files.iter().find(|f| f.path == path)
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

    // A file staged outside Glint (`git add`) and left unchecked must not enter
    // the commit, and must keep its staged content afterwards. This is the core
    // defect: the old code committed the whole index, leaking unchecked files.
    #[test]
    fn cli_staged_unchecked_file_stays_out_and_keeps_staged_changes() {
        let dir = scratch("glint-unchecked");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "base.txt", "base\n");
        sh(&dir, &["add", "base.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        // other.txt is staged via the CLI but left UNCHECKED in Glint.
        write(&dir, "other.txt", "staged via cli\n");
        sh(&dir, &["add", "other.txt"]);
        // a.txt is the file the user checks.
        write(&dir, "a.txt", "pick me\n");

        commit(d, &["a.txt".to_string()], "add a", "").unwrap();

        // The commit holds exactly a.txt, never the unchecked staged file.
        assert!(head_has(&dir, "a.txt"), "selected file is committed");
        assert!(!head_has(&dir, "other.txt"), "unchecked staged file must not be committed");

        // other.txt is still staged, with its staged content preserved.
        let st = get_status(d).unwrap();
        let o = file(&st, "other.txt").expect("other.txt still listed");
        assert!(o.staged, "other.txt stays staged");
        assert_eq!(staged_blob(&dir, "other.txt").as_deref(), Some("staged via cli\n"));
        assert!(file(&st, "a.txt").is_none(), "a.txt is clean after commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Selected files land in the commit; an unrelated staged addition and an
    // unrelated unstaged worktree edit both survive, and the committed files
    // reconcile to clean.
    #[test]
    fn selected_files_committed_index_and_worktree_reconcile() {
        let dir = scratch("glint-reconcile");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "base.txt", "base\n");
        write(&dir, "dirty.txt", "v1\n");
        sh(&dir, &["add", "-A"]);
        sh(&dir, &["commit", "-qm", "base"]);

        write(&dir, "f1.txt", "one\n");
        write(&dir, "f2.txt", "two\n");
        write(&dir, "staged.txt", "staged\n");
        sh(&dir, &["add", "staged.txt"]); // unrelated staged addition, unchecked
        write(&dir, "dirty.txt", "v1\nv2\n"); // unrelated unstaged edit, unchecked

        commit(d, &["f1.txt".to_string(), "f2.txt".to_string()], "add f1 f2", "detail").unwrap();

        assert!(head_has(&dir, "f1.txt") && head_has(&dir, "f2.txt"), "selected files committed");
        assert!(!head_has(&dir, "staged.txt"), "unrelated staged file excluded");
        assert_eq!(head_blob(&dir, "f2.txt"), "two\n");
        // base.txt (a HEAD file the selection never touched) is still present.
        assert!(head_has(&dir, "base.txt"), "untouched HEAD files remain");

        let st = get_status(d).unwrap();
        assert!(
            file(&st, "f1.txt").is_none() && file(&st, "f2.txt").is_none(),
            "committed files are clean"
        );
        assert!(file(&st, "staged.txt").expect("staged.txt listed").staged, "staged add preserved");
        let dy = file(&st, "dirty.txt").expect("dirty.txt listed");
        assert!(!dy.staged && dy.status == "modified", "unstaged worktree edit preserved");
        assert_eq!(read(&dir, "dirty.txt"), "v1\nv2\n", "worktree edit intact on disk");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Policy: selecting a partially staged file commits its full working-tree
    // content, and the file is clean afterwards.
    #[test]
    fn partially_staged_selected_commits_whole_worktree() {
        let dir = scratch("glint-partial-sel");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "p.txt", "l1\nl2\nl3\n");
        sh(&dir, &["add", "p.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        write(&dir, "p.txt", "L1\nl2\nl3\n");
        sh(&dir, &["add", "p.txt"]); // index = the staged edit
        write(&dir, "p.txt", "L1\nl2\nL3\n"); // worktree = a further edit

        commit(d, &["p.txt".to_string()], "commit p", "").unwrap();

        assert_eq!(head_blob(&dir, "p.txt"), "L1\nl2\nL3\n", "whole working tree committed");
        let st = get_status(d).unwrap();
        assert!(file(&st, "p.txt").is_none(), "selected partially staged file is clean after commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Policy: an UNSELECTED partially staged file keeps its staged/worktree
    // split and stays out of the commit.
    #[test]
    fn partially_staged_unselected_keeps_split() {
        let dir = scratch("glint-partial-keep");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "p.txt", "l1\nl2\nl3\n");
        sh(&dir, &["add", "p.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        write(&dir, "p.txt", "L1\nl2\nl3\n");
        sh(&dir, &["add", "p.txt"]); // staged content
        write(&dir, "p.txt", "L1\nl2\nL3\n"); // further worktree edit

        write(&dir, "q.txt", "q\n");
        commit(d, &["q.txt".to_string()], "add q", "").unwrap();

        assert!(head_has(&dir, "q.txt"), "the selected file is committed");
        assert_eq!(head_blob(&dir, "p.txt"), "l1\nl2\nl3\n", "p.txt in HEAD unchanged");
        assert_eq!(staged_blob(&dir, "p.txt").as_deref(), Some("L1\nl2\nl3\n"), "staged split preserved");
        assert_eq!(read(&dir, "p.txt"), "L1\nl2\nL3\n", "worktree split preserved");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // A deletion selected in Glint is recorded as a deletion; other files stay.
    #[test]
    fn deletion_is_committed() {
        let dir = scratch("glint-del");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "keep.txt", "keep\n");
        write(&dir, "del.txt", "gone soon\n");
        sh(&dir, &["add", "-A"]);
        sh(&dir, &["commit", "-qm", "base"]);

        std::fs::remove_file(dir.join("del.txt")).unwrap();
        commit(d, &["del.txt".to_string()], "remove del", "").unwrap();

        assert!(!head_has(&dir, "del.txt"), "deletion recorded in the commit");
        assert!(head_has(&dir, "keep.txt"), "other files untouched");
        assert!(file(&get_status(d).unwrap(), "del.txt").is_none(), "clean after deletion commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // A staged rename (`git mv`) commits under the new name and drops the old.
    #[test]
    fn staged_rename_is_committed_as_rename() {
        let dir = scratch("glint-rename-staged");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "old.txt", "same content\n");
        sh(&dir, &["add", "old.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        sh(&dir, &["mv", "old.txt", "new.txt"]);

        // Glint lists the rename under its old-side path (git2 status is old-side).
        let st = get_status(d).unwrap();
        let r = st.files.iter().find(|f| f.status == "renamed").expect("a renamed entry");
        let selected = r.path.clone();

        commit(d, &[selected], "rename old to new", "").unwrap();

        assert!(head_has(&dir, "new.txt"), "new name committed");
        assert!(!head_has(&dir, "old.txt"), "old name dropped");
        assert_eq!(head_blob(&dir, "new.txt"), "same content\n");
        let st2 = get_status(d).unwrap();
        assert!(
            st2.files.is_empty(),
            "clean after rename commit, got {:?}",
            st2.files.iter().map(|f| &f.path).collect::<Vec<_>>()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // An on-disk rename (not staged) commits under the new name and drops the
    // old, whether libgit2 reports it as one rename entry or as delete+add.
    #[test]
    fn unstaged_rename_is_committed() {
        let dir = scratch("glint-rename-unstaged");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "old.txt", "identical body\n");
        sh(&dir, &["add", "old.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        std::fs::rename(dir.join("old.txt"), dir.join("new.txt")).unwrap();

        // Select every changed entry Glint lists, mirroring a user who checks the
        // rename however it is surfaced.
        let selected: Vec<String> = get_status(d).unwrap().files.iter().map(|f| f.path.clone()).collect();
        commit(d, &selected, "rename old to new", "").unwrap();

        assert!(head_has(&dir, "new.txt"), "new name committed");
        assert!(!head_has(&dir, "old.txt"), "old name dropped");
        assert!(get_status(d).unwrap().files.is_empty(), "clean after rename commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // A file staged as new and then renamed on disk still commits cleanly under
    // the new name, rather than erroring on a missing old-name index entry (the
    // old name was never in HEAD, so dropping it is a no-op).
    #[test]
    fn staged_new_then_renamed_commits_new_name() {
        let dir = scratch("glint-newrename");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "seed.txt", "seed\n");
        sh(&dir, &["add", "seed.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        write(&dir, "old.txt", "brand new\n");
        sh(&dir, &["add", "old.txt"]); // staged as new, never committed
        std::fs::rename(dir.join("old.txt"), dir.join("new.txt")).unwrap();

        let selected: Vec<String> = get_status(d).unwrap().files.iter().map(|f| f.path.clone()).collect();
        commit(d, &selected, "add renamed", "").unwrap();

        assert!(head_has(&dir, "new.txt"), "committed under the new name");
        assert!(!head_has(&dir, "old.txt"), "the never-committed old name is absent");
        assert_eq!(head_blob(&dir, "new.txt"), "brand new\n");
        assert!(get_status(d).unwrap().files.is_empty(), "clean after commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // The initial commit on an unborn branch commits only the selected file and
    // preserves an unrelated file staged before that first commit.
    #[test]
    fn initial_commit_on_unborn_branch_keeps_unrelated_staged() {
        let dir = scratch("glint-unborn");
        let d = dir.to_str().unwrap();
        init_repo(&dir); // no commits yet: unborn main

        write(&dir, "a.txt", "first\n");
        write(&dir, "other.txt", "staged only\n");
        sh(&dir, &["add", "other.txt"]); // staged before the first commit, unchecked

        assert_eq!(get_status(d).unwrap().branch, "main", "unborn branch still reports its name");

        commit(d, &["a.txt".to_string()], "initial", "").unwrap();

        assert!(head_has(&dir, "a.txt"), "initial commit contains the selected file");
        assert!(!head_has(&dir, "other.txt"), "unchecked staged file excluded from initial commit");
        let st = get_status(d).unwrap();
        assert!(file(&st, "other.txt").expect("other.txt present").staged, "staged add survives");
        assert!(file(&st, "a.txt").is_none(), "committed file is clean");
        // The unborn branch now has exactly one root commit.
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent_count(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // An empty selection and a selection with no actual changes are both refused,
    // and neither moves HEAD.
    #[test]
    fn empty_and_noop_selections_are_refused() {
        let dir = scratch("glint-empty");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "a.txt", "a\n");
        sh(&dir, &["add", "a.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        let none: Vec<String> = Vec::new();
        assert!(commit(d, &none, "msg", "").is_err(), "empty selection refused");

        let e = commit(d, &["a.txt".to_string()], "msg", "").unwrap_err();
        assert!(e.contains("no changes"), "no-op selection refused with a clear message: {e}");

        // HEAD is still the single base commit (nothing was committed).
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().peel_to_commit().unwrap().parent_count(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // A repository mid-merge (an in-progress operation with an unmerged index) is
    // refused with an actionable message rather than producing a broken commit.
    #[test]
    fn in_progress_operation_is_refused() {
        let dir = scratch("glint-merge");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "c.txt", "base\n");
        sh(&dir, &["add", "c.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        sh(&dir, &["checkout", "-qb", "feature"]);
        write(&dir, "c.txt", "feature\n");
        sh(&dir, &["commit", "-qam", "feature edit"]);
        sh(&dir, &["checkout", "-q", "main"]);
        write(&dir, "c.txt", "mainline\n");
        sh(&dir, &["commit", "-qam", "main edit"]);

        // Conflicting merge: leaves the repo mid-merge with an unmerged index.
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["merge", "feature"])
            .status();

        let e = commit(d, &["c.txt".to_string()], "resolve", "").unwrap_err();
        assert!(
            e.contains("in progress") || e.contains("conflict"),
            "mid-merge commit refused with an actionable message: {e}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // A bare repository has no working tree to commit from and is refused.
    #[test]
    fn bare_repository_is_refused() {
        let base = scratch("glint-bare");
        run_in(&base, &["init", "-q", "--bare", "-b", "main", "r.git"]);
        let bare = base.join("r.git");
        let e = commit(bare.to_str().unwrap(), &["x".to_string()], "m", "").unwrap_err();
        assert!(e.contains("bare"), "bare repo refused: {e}");
        let _ = std::fs::remove_dir_all(&base);
    }

    // Issue #1: reconciliation must copy the COMMITTED tree, not re-read the
    // working tree. A file changed after the tree is built must stay an unstaged
    // edit, not silently become staged. Drives `reconcile_index` directly with a
    // worktree that differs from the committed tree (the mid-commit race the
    // black-box `commit` path cannot inject).
    #[test]
    fn reconcile_uses_committed_tree_not_worktree() {
        let dir = scratch("glint-reconcile-tree");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "seed.txt", "seed\n");
        sh(&dir, &["add", "seed.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        // Commit f.txt = "X" so HEAD, index and worktree all hold X.
        write(&dir, "f.txt", "X\n");
        commit(d, &["f.txt".to_string()], "add f", "").unwrap();
        assert_eq!(head_blob(&dir, "f.txt"), "X\n");

        // Now the worktree gains newer content AFTER the committed tree exists.
        write(&dir, "f.txt", "Y\n");
        let repo = Repository::open(&dir).unwrap();
        let committed_tree = repo.head().unwrap().peel_to_tree().unwrap();
        let mut index = repo.index().unwrap();
        index.read(true).unwrap();
        let plan = vec![PlannedChange {
            old_path: "f.txt".to_string(),
            target: Some("f.txt".to_string()),
        }];
        reconcile_index(&mut index, &committed_tree, &plan).unwrap();

        // The index holds the COMMITTED content (X); the newer worktree content
        // (Y) stays an unstaged edit rather than becoming staged.
        assert_eq!(staged_blob(&dir, "f.txt").as_deref(), Some("X\n"), "index keeps committed blob");
        let st = get_status(d).unwrap();
        let f = file(&st, "f.txt").expect("f.txt listed after the post-commit edit");
        assert!(!f.staged, "newer worktree content is unstaged, not staged");
        assert_eq!(f.status, "modified");
        assert_eq!(read(&dir, "f.txt"), "Y\n", "worktree edit intact");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Issue #4: filenames with glob characters are handled literally. The bracket
    // file is committed; a sibling that the pathspec "a[1].txt" would glob-match
    // ("a1.txt") is staged and must be left completely alone.
    #[test]
    fn glob_char_filenames_commit_and_reconcile_literally() {
        let dir = scratch("glint-glob");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "seed.txt", "seed\n");
        sh(&dir, &["add", "seed.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        write(&dir, "a[1].txt", "bracket\n");
        write(&dir, "a1.txt", "sibling\n");
        sh(&dir, &["add", "a1.txt"]); // unrelated staged sibling, unchecked

        commit(d, &["a[1].txt".to_string()], "add bracket file", "").unwrap();

        assert!(head_has(&dir, "a[1].txt"), "the glob-named file is committed");
        assert!(!head_has(&dir, "a1.txt"), "the sibling must not be committed");
        assert_eq!(head_blob(&dir, "a[1].txt"), "bracket\n");
        let st = get_status(d).unwrap();
        assert!(file(&st, "a1.txt").expect("a1.txt listed").staged, "sibling stays staged");
        assert_eq!(staged_blob(&dir, "a1.txt").as_deref(), Some("sibling\n"), "sibling content intact");
        assert!(file(&st, "a[1].txt").is_none(), "committed glob-named file is clean");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Issue #4: a delete plus an unrelated add (different content, so git infers
    // no rename) commits both sides when both are selected.
    #[test]
    fn delete_add_pair_commits_both_sides() {
        let dir = scratch("glint-delete-add");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "gone.txt", "old content here\n");
        sh(&dir, &["add", "gone.txt"]);
        sh(&dir, &["commit", "-qm", "base"]);

        std::fs::remove_file(dir.join("gone.txt")).unwrap();
        write(&dir, "fresh.txt", "totally different\n");

        let selected: Vec<String> = get_status(d).unwrap().files.iter().map(|f| f.path.clone()).collect();
        assert!(
            selected.iter().any(|p| p == "gone.txt") && selected.iter().any(|p| p == "fresh.txt"),
            "both sides listed as separate changes: {selected:?}"
        );
        commit(d, &selected, "delete and add", "").unwrap();

        assert!(!head_has(&dir, "gone.txt"), "deletion recorded");
        assert!(head_has(&dir, "fresh.txt"), "addition recorded");
        assert!(get_status(d).unwrap().files.is_empty(), "clean after commit");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Issue #2: attaching a commit built on a stale parent is refused (the ref
    // moved in another process) and does NOT clobber the newer commit.
    #[test]
    fn stale_parent_is_refused_and_head_unmoved() {
        let dir = scratch("glint-cas");
        init_repo(&dir);
        write(&dir, "base.txt", "base\n");
        sh(&dir, &["add", "base.txt"]);
        sh(&dir, &["commit", "-qm", "c0"]);
        let repo = Repository::open(&dir).unwrap();
        let c0 = repo.head().unwrap().peel_to_commit().unwrap(); // what we build on

        // Another "process" advances HEAD so the ref no longer matches c0.
        write(&dir, "base.txt", "base2\n");
        sh(&dir, &["commit", "-qam", "c1"]);
        let c1 = repo.head().unwrap().target().unwrap();

        // Build a commit object on the STALE c0 and try to attach it.
        let tree = c0.tree().unwrap();
        let sig = repo.signature().unwrap();
        let new_oid = repo.commit(None, &sig, &sig, "stale", &tree, &[&c0]).unwrap();
        let head_ref_name = Some(repo.head().unwrap().name().unwrap().to_string());
        let e = attach_commit(&repo, &head_ref_name, Some(&c0), new_oid, "stale").unwrap_err();

        assert!(
            e.contains("another Git process") || e.contains("refresh"),
            "actionable conflict message: {e}"
        );
        // HEAD was not clobbered: it still points at c1.
        let repo2 = Repository::open(&dir).unwrap();
        assert_eq!(repo2.head().unwrap().target().unwrap(), c1, "HEAD not moved by the refused attach");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Issue #3 (backend enabler): a file with both staged and unstaged changes is
    // reported as partially staged so the UI can warn before committing it.
    #[test]
    fn partially_staged_flag_is_reported() {
        let dir = scratch("glint-partial-flag");
        let d = dir.to_str().unwrap();
        init_repo(&dir);
        write(&dir, "p.txt", "one\n");
        write(&dir, "s.txt", "s\n");
        sh(&dir, &["add", "-A"]);
        sh(&dir, &["commit", "-qm", "base"]);

        // p.txt: staged edit plus a further worktree edit (partially staged).
        write(&dir, "p.txt", "ONE\n");
        sh(&dir, &["add", "p.txt"]);
        write(&dir, "p.txt", "ONE\ntwo\n");
        // s.txt: fully staged, no further edit (not partially staged).
        write(&dir, "s.txt", "S\n");
        sh(&dir, &["add", "s.txt"]);

        let st = get_status(d).unwrap();
        assert!(file(&st, "p.txt").expect("p.txt listed").partially_staged, "p.txt is partially staged");
        assert!(!file(&st, "s.txt").expect("s.txt listed").partially_staged, "s.txt is fully staged only");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
