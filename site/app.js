// Glint website — theme switching, interactive panel demo, copy-to-clipboard.
// Themes mirror the app's src/themes.js: pick one and the whole page + the
// demo panels retint live. No framework, no build step.

const THEMES = {
  aurora:   { name: "Aurora",   desc: "Translucent, cool",  swatch: "#8ea8ff" },
  midnight: { name: "Midnight", desc: "Deep, glassy dark",  swatch: "#1f2333" },
  sunset:   { name: "Sunset",   desc: "Warm, vivid",        swatch: "#ff9a6a" },
  forest:   { name: "Forest",   desc: "Calm, natural",      swatch: "#5fb8a0" },
  graphite: { name: "Graphite", desc: "Quiet, neutral",     swatch: "#c2c5cd" },
};
const KEYS = Object.keys(THEMES);
const root = document.documentElement;

function currentTheme() {
  return root.dataset.theme && THEMES[root.dataset.theme] ? root.dataset.theme : "aurora";
}

function applyTheme(key) {
  if (!THEMES[key]) key = "aurora";
  root.dataset.theme = key;
  // reflect selection state across every control
  document.querySelectorAll("[data-theme-btn]").forEach((b) => {
    b.setAttribute("aria-pressed", String(b.dataset.themeBtn === key));
  });
  try { localStorage.setItem("glint.site.theme", key); } catch (e) { /* private mode */ }
}

// --- menu-bar swatches ---
function buildMenubarSwatches() {
  const wrap = document.getElementById("swatches");
  if (!wrap) return;
  KEYS.forEach((key) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "sw";
    b.dataset.themeBtn = key;
    b.style.background = THEMES[key].swatch;
    b.title = THEMES[key].name;
    b.setAttribute("aria-label", `${THEMES[key].name} theme`);
    b.addEventListener("click", () => applyTheme(key));
    wrap.appendChild(b);
  });
}

// --- in-panel demo swatches ---
function buildDemoSwatches() {
  const wrap = document.getElementById("demo-swatches");
  if (!wrap) return;
  KEYS.forEach((key) => {
    const b = document.createElement("button");
    b.type = "button";
    b.dataset.themeBtn = key;
    b.style.background = THEMES[key].swatch;
    b.setAttribute("aria-label", `${THEMES[key].name} theme`);
    b.addEventListener("click", () => applyTheme(key));
    wrap.appendChild(b);
  });
}

// --- themes section: labelled rows ---
function buildThemeList() {
  const list = document.getElementById("theme-list");
  if (!list) return;
  KEYS.forEach((key) => {
    const t = THEMES[key];
    const b = document.createElement("button");
    b.type = "button";
    b.className = "theme-row";
    b.dataset.themeBtn = key;
    b.innerHTML =
      `<span class="dot" style="background:${t.swatch}"></span>` +
      `<span><span class="tr-name">${t.name}</span><br><span class="tr-desc">${t.desc}</span></span>` +
      `<span class="tr-check" aria-hidden="true">` +
      `<svg viewBox="0 0 24 24" width="18" height="18"><path fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" d="M5 12l5 5 9-11"/></svg></span>`;
    b.addEventListener("click", () => applyTheme(key));
    list.appendChild(b);
  });
}

// --- interactive demo panels: toggle staged files + sync spin ---
function wireDemoPanels() {
  document.querySelectorAll(".gpanel[data-demo] .gp-file").forEach((row) => {
    row.addEventListener("click", () => {
      const check = row.querySelector(".gp-check");
      const on = check.classList.toggle("on");
      check.classList.toggle("off", !on);
      check.innerHTML = on
        ? '<svg viewBox="0 0 24 24"><path fill="none" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" d="M5 12l5 5 9-11"/></svg>'
        : "";
    });
  });
  document.querySelectorAll(".gpanel[data-demo] .gp-syncbtn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const icon = btn.querySelector("svg");
      icon.animate(
        [{ transform: "rotate(0)" }, { transform: "rotate(360deg)" }],
        { duration: 600, easing: "cubic-bezier(0.4,0,0.2,1)" }
      );
    });
  });
}

// --- copy install command ---
function wireCopy() {
  document.querySelectorAll(".copy-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const text = btn.dataset.copy || "";
      const label = btn.querySelector(".copy-label");
      try {
        await navigator.clipboard.writeText(text);
      } catch (e) {
        // fallback for older webviews
        const ta = document.createElement("textarea");
        ta.value = text; document.body.appendChild(ta); ta.select();
        try { document.execCommand("copy"); } catch (_) {}
        ta.remove();
      }
      btn.classList.add("copied");
      const prev = label ? label.textContent : "";
      if (label) label.textContent = "Copied";
      setTimeout(() => {
        btn.classList.remove("copied");
        if (label) label.textContent = prev || "Copy";
      }, 1600);
    });
  });
}

// --- boot ---
buildMenubarSwatches();
buildDemoSwatches();
buildThemeList();
wireDemoPanels();
wireCopy();

let saved = "aurora";
try { saved = localStorage.getItem("glint.site.theme") || "aurora"; } catch (e) {}
applyTheme(saved);

const yearEl = document.getElementById("year");
if (yearEl) yearEl.textContent = String(new Date().getFullYear());
