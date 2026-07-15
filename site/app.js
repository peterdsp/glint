// Glint website - theme switching, interactive panel demo, copy-to-clipboard.
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
      const t = window.GLINT_T || ((k) => k);
      btn.classList.add("copied");
      if (label) label.textContent = t("copiedLabel");
      setTimeout(() => {
        btn.classList.remove("copied");
        if (label) label.textContent = t("copyLabel");
      }, 1600);
    });
  });
}

// --- live global-Git counters ---
// Illustrative per-second rates for worldwide Git activity. Each counter is
// seeded to "today so far" (rate x seconds since the viewer's local midnight)
// and then keeps climbing live, so the scale reads as real and the motion is
// visible. Not a live feed - a tangible sense of Git's global throughput.
function startLiveCounters() {
  const nodes = Array.from(document.querySelectorAll(".live-num[data-rate]"));
  if (!nodes.length) return;

  const secondsSinceMidnight = () => {
    const now = new Date();
    const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    return (now - midnight) / 1000;
  };

  const fmt = new Intl.NumberFormat();
  const seedSecs = secondsSinceMidnight();
  const counters = nodes.map((el) => {
    const rate = parseFloat(el.dataset.rate) || 0;
    return { el, rate, seed: rate * seedSecs, last: "" };
  });

  const start = performance.now();
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  const render = (now) => {
    const elapsed = (now - start) / 1000;
    for (const c of counters) {
      const text = fmt.format(Math.floor(c.seed + c.rate * elapsed));
      if (text !== c.last) { c.el.textContent = text; c.last = text; }
    }
  };

  if (reduce) {
    render(performance.now());
    setInterval(() => render(performance.now()), 1000);
  } else {
    const loop = (now) => { render(now); requestAnimationFrame(loop); };
    requestAnimationFrame(loop);
  }
}

