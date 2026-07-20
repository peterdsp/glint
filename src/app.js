// Glint frontend controller: theme switching + Git status rendering.
// Talks to the Rust core over Tauri IPC (window.__TAURI__, enabled via
// withGlobalTauri). Until a repository is connected it shows the onboarding
// card; there is no placeholder repo data.

const THEMES = window.GLINT_THEMES;
const invoke = window.__TAURI__?.core?.invoke;

// The currently rendered status; the commit path reads staged files from it.
let current = { files: [] };

// Sandboxed Mac App Store build. Set from the Rust `is_app_store` command
// before boot. When true, the license/trial and self-update surfaces are
// removed entirely (Apple gates the purchase and pushes updates; shipping
// either violates guidelines 3.1.1 and 2.4.5). Defaults to false so the direct
// build - and a plain-browser preview - behave normally.
let isAppStore = false;

// Strip the license and self-update UI from the DOM. Not merely hidden: the
// nodes are removed so there is no license-key entry or update check anywhere
// in the App Store build.
function stripStoreProhibitedUI() {
  for (const id of [
    "license-gate",       // trial-expired purchase/key overlay
    "trial-bar",          // "N days left" banner
    "set-license-section",// Settings > License (buy + key input)
    "set-updates-section",// Settings > Updates (check now)
  ]) {
    const el = document.getElementById(id);
    if (el) el.remove();
  }
}

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
    s.textContent = `-${f.removed}`;
    wrap.appendChild(s);
  }
  return wrap;
}

