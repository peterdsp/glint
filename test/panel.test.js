// Interaction tests for the Glint panel accessibility work. These drive the
// real DOM the way a keyboard/screen-reader user would and assert roles,
// states, focus movement, and the announcements that back each operation.

const { test } = require("node:test");
const assert = require("node:assert/strict");
const { setup, waitFor, frames } = require("./harness");

const filesReady = (document) => document.querySelectorAll("#files li").length > 0;
// render() sets repo-name from the status; a reliable "the view has rendered"
// signal even when there are no changed files.
const rendered = (document) => document.getElementById("repo-name").textContent !== "-";

test("file rows expose semantic checkbox + button controls with names and states", async () => {
  const { document } = await setup();
  await waitFor(() => filesReady(document));

  const rows = document.querySelectorAll("#files li");
  assert.equal(rows.length, 2, "both changed files render");

  const [alpha, beta] = rows;
  assert.equal(alpha.getAttribute("role"), "listitem");

  // Staging control is a real button acting as a checkbox, with a name + state.
  const alphaCheck = alpha.querySelector(".check");
  assert.equal(alphaCheck.tagName, "BUTTON", "checkbox is a native button, not a span");
  assert.equal(alphaCheck.getAttribute("role"), "checkbox");
  assert.equal(alphaCheck.getAttribute("aria-checked"), "false", "unstaged file reads unchecked");
  assert.equal(alphaCheck.getAttribute("aria-label"), "src/alpha.js");

  const betaCheck = beta.querySelector(".check");
  assert.equal(betaCheck.getAttribute("aria-checked"), "true", "staged file reads checked");

  // Opening the diff is a real button with an action-naming label.
  const alphaPath = alpha.querySelector(".file-path");
  assert.equal(alphaPath.tagName, "BUTTON", "path opener is a native button, not a span");
  assert.match(alphaPath.getAttribute("aria-label"), /src\/alpha\.js/);
  assert.match(alphaPath.getAttribute("aria-label"), /diff/i);

  // List semantics survive list-style:none (explicit roles).
  assert.equal(document.getElementById("files").getAttribute("role"), "list");
});

test("activating the checkbox toggles staged state and aria-checked", async () => {
  const { document } = await setup();
  await waitFor(() => filesReady(document));

  // alpha starts unstaged. Activating rebuilds the row, so re-query each time.
  document.querySelector("#files li:first-child .check").click();
  let check = document.querySelector("#files li:first-child .check");
  assert.equal(check.getAttribute("aria-checked"), "true");
  assert.ok(check.classList.contains("on"));
  assert.ok(check.querySelector("i.ti-check"), "shows the check glyph when on");

  document.querySelector("#files li:first-child .check").click();
  check = document.querySelector("#files li:first-child .check");
  assert.equal(check.getAttribute("aria-checked"), "false");
  assert.ok(check.classList.contains("off"));
});

test("activating a file path opens its diff over IPC", async () => {
  const { document, calls } = await setup();
  await waitFor(() => filesReady(document));

  document.querySelector("#files li:nth-child(2) .file-path").click();
  await waitFor(() => calls.some((c) => c.cmd === "open_diff"));

  // Field-by-field (the args object comes from the jsdom realm, so a strict
  // deepEqual would compare prototypes across realms).
  const call = calls.find((c) => c.cmd === "open_diff");
  assert.equal(call.args.path, "/tmp/demo-repo");
  assert.equal(call.args.file, "src/beta.js");
});

test("sync buttons carry live counts and reflect disabled state", async () => {
  const { document } = await setup();
  await waitFor(() => filesReady(document));

  const pull = document.getElementById("pull-btn");
  const push = document.getElementById("push-btn");
  assert.match(pull.getAttribute("aria-label"), /1 to pull/);
  assert.match(push.getAttribute("aria-label"), /2 to push/);
  assert.equal(pull.disabled, false, "pull enabled when behind > 0");
  assert.equal(push.disabled, false, "push enabled when ahead > 0");

  // Nothing to move => both disabled and exposed as such.
  const clean = await setup({ status: { branch: "main", ahead: 0, behind: 0, files: [] } });
  await waitFor(() => rendered(clean.document));
  assert.equal(clean.document.getElementById("pull-btn").disabled, true);
  assert.equal(clean.document.getElementById("push-btn").disabled, true);
});

