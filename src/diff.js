// Glint diff pop-out controller (issue #4). Reads its target from an init
// script (window.__GLINT_DIFF__) on first load, and from window.__glintShowDiff
// when the panel reuses an already-open window. Falls back to a sample diff in
// a plain browser so the view is never blank.

const invoke = window.__TAURI__?.core?.invoke;
const THEMES = window.GLINT_THEMES;

const SAMPLE_DIFF = {
  file: "src/menu-bar/panel.tsx",
  binary: false,
  hunks: [
    {
      header: "@@ -11,8 +11,10 @@ export function Panel() {",
      lines: [
        { kind: "ctx", old_ln: 11, new_ln: 11, content: "  const [open, setOpen] = useState(false);" },
        { kind: "ctx", old_ln: 12, new_ln: 12, content: "" },
        { kind: "del", old_ln: 13, new_ln: null, content: "  const theme = useTheme();" },
        { kind: "add", old_ln: null, new_ln: 13, content: "  const theme = useTheme('aurora');" },
        { kind: "add", old_ln: null, new_ln: 14, content: "  const glass = useVibrancy();" },
        { kind: "ctx", old_ln: 14, new_ln: 15, content: "" },
        { kind: "ctx", old_ln: 15, new_ln: 16, content: "  return (" },
        { kind: "del", old_ln: 16, new_ln: null, content: "    <div className=\"panel\">" },
        { kind: "add", old_ln: null, new_ln: 17, content: "    <div className=\"panel glass\" data-theme={theme}>" },
        { kind: "ctx", old_ln: 17, new_ln: 18, content: "      <Header />" },
      ],
    },
  ],
};

function applyTheme() {
  let key = "aurora";
  try {
    key = localStorage.getItem("glint.theme") || "aurora";
  } catch {}
  const theme = (THEMES && (THEMES[key] || THEMES.aurora)) || null;
  if (theme) {
    for (const [prop, val] of Object.entries(theme.vars)) {
      document.documentElement.style.setProperty(prop, val);
    }
  }
  document.documentElement.dataset.theme = key;
}

function gutter(n) {
  const el = document.createElement("span");
  el.className = "g";
  el.textContent = n == null ? "" : String(n);
  return el;
}

function renderDiff(d) {
  const fileEl = document.getElementById("file");
  fileEl.textContent = d.file || "-";
  document.title = "Glint - " + (d.file || "Diff");

  let adds = 0;
  let dels = 0;
  for (const h of d.hunks || []) {
    for (const l of h.lines) {
      if (l.kind === "add") adds++;
      else if (l.kind === "del") dels++;
    }
  }
  document.getElementById("adds").textContent = `+${adds}`;
  document.getElementById("dels").textContent = `-${dels}`;

  const body = document.getElementById("body");
  body.innerHTML = "";

  if (d.binary) {
    body.innerHTML = `<div class="empty">${window.t ? window.t("binaryFile") : "Binary file - no line diff."}</div>`;
    return;
  }
  if (!d.hunks || d.hunks.length === 0) {
    body.innerHTML = `<div class="empty">${window.t ? window.t("noChanges") : "No changes in this file."}</div>`;
    return;
  }

  for (const h of d.hunks) {
    const hunk = document.createElement("section");
    hunk.className = "hunk";

    const head = document.createElement("div");
    head.className = "hunk-head";
    head.textContent = h.header;
    hunk.appendChild(head);

    for (const l of h.lines) {
      const row = document.createElement("div");
      row.className = "dl " + l.kind;

      const sign = document.createElement("span");
      sign.className = "s";
      sign.textContent = l.kind === "add" ? "+" : l.kind === "del" ? "-" : "";

      const content = document.createElement("span");
      content.className = "c";
      content.textContent = l.content;

      row.append(gutter(l.old_ln), gutter(l.new_ln), sign, content);
      hunk.appendChild(row);
    }
    body.appendChild(hunk);
  }
}

async function loadDiff(repo, file) {
  document.getElementById("file").textContent = file;
  document.title = "Glint - " + file;

  if (!invoke) {
    renderDiff(SAMPLE_DIFF);
    return;
  }
  const body = document.getElementById("body");
  body.innerHTML = `<div class="empty">${window.t ? window.t("loadingDiff") : "Loading diff..."}</div>`;
  try {
    const d = await invoke("diff", { path: repo, file });
    renderDiff(d);
  } catch (e) {
    body.innerHTML = "";
    const el = document.createElement("div");
    el.className = "empty err";
    el.textContent = String(e && e.message ? e.message : e);
    body.appendChild(el);
  }
}

// Called by the panel (via eval) when it reuses an open diff window.
window.__glintShowDiff = (repo, file) => loadDiff(repo, file);

applyTheme();
if (window.__GLINT_DIFF__) {
  loadDiff(window.__GLINT_DIFF__.repo, window.__GLINT_DIFF__.file);
} else {
  renderDiff(SAMPLE_DIFF); // plain-browser preview
}
