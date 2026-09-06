// Regression guards for the accessibility affordances that live in CSS/markup.
// jsdom has no real cascade or media-query engine, so these assert the source
// declares the features rather than trying to compute them - cheap insurance
// that the focus rings and OS-preference blocks are not silently dropped.

const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const read = (f) => fs.readFileSync(path.join(__dirname, "..", "src", f), "utf8");

test("panel CSS declares focus, SR-only, and every OS-preference block", () => {
  const css = read("styles.css");
  for (const needle of [
    ":focus-visible",
    ".sr-only",
    "prefers-reduced-motion",
    "prefers-reduced-transparency",
    "prefers-contrast",
    "--panel-solid",
  ]) {
    assert.ok(css.includes(needle), `styles.css contains ${needle}`);
  }
});

test("diff CSS declares focus + reduced-transparency/contrast handling", () => {
  const css = read("diff.css");
  for (const needle of [
    ".dbody:focus-visible",
    "prefers-reduced-motion",
    "prefers-reduced-transparency",
    "prefers-contrast",
  ]) {
    assert.ok(css.includes(needle), `diff.css contains ${needle}`);
  }
});

test("panel markup drops role=application and adds the live regions", () => {
  const html = read("index.html");
  assert.ok(!/role="application"/.test(html), "role=application is removed");
  assert.ok(html.includes('id="sr-status"'), "polite live region present");
  assert.ok(html.includes('id="sr-alert"'), "assertive live region present");
  assert.ok(html.includes('role="list"'), "file list keeps list semantics");
});

test("diff markup exposes a focusable, labelled scroll region", () => {
  const html = read("diff.html");
  assert.ok(/id="body"[^>]*tabindex="0"/.test(html), "diff body is focusable");
  assert.ok(/aria-labelledby="file"/.test(html), "diff body is labelled by the file heading");
});