test("operation errors are announced assertively", async () => {
  const { document, window } = await setup({
    overrides: {
      pull() {
        throw new Error("network down");
      },
    },
  });
  await waitFor(() => filesReady(document));

  document.getElementById("pull-btn").click();
  await waitFor(() => document.getElementById("sr-alert").textContent.length > 0);
  await frames(window);

  // The failure lands in the assertive region; the polite region never carries
  // the error text (the "Pulling..." progress note there is expected).
  assert.equal(document.getElementById("sr-alert").textContent, "network down");
  assert.ok(
    !document.getElementById("sr-status").textContent.includes("network down"),
    "errors do not use the polite region"
  );
});

test("successful operations announce politely", async () => {
  const { document, window } = await setup();
  await waitFor(() => filesReady(document));

  document.getElementById("push-btn").click();
  await waitFor(() => document.getElementById("sr-status").textContent.length > 0);
  await frames(window);

  assert.match(document.getElementById("sr-status").textContent, /push/i);
  assert.equal(document.getElementById("sr-alert").textContent, "", "success is not assertive");
});

test("opening Settings moves focus in, inerts the background, and restores focus on close", async () => {
  const { document, window } = await setup();
  await waitFor(() => filesReady(document));

  const settingsBtn = document.getElementById("settings-btn");
  settingsBtn.focus(); // deterministic starting point
  settingsBtn.click();

  const panel = document.getElementById("settings");
  assert.equal(panel.hidden, false, "settings overlay is shown");
  assert.equal(document.activeElement, document.getElementById("settings-close"), "focus enters the overlay");
  const header = document.querySelector(".head");
  assert.ok(header.inert || header.hasAttribute("inert"), "header is inert behind the overlay");
  assert.ok(document.getElementById("repo-view").inert, "repo view is inert behind the overlay");

  // Escape closes and returns focus to the opener.
  panel.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  assert.equal(panel.hidden, true, "settings overlay is hidden again");
  assert.ok(!header.inert && !header.hasAttribute("inert"), "background is interactive again");
  assert.equal(document.activeElement, settingsBtn, "focus returns to the opener");
});

test("connecting a repo from onboarding lands focus in the repo view", async () => {
  const { document, window } = await setup({
    repo: null, // no saved repo => onboarding is shown
    overrides: { pick_repo: () => "/tmp/picked" },
  });
  await waitFor(() => document.getElementById("onboard").hidden === false);

  document.getElementById("ob-open").click(); // connect
  await waitFor(() => filesReady(document));
  await frames(window);

  const active = document.activeElement;
  assert.ok(active && active.classList.contains("check"), "keyboard lands on the first file's checkbox");
});

test("first-run mode chooser is a modal dialog that takes focus", async () => {
  const { document } = await setup({ overrides: { app_mode: () => null } });
  await waitFor(() => document.getElementById("modepick").hidden === false);

  const mp = document.getElementById("modepick");
  assert.equal(mp.getAttribute("role"), "dialog");
  assert.equal(mp.getAttribute("aria-modal"), "true");
  assert.equal(document.activeElement, document.getElementById("mp-menubar"), "focus enters the chooser");
});

test("expired-trial gate is a modal dialog that focuses the key field", async () => {
  const { document } = await setup({ overrides: { license_status: () => ({ state: "expired" }) } });
  await waitFor(() => document.getElementById("license-gate").hidden === false);

  const gate = document.getElementById("license-gate");
  assert.equal(gate.getAttribute("role"), "dialog");
  assert.equal(gate.getAttribute("aria-modal"), "true");
  await waitFor(() => document.activeElement === document.getElementById("lg-input"));
});

test("Cmd/Ctrl+Enter in the message field commits the staged files", async () => {
  const { document, window, calls } = await setup();
  await waitFor(() => filesReady(document));

  const summary = document.getElementById("commit-summary");
  summary.value = "Fix the thing";
  summary.dispatchEvent(new window.KeyboardEvent("keydown", { key: "Enter", metaKey: true, bubbles: true }));

  await waitFor(() => calls.some((c) => c.cmd === "commit"));
  const call = calls.find((c) => c.cmd === "commit");
  assert.deepEqual(call.args.files, ["src/beta.js"], "only staged files are committed");
  assert.equal(call.args.summary, "Fix the thing");
});
