// Glint frontend controller: theme switching + Git status rendering.
// Talks to the Rust core over Tauri IPC (window.__TAURI__, enabled via
// withGlobalTauri). When run outside Tauri (e.g. a plain browser) or when no
// repo is set, it falls back to sample data so the UI is always populated.

const THEMES = window.GLINT_THEMES;
const invoke = window.__TAURI__?.core?.invoke;

const SAMPLE = {
  repo: "peterdsp / glint",
  branch: "main",
  ahead: 5,
  behind: 2,
  files: [
    { path: "src/menu-bar/panel.tsx", status: "modified", staged: true, added: 42, removed: 0 },
    { path: "src/themes/liquid-glass.css", status: "added", staged: true, added: 118, removed: 0 },
    { path: "docs/architecture.md", status: "modified", staged: false, added: 0, removed: 7 },
  ],
};

// The currently rendered status; the commit path reads staged files from it.
let current = SAMPLE;

function repoPath() {
  try {
    return localStorage.getItem("glint.repo");
  } catch {
    return null;
  }
}

function applyTheme(key) {
  const theme = THEMES[key] || THEMES.aurora;
  for (const [prop, val] of Object.entries(theme.vars)) {
    document.documentElement.style.setProperty(prop, val);
  }
  document.documentElement.dataset.theme = key;
  document.getElementById("theme-name").textContent = theme.label;
  document.querySelectorAll(".swatch").forEach((b) => {
    b.style.boxShadow =
      b.dataset.key === key
        ? `0 0 0 2px var(--tint), 0 0 0 4px ${THEMES[key].vars["--accent"]}`
        : "0 0 0 1px rgba(0,0,0,0.15)";
  });
  try {
    localStorage.setItem("glint.theme", key);
  } catch {}
}

function buildSwatches() {
  const wrap = document.getElementById("swatches");
  wrap.innerHTML = "";
  for (const [key, theme] of Object.entries(THEMES)) {
    const b = document.createElement("button");
    b.className = "swatch";
    b.dataset.key = key;
    b.style.background = theme.swatch;
    b.setAttribute("aria-label", `${theme.label || key} theme`);
    b.onclick = () => applyTheme(key);
    wrap.appendChild(b);
  }
}

// Merge user-authored themes from disk into the picker (issue #6). No-op in a
// plain browser (no Tauri) or when the themes/ folder is empty.
async function loadDiskThemes() {
  if (!invoke) return;
  try {
    const list = await invoke("load_themes");
    if (!Array.isArray(list) || list.length === 0) return;
    for (const t of list) {
      if (!t || !t.key || !t.vars) continue;
      THEMES[t.key] = {
        label: t.label || t.key,
        swatch: t.swatch || t.vars["--accent"] || "#888888",
        vars: t.vars,
      };
    }
    buildSwatches();
    let key = "aurora";
    try {
      key = localStorage.getItem("glint.theme") || "aurora";
    } catch {}
    applyTheme(key); // re-apply so the selection ring lands correctly
  } catch (e) {
    console.warn("load_themes failed:", e);
  }
}

function deltaEl(f) {
  const wrap = document.createElement("span");
  wrap.className = "file-delta";
  if (f.added) {
    const s = document.createElement("span");
    s.className = "file-stat add";
    s.textContent = `+${f.added}`;
    wrap.appendChild(s);
  }
  if (f.removed) {
    const s = document.createElement("span");
    s.className = "file-stat del";
    s.textContent = `−${f.removed}`;
    wrap.appendChild(s);
  }
  return wrap;
}

function render(status) {
  current = status;
  document.getElementById("repo-name").textContent = status.repo || "—";
  document.getElementById("branch").textContent = status.branch;
  document.getElementById("commit-branch").textContent = status.branch;
  document.getElementById("ahead").textContent = status.ahead;
  document.getElementById("behind").textContent = status.behind;

  const n = status.files.length;
  document.getElementById("files-count").textContent =
    `${n} changed file${n === 1 ? "" : "s"}`;

  const list = document.getElementById("files");
  list.innerHTML = "";
  for (const f of status.files) {
    const li = document.createElement("li");
    li.className = "file";
    const check = document.createElement("span");
    check.className = "check " + (f.staged ? "on" : "off");
    if (f.staged) check.innerHTML = '<i class="ti ti-check"></i>';
    check.onclick = () => {
      f.staged = !f.staged;
      render(status);
    };
    const path = document.createElement("span");
    path.className = "file-path";
    path.textContent = f.path;
    path.title = "Open diff";
    path.onclick = () => openDiff(f.path);
    li.append(check, path, deltaEl(f));
    list.appendChild(li);
  }

  refreshSyncButtons();
}

// Pull/push are only meaningful when there's something to move.
function refreshSyncButtons() {
  const sync = document.getElementById("sync");
  const pull = document.getElementById("pull-btn");
  const push = document.getElementById("push-btn");
  if (sync) sync.disabled = false;
  if (pull) pull.disabled = !current.behind;
  if (push) push.disabled = !current.ahead;
}

