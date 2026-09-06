// Unit tests for the localization helper, including the accessible-name and
// announcement strings added for the accessibility work.

const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { JSDOM } = require("jsdom");

function loadI18n(locale) {
  const dom = new JSDOM("<!doctype html><body></body>", {
    runScripts: "dangerously",
    url: "https://glint.test/",
  });
  if (locale) dom.window.localStorage.setItem("glint.locale", locale);
  const s = dom.window.document.createElement("script");
  s.textContent = fs.readFileSync(path.join(__dirname, "..", "src", "i18n.js"), "utf8");
  dom.window.document.body.appendChild(s);
  return dom.window;
}

test("t() substitutes placeholders, including the new aria strings", () => {
  const w = loadI18n();
  assert.equal(w.t("changedFiles", { n: 3 }), "3 changed files");
  assert.equal(w.t("pullAria", { n: 2 }), "Pull from origin, 2 to pull");
  assert.equal(w.t("diffAdds", { n: 5 }), "5 additions");
});

test("t() returns the key for an unknown string", () => {
  assert.equal(loadI18n().t("does-not-exist"), "does-not-exist");
});

test("locale switching localizes accessible strings", () => {
  const w = loadI18n("sq");
  assert.equal(w.currentLocale(), "sq");
  assert.match(w.t("pullAria", { n: 1 }), /Merr nga origin/);
  w.setLocale("el");
  assert.match(w.t("fetchAria"), /origin/);
});

test("every locale defines the accessibility keys", () => {
  const w = loadI18n();
  const keys = ["pullAria", "pushAria", "fetchAria", "prAria", "committed", "diffAdds", "diffDels"];
  for (const loc of ["en", "el", "sq"]) {
    for (const k of keys) {
      assert.ok(w.GLINT_I18N[loc][k], `${loc}.${k} is defined`);
    }
  }
});