// --- live GitHub public-event stream ---
// Real data: GitHub's public events firehose (no auth, CORS-open, 60 req/hr
// per visitor IP). We fetch a batch, then reveal events one at a time for a
// streaming feel, and only refetch when the queue drains - well under the
// limit. Any failure (offline, rate-limited) hides the panel; the estimated
// counters above stand on their own.
function startGithubStream() {
  const wrap = document.getElementById("gh-stream-wrap");
  const list = document.getElementById("gh-stream");
  if (!wrap || !list) return;

  const ENDPOINT = "https://api.github.com/events?per_page=100";
  const REVEAL_MS = 2200;   // one row every ~2.2s
  const MIN_FETCH_MS = 90000; // never refetch faster than 90s (keeps us <60/hr)
  const MAX_ROWS = 7;

  const seen = new Set();
  let queue = [];
  let lastFetch = 0;
  let started = false;

  const t = (k) => (window.GLINT_T ? window.GLINT_T(k) : k);
  const shortBranch = (ref) => (ref || "").replace(/^refs\/(heads|tags)\//, "");

  // Build the "<b>actor</b> verb <ref>" bits for an event, or null to skip it.
  function describe(e) {
    const p = e.payload || {};
    switch (e.type) {
      case "PushEvent":   return { verb: t("ghPushed"), ref: shortBranch(p.ref) };
      case "CreateEvent":
        if (p.ref_type === "repository") return { verb: t("ghNewRepo"), ref: "" };
        if (p.ref_type === "tag")        return { verb: t("ghNewTag"), ref: shortBranch(p.ref) };
        return { verb: t("ghNewBranch"), ref: shortBranch(p.ref) };
      case "DeleteEvent": return { verb: t("ghDeleted"), ref: shortBranch(p.ref) };
      case "PullRequestEvent":
        return { verb: p.action === "closed" && p.pull_request && p.pull_request.merged ? t("ghMerged") : t("ghPR"), ref: "" };
      case "ForkEvent":   return { verb: t("ghForked"), ref: "" };
      case "WatchEvent":  return { verb: t("ghStarred"), ref: "" };
      case "IssuesEvent": return { verb: t("ghIssue"), ref: "" };
      default: return null;
    }
  }

  function relTime(iso) {
    const secs = Math.max(0, Math.round((Date.now() - Date.parse(iso)) / 1000));
    const lang = window.__glintLang || "en";
    try {
      const rtf = new Intl.RelativeTimeFormat(lang, { numeric: "auto", style: "narrow" });
      if (secs < 60) return rtf.format(-secs, "second");
      if (secs < 3600) return rtf.format(-Math.round(secs / 60), "minute");
      return rtf.format(-Math.round(secs / 3600), "hour");
    } catch (e) {
      return secs < 60 ? secs + "s" : Math.round(secs / 60) + "m";
    }
  }

  function reveal() {
    const e = queue.shift();
    if (e) {
      const d = describe(e);
      if (d) {
        const repo = e.repo && e.repo.name ? e.repo.name : "";
        const a = document.createElement("a");
        a.className = "gh-row";
        a.href = "https://github.com/" + repo;
        a.target = "_blank";
        a.rel = "noopener noreferrer";

        const img = document.createElement("img");
        img.className = "gh-av";
        img.src = (e.actor && e.actor.avatar_url) || "";
        img.alt = "";
        img.loading = "lazy";
        img.width = 30; img.height = 30;
        img.referrerPolicy = "no-referrer";
        img.addEventListener("error", () => { img.style.visibility = "hidden"; });

        const body = document.createElement("div");
        body.className = "gh-body";
        const line = document.createElement("div");
        line.className = "gh-line";
        const who = document.createElement("b");
        who.textContent = (e.actor && e.actor.login) || "someone";
        line.append(who, document.createTextNode(" " + d.verb + " "));
        if (d.ref) {
          const ref = document.createElement("span");
          ref.className = "gh-ref";
          ref.textContent = d.ref;
          line.append(ref);
        }
        const repoEl = document.createElement("div");
        repoEl.className = "gh-repo";
        repoEl.textContent = repo;
        body.append(line, repoEl);

        const time = document.createElement("time");
        time.className = "gh-time";
        time.dateTime = e.created_at;
        time.dataset.iso = e.created_at;
        time.textContent = relTime(e.created_at);

        a.append(img, body, time);
        list.prepend(a);
        while (list.children.length > MAX_ROWS) list.lastElementChild.remove();
      }
    }
    maybeFetch();
  }

  function maybeFetch() {
    if (queue.length >= 5) return;
    if (Date.now() - lastFetch < MIN_FETCH_MS) return;
    fetchBatch();
  }

  async function fetchBatch() {
    lastFetch = Date.now();
    try {
      const res = await fetch(ENDPOINT, { headers: { Accept: "application/vnd.github+json" } });
      if (!res.ok) throw new Error("HTTP " + res.status);
      const events = await res.json();
      if (!Array.isArray(events)) throw new Error("bad payload");
      // oldest-first so each revealed row is newer than the last
      const fresh = events
        .filter((e) => e && e.id && !seen.has(e.id) && describe(e))
        .reverse();
      fresh.forEach((e) => seen.add(e.id));
      if (seen.size > 800) seen.clear(); // bound memory over a long session
      queue.push(...fresh);
      if (!started && queue.length) {
        started = true;
        wrap.hidden = false;
      }
      if (!started) { wrap.hidden = true; } // nothing renderable yet - stay hidden
    } catch (e) {
      if (!started) wrap.hidden = true; // first load failed: leave counters alone
      // if already streaming, keep draining whatever is queued and try later
    }
  }

  // refresh visible relative times as rows age
  setInterval(() => {
    list.querySelectorAll(".gh-time").forEach((el) => {
      if (el.dataset.iso) el.textContent = relTime(el.dataset.iso);
    });
  }, 5000);

  // reveal loop, paused while the tab is hidden to save the rate budget
  let timer = null;
  const start = () => { if (!timer) timer = setInterval(reveal, REVEAL_MS); };
  const stop = () => { if (timer) { clearInterval(timer); timer = null; } };
  document.addEventListener("visibilitychange", () => (document.hidden ? stop() : start()));

  fetchBatch().then(() => { if (!document.hidden) start(); });
}

// --- ambient background field: faint Git numbers drifting site-wide ---
// A fixed layer behind all content. Most items are static Git snippets; a
// share are live counters (seeded to "today so far") so the scale figures
// live everywhere, not just the Live section. Kept very low-opacity so text
// stays perfectly readable, and paused while the tab is hidden.
function buildBackgroundField() {
  if (document.querySelector(".bg-field")) return;
  const field = document.createElement("div");
  field.className = "bg-field";
  field.setAttribute("aria-hidden", "true");

  const fmt = new Intl.NumberFormat();
  const now = new Date();
  const midnightSecs = (now - new Date(now.getFullYear(), now.getMonth(), now.getDate())) / 1000;

  const rates = [412, 74000, 186, 26, 5, 33, 903, 1240];
  const phrases = [
    "git push", "git commit", "+42", "-7", "↑5 ↓2", "merged #482",
    "branch main", "stage 3 files", "pull origin", "+118", "fork", "★ 1.2k",
    "HEAD~1", "rebase -i", "origin/main",
  ];
  const hex = () => {
    let s = "";
    for (let i = 0; i < 7; i++) s += "0123456789abcdef"[Math.floor(Math.random() * 16)];
    return s;
  };

  const tickers = [];
  const COUNT = window.innerWidth < 640 ? 34 : 58;
  for (let i = 0; i < COUNT; i++) {
    const el = document.createElement("span");
    el.className = "bg-num";
    el.style.left = Math.round(Math.random() * 94) + "%";
    el.style.top = Math.round(Math.random() * 100) + "%";
    el.style.fontSize = (12 + Math.round(Math.random() * 26)) + "px";
    el.style.setProperty("--o", (0.09 + Math.random() * 0.09).toFixed(3));
    el.style.animationDuration = (14 + Math.random() * 24).toFixed(1) + "s";
    el.style.animationDelay = (-Math.random() * 42).toFixed(1) + "s";
    if (Math.random() < 0.32) el.classList.add("bg-accent");

    const kind = Math.random();
    if (kind < 0.55) {
      const rate = rates[Math.floor(Math.random() * rates.length)];
      el.dataset.rate = String(rate);
      el.dataset.seed = String(Math.floor(rate * midnightSecs));
      el.textContent = fmt.format(Number(el.dataset.seed));
      tickers.push(el);
    } else if (kind < 0.72) {
      el.textContent = phrases[Math.floor(Math.random() * phrases.length)];
    } else {
      el.textContent = hex();
    }
    field.appendChild(el);
  }
  document.body.prepend(field);

  if (!tickers.length) return;
  const start = performance.now();
  const update = () => {
    const elapsed = (performance.now() - start) / 1000;
    for (const el of tickers) {
      el.textContent = fmt.format(Math.floor(Number(el.dataset.seed) + Number(el.dataset.rate) * elapsed));
    }
  };
  update();
  setInterval(() => { if (!document.hidden) update(); }, 140);
}

// --- boot ---
buildBackgroundField();
buildMenubarSwatches();
buildDemoSwatches();
buildThemeList();
wireDemoPanels();
wireCopy();
startLiveCounters();
startGithubStream();

let saved = "aurora";
try { saved = localStorage.getItem("glint.site.theme") || "aurora"; } catch (e) {}
applyTheme(saved);

const yearEl = document.getElementById("year");
if (yearEl) yearEl.textContent = String(new Date().getFullYear());

// --- live menu-bar clock: the viewer's local time, in their locale format ---
function startClock() {
  const el = document.querySelector(".mb-clock");
  if (!el) return;
  const tick = () => {
    try {
      // 24-hour, no AM/PM - matches the macOS menu bar. Local timezone.
      el.textContent = new Date().toLocaleTimeString([], {
        hour: "numeric",
        minute: "2-digit",
        hour12: false,
      });
    } catch (e) {
      /* leave the fallback text */
    }
  };
  tick();
  setInterval(tick, 15000);
}
startClock();
