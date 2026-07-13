// Glint frontend controller: theme switching + Git status rendering.
// Talks to the Rust core over Tauri IPC (window.__TAURI__, enabled via
// withGlobalTauri). When run outside Tauri (e.g. a plain browser) or when no
// repo is set, it falls back to sample data so the UI is always populated.

const THEMES = window.GLINT_THEMES;
const invoke = window.__TAURI__?.core?.invoke;

const SAMPLE = {
  repo: "peterdsp / desktop",
  branch: "menu-bar-rewrite",
  ahead: 5,
  behind: 2,
  files: [
    { path: "src/menu-bar/panel.tsx", status: "modified", staged: true, delta: "+42" },
    { path: "src/themes/liquid-glass.css", status: "added", staged: true, delta: "+118" },
    { path: "docs/architecture.md", status: "modified", staged: false, delta: "−7" },
  ],
};

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
    b.setAttribute("aria-label", `${key} theme`);
    b.onclick = () => applyTheme(key);
    wrap.appendChild(b);
  }
}

function deltaClass(delta) {
  return delta && delta.trim().startsWith("−") ? "del" : "add";
}

function render(status) {
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
    const stat = document.createElement("span");
    stat.className = "file-stat " + deltaClass(f.delta);
    stat.textContent = f.delta || "";
    li.append(check, path, stat);
    list.appendChild(li);
  }
}

async function loadStatus() {
  const path = (() => {
    try {
      return localStorage.getItem("glint.repo");
    } catch {
      return null;
    }
  })();

  if (invoke && path) {
    try {
      const s = await invoke("get_status", { path });
      render({
        repo: path.split("/").filter(Boolean).slice(-2).join(" / "),
        branch: s.branch,
        ahead: s.ahead,
        behind: s.behind,
        files: s.files.map((f) => ({ ...f, delta: "" })),
      });
      return;
    } catch (e) {
      console.warn("get_status failed, using sample:", e);
    }
  }
  render(SAMPLE);
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
  document.getElementById("sync").onclick = () => loadStatus();
  document.getElementById("commit-btn").onclick = () => {
    // TODO: wire to a Rust `commit` command (staged files + summary/description).
    console.log("commit:", document.getElementById("commit-summary").value);
  };
}

buildSwatches();
applyTheme(localStorage.getItem("glint.theme") || "aurora");
wireActions();
loadStatus();
