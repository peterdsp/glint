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
    b.style.setProperty("--sw", THEMES[key].swatch);
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

// --- the live panel: its files, its counters and the controls beside it ---
// The switches on the right and the rows in the panel are the same state, so
// either one stages a file, and the commit button counts what is staged.
function wireDemoPanels() {
  const panel = document.querySelector(".gpanel[data-demo]");
  if (!panel) return;
  const t = (k) => (window.GLINT_T ? window.GLINT_T(k) : k);
  const rows = Array.from(panel.querySelectorAll(".gp-file"));
  const state = { pull: 2, push: 5, staged: rows.map((r) => r.querySelector(".gp-check").classList.contains("on")) };

  function render() {
    rows.forEach((row, i) => {
      row.querySelector(".gp-check").classList.toggle("on", state.staged[i]);
      row.setAttribute("aria-checked", String(state.staged[i]));
    });
    document.querySelectorAll("[data-toggle]").forEach((sw) => {
      sw.setAttribute("aria-checked", String(state.staged[+sw.dataset.toggle]));
    });
    const n = state.staged.filter(Boolean).length;
    const commit = panel.querySelector('[data-gp="commit"]');
    if (commit) {
      commit.textContent = n === 0 ? t("gpNothing") : n === 1 ? t("gpCommit1") : t("gpCommitN").replace("{n}", n);
      commit.classList.toggle("is-empty", n === 0);
    }
    const changed = panel.querySelector('[data-gp="changed"]');
    if (changed) changed.textContent = rows.length === 1 ? t("gpChanged1") : t("gpChanged").replace("{n}", rows.length);
    ["pull", "push"].forEach((k) => {
      panel.querySelectorAll('[data-gp="' + k + '"]').forEach((el) => (el.textContent = state[k]));
      document.querySelectorAll('[data-gp="' + k + '-out"]').forEach((el) => (el.textContent = state[k]));
    });
  }

  rows.forEach((row, i) => {
    row.setAttribute("role", "checkbox");
    row.tabIndex = 0;
    const flip = () => { state.staged[i] = !state.staged[i]; render(); };
    row.addEventListener("click", flip);
    row.addEventListener("keydown", (e) => { if (e.key === " " || e.key === "Enter") { e.preventDefault(); flip(); } });
  });
  document.querySelectorAll("[data-toggle]").forEach((sw) => {
    sw.addEventListener("click", () => { const i = +sw.dataset.toggle; state.staged[i] = !state.staged[i]; render(); });
  });
  document.querySelectorAll("[data-step]").forEach((b) => {
    b.addEventListener("click", () => {
      const k = b.dataset.step;
      state[k] = Math.max(0, Math.min(99, state[k] + Number(b.dataset.by)));
      render();
    });
  });
  panel.querySelectorAll(".gp-syncbtn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const icon = btn.querySelector("svg");
      if (icon.animate) icon.animate([{ transform: "rotate(0)" }, { transform: "rotate(360deg)" }], { duration: 600, easing: "cubic-bezier(0.4,0,0.2,1)" });
      // a sync pulls what was waiting and pushes what was ahead
      setTimeout(() => { state.pull = 0; state.push = 0; render(); }, 600);
    });
  });
  document.addEventListener("glint:lang", render);
  render();
}

// --- entrances below the fold: each tile rises as it enters ---
function wireReveal() {
  const els = Array.from(document.querySelectorAll(".reveal"));
  if (!("IntersectionObserver" in window) || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
    els.forEach((el) => el.classList.add("in"));
    return;
  }
  const io = new IntersectionObserver((entries) => {
    entries.forEach((e) => {
      if (!e.isIntersecting) return;
      const sibs = Array.from(e.target.parentNode.children).filter((x) => x.classList.contains("reveal"));
      e.target.style.setProperty("--i", Math.min(sibs.indexOf(e.target), 6));
      e.target.classList.add("in");
      io.unobserve(e.target);
    });
  }, { rootMargin: "0px 0px -6% 0px", threshold: 0.01 });
  els.forEach((el) => io.observe(el));
  // the entrance is a courtesy; the content is not
  setTimeout(() => els.forEach((el) => el.classList.add("in")), 4000);
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
        const item = document.createElement("li");
        item.append(a);
        list.prepend(item);
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

// --- boot ---
buildMenubarSwatches();
buildDemoSwatches();
buildThemeList();
wireDemoPanels();
wireReveal();
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

// Support channel: offer GitHub issues only while the repository is public.
// The GitHub API answers 200 for a public repository and 404 for a private one;
// on any other answer the page keeps the wording it shipped with.
(function () {
  const hosts = document.querySelectorAll("[data-repo-channel]");
  if (!hosts.length || !window.fetch) return;
  hosts.forEach((host) => {
    const repo = host.getAttribute("data-repo-channel");
    const key = "repo-public:" + repo;
    let cached = null;
    try { cached = JSON.parse(localStorage.getItem(key) || "null"); } catch (e) {}
    const apply = (isPublic) => {
      host.querySelectorAll("[data-when]").forEach((el) => {
        el.hidden = (el.getAttribute("data-when") === "public") !== isPublic;
      });
    };
    if (cached && Date.now() - cached.t < 3600000) { apply(cached.v); return; }
    fetch("https://api.github.com/repos/" + repo, { headers: { accept: "application/vnd.github+json" } }).then((res) => {
      if (res.status !== 200 && res.status !== 404) return;
      const isPublic = res.status === 200;
      try { localStorage.setItem(key, JSON.stringify({ v: isPublic, t: Date.now() })); } catch (e) {}
      apply(isPublic);
    }).catch(() => {});
  });
})();
