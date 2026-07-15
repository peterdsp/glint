// Glint website - in-browser connection speed test.
// Uses Cloudflare's public speed endpoints (speed.cloudflare.com, CORS-open,
// no auth) to measure download, upload, ping, and jitter, then rates common
// modern-home scenarios. No backend needed - fits the static site.

(function () {
  const CF = "https://speed.cloudflare.com";
  const t = (k) => (window.GLINT_T ? window.GLINT_T(k) : k);

  // rough throughput caps so gigabit lines don't pull hundreds of MB
  const DOWN_CAP_BYTES = 180e6;
  const UP_CAP_BYTES = 70e6;
  const DOWN_MS = 8000;
  const UP_MS = 6000;

  // --- scenario definitions ---------------------------------------------
  // down: [min, recommended] Mbps · up: min Mbps · ping/jitter: max ms
  // Any omitted metric is not binding for that scenario.
  const ICONS = {
    web:   "M4 5h16v11H4zM2 20h20M9 16v4m6-4v4",
    hd:    "M3 5h18v12H3zM3 9h18M8 21h8",
    uhd:   "M4 5h16v14H4zM4 9h16M8 13l2 2 4-4",
    iptv:  "M4 6h16v10H4zM8 20h8M9 9l4 2-4 2z",
    game:  "M7 12h4M9 10v4M15 11h.01M18 13h.01M6 8h12a3 3 0 0 1 3 3l1 6a2 2 0 0 1-3.6 1.4L16 16H8l-1.4 3.4A2 2 0 0 1 3 18l1-7a3 3 0 0 1 3-3Z",
    cam:   "M3 8h11v8H3zM14 11l6-3v8l-6-3M6 8V6a2 2 0 0 1 2-2h2",
    movie: "M4 5h16v14H4zM4 5l3 4m3-4 3 4m3-4 3 4M4 9h16",
    home:  "M3 11l9-7 9 7M5 10v9h14v-9M10 19v-5h4v5",
  };
  const SCN = [
    { key: "web",   name: "spScnWeb",    icon: ICONS.web,   down: [3, 15],   spec: "15 Mbps ↓" },
    { key: "hd",    name: "spScnHd",     icon: ICONS.hd,    down: [5, 12],   spec: "12 Mbps ↓" },
    { key: "uhd",   name: "spScn4k",     icon: ICONS.uhd,   down: [15, 30],  spec: "25–30 Mbps ↓" },
    { key: "iptv",  name: "spScnIptv",   icon: ICONS.iptv,  down: [18, 35],  jitter: 30, spec: "35 Mbps ↓ · low jitter" },
    { key: "game",  name: "spScnGaming", icon: ICONS.game,  down: [20, 40],  ping: 60,   spec: "40 Mbps ↓ · ping <60 ms" },
    { key: "cam",   name: "spScnCam",    icon: ICONS.cam,   up: 12,          spec: "12 Mbps ↑ · 4 cams" },
    { key: "movie", name: "spScnMovie",  icon: ICONS.movie, down: [45, 75],  ping: 80,   spec: "75 Mbps ↓ · ping <80 ms" },
    { key: "home",  name: "spScnHome",   icon: ICONS.home,  down: [80, 120], up: 15, ping: 80, jitter: 30, spec: "120 ↓ / 15 ↑ Mbps" },
  ];

  const RANK = { good: 0, ok: 1, poor: 2 };
  const worse = (a, b) => (RANK[b] > RANK[a] ? b : a);

  function verdict(s, r) {
    let level = "good";
    if (s.down) {
      if (r.down < s.down[0]) level = worse(level, "poor");
      else if (r.down < s.down[1]) level = worse(level, "ok");
    }
    if (s.up) {
      if (r.up < s.up * 0.7) level = worse(level, "poor");
      else if (r.up < s.up) level = worse(level, "ok");
    }
    if (s.ping) {
      if (r.ping > s.ping * 1.6) level = worse(level, "poor");
      else if (r.ping > s.ping) level = worse(level, "ok");
    }
    if (s.jitter) {
      if (r.jitter > s.jitter * 2) level = worse(level, "poor");
      else if (r.jitter > s.jitter) level = worse(level, "ok");
    }
    return level;
  }

  // --- measurement -------------------------------------------------------
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  async function measureLatency(samples) {
    const raw = [];
    for (let i = 0; i < samples; i++) {
      const t0 = performance.now();
      try {
        await fetch(`${CF}/__down?bytes=0&r=${Math.random()}`, { cache: "no-store" });
      } catch (e) { continue; }
      raw.push(performance.now() - t0);
      await sleep(60);
    }
    if (!raw.length) throw new Error("no latency samples");
    const sorted = raw.slice().sort((a, b) => a - b);
    const ping = sorted[Math.floor(sorted.length / 2)];
    let jit = 0;
    for (let i = 1; i < raw.length; i++) jit += Math.abs(raw[i] - raw[i - 1]);
    jit = raw.length > 1 ? jit / (raw.length - 1) : 0;
    return { ping, jitter: jit };
  }

  async function measureDownload(onLive) {
    const start = performance.now();
    let bytes = 0, stop = false, base = null;
    const controllers = new Set();
    async function worker() {
      while (!stop) {
        const ctrl = new AbortController();
        controllers.add(ctrl);
        try {
          const res = await fetch(`${CF}/__down?bytes=25000000&r=${Math.random()}`, { cache: "no-store", signal: ctrl.signal });
          const reader = res.body.getReader();
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            bytes += value.length;
            const el = performance.now() - start;
            if (!base && el > 1000) base = { bytes, ms: performance.now() };
            if (onLive && base) onLive(mbps(base, bytes));
            if (el >= DOWN_MS || bytes >= DOWN_CAP_BYTES) { stop = true; ctrl.abort(); break; }
          }
        } catch (e) { /* aborted / net error */ }
        controllers.delete(ctrl);
      }
    }
    const workers = Array.from({ length: 4 }, worker);
    await Promise.race([Promise.all(workers), sleep(DOWN_MS + 800)]);
    stop = true;
    controllers.forEach((c) => { try { c.abort(); } catch (e) {} });
    return mbps(base || { bytes: 0, ms: start }, bytes);
  }

  async function measureUpload(onLive) {
    const CHUNK = 2 * 1024 * 1024;
    const buf = new Uint8Array(CHUNK);
    const start = performance.now();
    let bytes = 0, stop = false, base = null;
    async function worker() {
      while (!stop) {
        try {
          await fetch(`${CF}/__up?r=${Math.random()}`, { method: "POST", body: buf, cache: "no-store" });
        } catch (e) { if (stop) break; await sleep(60); continue; }
        bytes += CHUNK;
        const el = performance.now() - start;
        if (!base && el > 1000) base = { bytes, ms: performance.now() };
        if (onLive && base) onLive(mbps(base, bytes));
        if (el >= UP_MS || bytes >= UP_CAP_BYTES) { stop = true; break; }
      }
    }
    await Promise.all(Array.from({ length: 3 }, worker));
    return mbps(base || { bytes: 0, ms: start }, bytes);
  }

  function mbps(base, bytes) {
    const secs = (performance.now() - base.ms) / 1000;
    if (secs <= 0) return 0;
    return ((bytes - base.bytes) * 8) / secs / 1e6;
  }

  // --- UI ----------------------------------------------------------------
  const $ = (id) => document.getElementById(id);
  const fmt1 = (n) => (n >= 100 ? Math.round(n).toString() : n.toFixed(1));

  function buildScenarios() {
    const wrap = $("sp-scenarios");
    if (!wrap || wrap.childElementCount) return;
    SCN.forEach((s) => {
      const el = document.createElement("article");
      el.className = "sp-scn";
      el.dataset.key = s.key;
      el.innerHTML =
        `<span class="sp-ic"><svg viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="${s.icon}"/></svg></span>` +
        `<div class="sp-scn-name" data-i18n="${s.name}">${s.name}</div>` +
        `<div class="sp-scn-req">${s.spec}</div>` +
        `<div class="sp-verdict" data-verdict><span data-i18n="spWaiting">${t("spWaiting")}</span></div>`;
      wrap.appendChild(el);
    });
    relabel(); // localize the freshly-added scenario nodes
  }

  function relabel() {
    document.querySelectorAll("#sp-scenarios [data-i18n]").forEach((el) => {
      el.textContent = t(el.dataset.i18n);
    });
  }

  function paintVerdicts(r) {
    SCN.forEach((s) => {
      const card = document.querySelector(`.sp-scn[data-key="${s.key}"] [data-verdict]`);
      if (!card) return;
      const lvl = verdict(s, r);
      card.className = "sp-verdict " + lvl;
      const label = lvl === "good" ? "spGood" : lvl === "ok" ? "spOk" : "spPoor";
      card.innerHTML = `<span data-i18n="${label}">${t(label)}</span>`;
    });
  }

  let running = false;
  async function run() {
    if (running) return;
    running = true;
    const btn = $("sp-run"), label = $("sp-run-label"), status = $("sp-status"), card = document.querySelector(".sp-card");
    const setVal = (id, v) => { const e = $(id); if (e) e.textContent = v; };
    const setBar = (id, mb) => { const e = $(id); if (e) e.style.width = Math.max(2, Math.min(100, (mb / 1000) * 100)) + "%"; };

    btn.disabled = true;
    btn.classList.add("sp-run-spin");
    card.classList.add("is-active");
    if (label) label.textContent = t("spTesting");
    ["sp-down", "sp-up", "sp-ping", "sp-jit"].forEach((id) => setVal(id, "--"));
    setBar("sp-down-bar", 0); setBar("sp-up-bar", 0);

    try {
      status.textContent = t("spStatusPing");
      const lat = await measureLatency(12);
      setVal("sp-ping", Math.round(lat.ping));
      setVal("sp-jit", Math.round(lat.jitter));

      status.textContent = t("spStatusDown");
      const down = await measureDownload((mb) => { setVal("sp-down", fmt1(mb)); setBar("sp-down-bar", mb); });
      setVal("sp-down", fmt1(down)); setBar("sp-down-bar", down);

      status.textContent = t("spStatusUp");
      const up = await measureUpload((mb) => { setVal("sp-up", fmt1(mb)); setBar("sp-up-bar", mb); });
      setVal("sp-up", fmt1(up)); setBar("sp-up-bar", up);

      const results = { down, up, ping: lat.ping, jitter: lat.jitter };
      paintVerdicts(results);
      window.__glintSpeed = results;
      status.textContent = t("spStatusDone");
    } catch (e) {
      status.textContent = t("spStatusErr");
    } finally {
      btn.disabled = false;
      btn.classList.remove("sp-run-spin");
      card.classList.remove("is-active");
      if (label) label.textContent = t("spRun");
      running = false;
    }
  }

  function init() {
    if (!$("sp-run")) return;
    buildScenarios();
    $("sp-run").addEventListener("click", run);
    // re-localize scenario labels + verdicts when the language changes
    document.addEventListener("glint:lang", () => {
      relabel();
      if (window.__glintSpeed) paintVerdicts(window.__glintSpeed);
      const label = $("sp-run-label");
      if (label && !running) label.textContent = t("spRun");
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