function statusToView(path, s) {
  return {
    repo: path.split("/").filter(Boolean).slice(-2).join(" / "),
    branch: s.branch,
    ahead: s.ahead,
    behind: s.behind,
    files: s.files, // { path, status, staged, added, removed }
  };
}

let toastTimer = null;
function showToast(msg, kind = "") {
  const el = document.getElementById("toast");
  if (!el) return;
  el.textContent = msg;
  el.className = "toast" + (kind ? " " + kind : "");
  el.hidden = false;
  clearTimeout(toastTimer);
  if (kind !== "busy") {
    toastTimer = setTimeout(() => {
      el.hidden = true;
    }, kind === "err" ? 4200 : 2200);
  }
}

// PR + CI badge next to the branch (issue #3). Hidden unless there's an open
// PR for the current branch; the CI dot is colored by check-run status.
async function loadPrStatus() {
  const pill = document.getElementById("pr-pill");
  if (!pill) return;
  pill.hidden = true;
  const path = repoPath();
  if (!invoke || !path) return;
  try {
    const pr = await invoke("pr_status", { path });
    if (!pr) return;
    pill.textContent = `#${pr.number}`;
    pill.className = "pr-pill checks-" + (pr.checks || "none");
    pill.title = `${pr.draft ? "Draft · " : ""}${pr.title} — CI: ${pr.checks}`;
    pill.onclick = () => {
      if (pr.url) invoke("open_url", { url: pr.url }).catch(() => {});
    };
    pill.hidden = false;
  } catch (e) {
    console.warn("pr_status failed:", e);
  }
}

// Open the diff pop-out for a file (a separate resizable window).
function openDiff(file) {
  const path = repoPath();
  if (!invoke || !path) {
    showToast("Connect a repo to see diffs", "err");
    return;
  }
  invoke("open_diff", { path, file }).catch((e) =>
    showToast(String(e && e.message ? e.message : e), "err")
  );
}

// Shared runner for the networked commands (fetch/pull/push). Each returns a
// fresh RepoStatus, which we render; errors from Rust are already user-facing.
async function runSync(cmd, busyMsg, okMsg) {
  const path = repoPath();
  if (!invoke || !path) {
    showToast("Connect a repo first — the ⌄ button up top", "err");
    return;
  }
  showToast(busyMsg, "busy");
  ["sync", "pull-btn", "push-btn"].forEach((id) => {
    const b = document.getElementById(id);
    if (b) b.disabled = true;
  });
  try {
    const s = await invoke(cmd, { path });
    render(statusToView(path, s));
    showToast(typeof okMsg === "function" ? okMsg(s) : okMsg);
  } catch (e) {
    showToast(String(e && e.message ? e.message : e), "err");
    refreshSyncButtons();
  }
}

async function loadStatus() {
  const path = repoPath();
  if (invoke && path) {
    try {
      const s = await invoke("get_status", { path });
      render(statusToView(path, s));
      loadPrStatus(); // fire-and-forget PR + CI badge
      return;
    } catch (e) {
      console.warn("get_status failed, using sample:", e);
    }
  }
  render(SAMPLE);
}

async function doCommit() {
  const summaryEl = document.getElementById("commit-summary");
  const descEl = document.getElementById("commit-desc");
  const summary = summaryEl.value.trim();
  const path = repoPath();

  if (!invoke || !path) {
    console.log("commit (demo — no repo set):", summary);
    return;
  }
  if (!summary) {
    summaryEl.focus();
    return;
  }
  const files = (current.files || []).filter((f) => f.staged).map((f) => f.path);
  if (!files.length) return;

  const btn = document.getElementById("commit-btn");
  btn.style.opacity = "0.6";
  try {
    await invoke("commit", { path, files, summary, description: descEl.value });
    summaryEl.value = "";
    descEl.value = "";
    await loadStatus();
  } catch (e) {
    console.warn("commit failed:", e);
  } finally {
    btn.style.opacity = "";
  }
}

function wireActions() {
  document.getElementById("switch-repo").onclick = () => {
    const path = window.prompt("Path to a Git repository:");
    if (path) {
      try {
        localStorage.setItem("glint.repo", path);
      } catch {}
      loadStatus();
    }
  };
  document.getElementById("sync").onclick = () =>
    runSync("fetch", "Fetching…", (s) =>
      s.behind || s.ahead ? `${s.behind} to pull · ${s.ahead} to push` : "Up to date"
    );
  document.getElementById("pull-btn").onclick = () =>
    runSync("pull", "Pulling…", "Pulled");
  document.getElementById("push-btn").onclick = () =>
    runSync("push", "Pushing…", "Pushed");
  document.getElementById("commit-btn").onclick = doCommit;
}

buildSwatches();
applyTheme(localStorage.getItem("glint.theme") || "aurora");
wireActions();
loadStatus();
loadDiskThemes();
