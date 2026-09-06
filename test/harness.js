// Test harness: boot the real Glint panel (index.html + i18n.js + themes.js +
// app.js) inside jsdom with a mock Tauri IPC, so interaction behavior can be
// driven and asserted the way a keyboard or screen-reader user would meet it.
//
// The app talks to the Rust core only through window.__TAURI__.core.invoke; the
// mock records every call and returns fixtures, so no native side is needed.

const fs = require("node:fs");
const path = require("node:path");
const { JSDOM } = require("jsdom");

const SRC = path.join(__dirname, "..", "src");
const readSrc = (name) => fs.readFileSync(path.join(SRC, name), "utf8");

// A repo with one unstaged and one staged (and partially staged) file, plus
// something to pull and to push - enough to exercise every control.
const DEFAULT_STATUS = {
  branch: "main",
  ahead: 2,
  behind: 1,
  files: [
    { path: "src/alpha.js", status: "M", staged: false, added: 3, removed: 1, partially_staged: false },
    { path: "src/beta.js", status: "M", staged: true, added: 5, removed: 0, partially_staged: true },
  ],
};

function makeInvoke(calls, overrides, status) {
  return function invoke(cmd, args) {
    calls.push({ cmd, args });
    if (overrides && Object.prototype.hasOwnProperty.call(overrides, cmd)) {
      // Run through a microtask so a throwing override becomes a rejection,
      // exactly like a failing IPC call.
      return Promise.resolve().then(() => overrides[cmd](args));
    }
    switch (cmd) {
      case "is_app_store": return Promise.resolve(false);
      case "app_mode": return Promise.resolve("menubar");
      case "load_themes": return Promise.resolve([]);
      case "license_status": return Promise.resolve({ state: "licensed", email: null });
      case "github_token_set": return Promise.resolve(false);
      case "app_version": return Promise.resolve("test");
      case "get_status": return Promise.resolve(status);
      case "pr_status": return Promise.resolve(null);
      case "update_now": return Promise.resolve(false);
      case "open_diff": return Promise.resolve();
      case "diff": return Promise.resolve({ file: args && args.file, binary: false, hunks: [] });
      case "fetch":
      case "pull":
      case "push": return Promise.resolve(status);
      case "commit": return Promise.resolve();
      case "set_github_token": return Promise.resolve();
      default: return Promise.resolve(null);
    }
  };
}

async function setup(opts = {}) {
  const { repo = "/tmp/demo-repo", overrides, status = DEFAULT_STATUS, locale } = opts;
  const calls = [];
  const dom = new JSDOM(readSrc("index.html"), {
    runScripts: "dangerously",
    url: "https://glint.test/",
    pretendToBeVisual: true, // provides requestAnimationFrame for announce()
    beforeParse(window) {
      window.__TAURI__ = { core: { invoke: makeInvoke(calls, overrides, status) } };
    },
  });
  const { window } = dom;
  try {
    if (repo) window.localStorage.setItem("glint.repo", repo);
    if (locale) window.localStorage.setItem("glint.locale", locale);
  } catch {}

  // The <script src> tags in the HTML are inert (jsdom loads no external
  // resources); run the real sources here, in order, in the page context.
  for (const name of ["i18n.js", "themes.js", "app.js"]) {
    const s = window.document.createElement("script");
    s.textContent = readSrc(name);
    window.document.body.appendChild(s);
  }
  return { dom, window, document: window.document, calls };
}

function waitFor(predicate, { timeout = 3000, interval = 10 } = {}) {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    (function poll() {
      let ok = false;
      try { ok = predicate(); } catch { ok = false; }
      if (ok) return resolve();
      if (Date.now() - start > timeout) return reject(new Error("waitFor timed out"));
      setTimeout(poll, interval);
    })();
  });
}

// Wait a couple of animation frames so announce()'s requestAnimationFrame lands.
function frames(window, n = 2) {
  return new Promise((resolve) => {
    let left = n;
    (function step() {
      if (left-- <= 0) return resolve();
      if (window.requestAnimationFrame) window.requestAnimationFrame(step);
      else setTimeout(step, 16);
    })();
  });
}

module.exports = { setup, waitFor, frames, DEFAULT_STATUS };