function render(status) {
  current = status;
  document.getElementById("repo-name").textContent = status.repo || "-";
  document.getElementById("branch").textContent = status.branch;
  document.getElementById("commit-btn").textContent = t("commitTo", { branch: status.branch });
  document.getElementById("ahead").textContent = status.ahead;
  document.getElementById("behind").textContent = status.behind;

  const n = status.files.length;
  document.getElementById("files-count").textContent =
    n === 1 ? t("changedFile") : t("changedFiles", { n });

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
    path.title = t("openDiff");
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

// Ko-fi page where licenses are sold (direct build). Update to your handle.
const KOFI_URL = "https://ko-fi.com/peterdsp";

function openKofi() {
  if (invoke) invoke("open_url", { url: KOFI_URL }).catch(() => {});
}

async function activateKey(inputId, msgId) {
  const input = document.getElementById(inputId);
  const msg = document.getElementById(msgId);
  const key = (input.value || "").trim();
  if (!key) {
    input.focus();
    return;
  }
  if (msg) msg.textContent = t("activating");
  try {
    const s = await invoke("activate_license", { key });
    if (s && s.state === "licensed") {
      if (msg) msg.textContent = t("activated");
      input.value = "";
      setTimeout(loadLicense, 600);
    } else if (msg) {
      msg.textContent = t("keyInvalid");
    }
  } catch (e) {
    if (msg) msg.textContent = String(e && e.message ? e.message : e);
  }
}

// Trial / license state drives the gate (base screen) and the settings panel.
// In a plain browser (no Tauri) everything is unlocked.
// Show or hide the in-app purchase controls (the Ko-fi buy button and the
// license-key input/activate). These belong to the direct build's trial gate
// only. When the app is already licensed - which is ALWAYS the case in the App
// Store build, where Apple gates the purchase - they must be hidden: shipping a
// "Get a license on Ko-fi" link inside the App Store binary is an external-
// purchase steering violation (App Store Review Guideline 3.1.1).
function showPurchaseUI(on) {
  for (const id of ["set-buy", "set-key", "set-activate"]) {
    const el = document.getElementById(id);
    if (el) el.hidden = !on;
  }
}

async function loadLicense() {
  // The App Store build has no license/trial concept - the surface is removed.
  if (isAppStore) return;
  const gate = document.getElementById("license-gate");
  const bar = document.getElementById("trial-bar");
  const setLic = document.getElementById("set-license");
  if (!invoke) {
    if (gate) gate.hidden = true;
    if (bar) bar.hidden = true;
    if (setLic) setLic.textContent = t("unlockedDev");
    showPurchaseUI(false);
    return;
  }
  let s;
  try {
    s = await invoke("license_status");
  } catch (e) {
    console.warn("license_status failed:", e);
    return;
  }
  if (!s) return;

  if (s.state === "licensed") {
    if (gate) gate.hidden = true;
    if (bar) bar.hidden = true;
    if (setLic) setLic.textContent = s.email ? t("licensedTo", { email: s.email }) : t("licensedThanks");
    showPurchaseUI(false);
  } else if (s.state === "trial") {
    if (gate) gate.hidden = true;
    if (bar) {
      bar.hidden = false;
      bar.textContent = s.days_left === 1 ? t("trialBanner1") : t("trialBanner", { n: s.days_left });
    }
    if (setLic)
      setLic.textContent = s.days_left === 1 ? t("freeTrialLeft1") : t("freeTrialLeft", { n: s.days_left });
    showPurchaseUI(true);
  } else {
    // expired: block the base panel until a valid key is entered
    if (bar) bar.hidden = true;
    if (gate) gate.hidden = false;
    if (setLic) setLic.textContent = t("trialEndedShort");
    showPurchaseUI(true);
  }
}

function wireLicense() {
  const bar = document.getElementById("trial-bar");
  if (bar) bar.onclick = openKofi;
  const buy = document.getElementById("lg-buy");
  if (buy) buy.onclick = openKofi;
  const act = document.getElementById("lg-activate");
  if (act) act.onclick = () => activateKey("lg-input", "lg-msg");
}

// Language selector (Settings). Switching re-renders dynamic strings too.
function buildLangSelector() {
  const wrap = document.getElementById("lang-row");
  if (!wrap || !window.GLINT_LOCALES) return;
  wrap.innerHTML = "";
  const cur = window.currentLocale();
  for (const l of window.GLINT_LOCALES) {
    const b = document.createElement("button");
    b.className = "lang-btn";
    b.type = "button";
    b.textContent = l.label;
    b.setAttribute("aria-pressed", String(l.code === cur));
    b.onclick = () => changeLocale(l.code);
    wrap.appendChild(b);
  }
}

function changeLocale(code) {
  window.setLocale(code); // updates data-i18n text + persists
  buildLangSelector();
  render(current); // re-render file count + commit button
  loadLicense(); // re-render trial / license strings
}

// Reflect whether a GitHub token is stored, without ever showing the secret.
async function refreshGhStatus() {
  const st = document.getElementById("gh-status");
  if (!st || !invoke) return;
  try {
    const set = await invoke("github_token_set");
    st.textContent = set ? t("ghTokenActive") : t("ghTokenNone");
  } catch (e) {
    st.textContent = t("ghTokenNone");
  }
}

// Settings overlay: theme picker + GitHub + license + updates.
function wireSettings() {
  const panel = document.getElementById("settings");
  const open = document.getElementById("settings-btn");
  const close = document.getElementById("settings-close");
  if (open && panel) open.onclick = () => { panel.hidden = false; refreshGhStatus(); };
  if (close && panel) close.onclick = () => { panel.hidden = true; };

  const buy = document.getElementById("set-buy");
  if (buy) buy.onclick = openKofi;
  const act = document.getElementById("set-activate");
  if (act) act.onclick = () => activateKey("set-key", "set-license");

  // GitHub token: stored in the OS Keychain, never read back into the UI.
  const ghSave = document.getElementById("gh-save");
  if (ghSave)
    ghSave.onclick = async () => {
      if (!invoke) return;
      const field = document.getElementById("gh-token");
      const token = field ? field.value.trim() : "";
      try {
        await invoke("set_github_token", { token });
        if (field) field.value = "";
        refreshGhStatus();
      } catch (e) {
        const st = document.getElementById("gh-status");
        if (st) st.textContent = t("ghTokenError");
      }
    };
  refreshGhStatus();

  const upd = document.getElementById("set-update");
  const msg = document.getElementById("set-update-msg");
  if (upd)
    upd.onclick = async () => {
      if (!invoke) return;
      if (msg) msg.textContent = t("checkingUpdates");
      try {
        const updated = await invoke("update_now");
        if (msg) msg.textContent = updated ? t("updateFound") : t("upToDateVersion");
      } catch (e) {
        if (msg) msg.textContent = t("updateUnavailable");
      }
    };

  // App version.
  if (invoke)
    invoke("app_version")
      .then((v) => {
        const el = document.getElementById("set-version");
        if (el && v) el.textContent = `Glint ${v}`;
      })
      .catch(() => {});
}

// Silent background auto-update on launch (direct build). Downloads, installs,
// and relaunches with no prompts. No-op if the updater isn't configured, on the
// App Store build, or offline - the app just keeps running.
async function autoUpdate() {
  // The App Store build ships no self-updater; Apple delivers updates.
  if (!invoke || isAppStore) return;
  try {
    await invoke("update_now");
  } catch (e) {
    /* not configured / offline - ignore */
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
    pill.title = `${pr.draft ? "Draft · " : ""}${pr.title} - CI: ${pr.checks}`;
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
    showToast(t("connectRepoDiff"), "err");
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
    showToast(t("connectRepo"), "err");
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

// Swap between the onboarding card and the repository view.
function showOnboard(on) {
  const ob = document.getElementById("onboard");
  const rv = document.getElementById("repo-view");
  if (ob) ob.hidden = !on;
  if (rv) rv.hidden = on;
}

async function loadStatus() {
  const path = repoPath();
  if (invoke && path) {
    try {
      const s = await invoke("get_status", { path });
      showOnboard(false);
      render(statusToView(path, s));
      loadPrStatus(); // fire-and-forget PR + CI badge
      return;
    } catch (e) {
      // The saved path is gone or not a Git repo - forget it and re-onboard.
      console.warn("get_status failed:", e);
      try {
        localStorage.removeItem("glint.repo");
      } catch {}
    }
  }
  showOnboard(true);
}

// Native folder picker to connect a repository. Validates the choice by loading
// its status; a non-repo folder is rejected with a clear message.
async function connectRepo() {
  if (!invoke) return;
  let path;
  try {
    path = await invoke("pick_repo");
  } catch (e) {
    return;
  }
  if (!path) return;
  try {
    const s = await invoke("get_status", { path });
    try {
      localStorage.setItem("glint.repo", path);
    } catch {}
    showOnboard(false);
    render(statusToView(path, s));
    loadPrStatus();
  } catch (e) {
    showToast(t("notARepo"), "err");
  }
}

async function doCommit() {
  const summaryEl = document.getElementById("commit-summary");
  const descEl = document.getElementById("commit-desc");
  const summary = summaryEl.value.trim();
  const path = repoPath();

  if (!invoke || !path) {
    console.log("commit (demo - no repo set):", summary);
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
  document.getElementById("switch-repo").onclick = connectRepo;
  const obOpen = document.getElementById("ob-open");
  if (obOpen) obOpen.onclick = connectRepo;
  document.getElementById("sync").onclick = () =>
    runSync("fetch", t("fetching"), (s) =>
      s.behind || s.ahead ? t("syncCounts", { behind: s.behind, ahead: s.ahead }) : t("upToDate")
    );
  document.getElementById("pull-btn").onclick = () =>
    runSync("pull", t("pulling"), t("pulled"));
  document.getElementById("push-btn").onclick = () =>
    runSync("push", t("pushing"), t("pushed"));
  document.getElementById("commit-btn").onclick = doCommit;
}

// Presentation mode (menu bar vs Dock window): detect the saved choice, or
// show the first-run chooser. Switching writes the choice and relaunches.
async function initMode() {
  if (!invoke) return "menubar"; // plain-browser preview
  let mode;
  try {
    mode = await invoke("app_mode");
  } catch {
    mode = "menubar";
  }
  if (mode) {
    document.body.dataset.mode = mode;
    const active = document.getElementById(
      mode === "dock" ? "set-mode-dock" : "set-mode-menubar"
    );
    if (active) active.classList.add("active");
    return mode;
  }
  // First run: show only the chooser (hide the repo view and onboarding so
  // nothing bleeds through until a mode is picked).
  const mp = document.getElementById("modepick");
  const rv = document.getElementById("repo-view");
  const ob = document.getElementById("onboard");
  if (rv) rv.hidden = true;
  if (ob) ob.hidden = true;
  if (mp) mp.hidden = false;
  return null;
}

function wireMode() {
  const pick = (mode) => invoke && invoke("set_app_mode", { mode }).catch(() => {});
  const map = {
    "mp-menubar": "menubar",
    "mp-dock": "dock",
    "set-mode-menubar": "menubar",
    "set-mode-dock": "dock",
  };
  for (const [id, mode] of Object.entries(map)) {
    const el = document.getElementById(id);
    if (el) el.onclick = () => pick(mode);
  }
}

async function boot() {
  // Resolve the build variant first, then strip the App Store-prohibited
  // license/update UI before anything renders.
  if (invoke) {
    try {
      isAppStore = await invoke("is_app_store");
    } catch {
      isAppStore = false;
    }
  }
  if (isAppStore) stripStoreProhibitedUI();

  loadDiskThemes();
  loadLicense();
  autoUpdate();
  const mode = await initMode();
  if (mode === null) return; // first run: only the chooser until a mode is picked
  loadStatus();
}

if (window.applyI18n) window.applyI18n();
buildSwatches();
buildLangSelector();
applyTheme(localStorage.getItem("glint.theme") || "aurora");
wireActions();
wireLicense();
wireSettings();
wireMode();
boot();
