// Local integration test of the licensing Worker, no deploy / no real email.
// Simulates Ko-fi's Shop Order webhook and asserts the Worker mints a valid key
// and would email it. Run: node scripts/wtest.mjs
import handler from "../src/index.js";
import fs from "node:fs";
import { execSync } from "node:child_process";

const seed = fs
  .readFileSync(new URL("../../.secrets/glint-license-private.key", import.meta.url), "utf8")
  .trim();

// Intercept the Resend email call so nothing is actually sent.
let emailed = null;
const realFetch = globalThis.fetch;
globalThis.fetch = async (url, opts) => {
  if (String(url).includes("resend.com")) {
    emailed = JSON.parse(opts.body);
    return new Response(JSON.stringify({ id: "mock" }), { status: 200 });
  }
  return realFetch(url, opts);
};

const env = {
  LICENSE_PRIVATE_KEY: seed,
  KOFI_VERIFICATION_TOKEN: "testtoken",
  GLINT_ITEM_CODES: "c2c1f5857a",
  FROM_EMAIL: "Glint <licenses@peterdsp.dev>",
  RESEND_API_KEY: "test_key",
};

function kofiRequest(overrides = {}) {
  const payload = {
    verification_token: "testtoken",
    type: "Shop Order",
    email: "buyer@test.com",
    from_name: "Buyer",
    message_id: "m-" + Math.round(performance.now()),
    shop_items: [{ direct_link_code: "c2c1f5857a" }],
    ...overrides,
  };
  const body = new URLSearchParams({ data: JSON.stringify(payload) });
  return new Request("https://worker.example/", {
    method: "POST",
    body,
    headers: { "content-type": "application/x-www-form-urlencoded" },
  });
}

let failures = 0;
const check = (name, cond) => {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}`);
  if (!cond) failures++;
};

// 1. Happy path: a Glint shop order mints + emails a key.
const res = await handler.fetch(kofiRequest(), env, { waitUntil() {} });
const text = await res.text();
check("shop order returns 200 licensed", res.status === 200 && text === "licensed");
check("email was sent to the buyer", emailed && emailed.to[0] === "buyer@test.com");

const key = (emailed?.text || "")
  .split("\n")
  .map((s) => s.trim())
  .find((l) => /^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(l) && l.length > 60);
check("email contains a license key", !!key);

// Verify the emailed key with the app's exact crypto.
if (key) {
  const pub = execSync(`cargo run -q --features keygen --bin glint-keygen -- pubkey "${seed}"`, {
    cwd: new URL("../../src-tauri", import.meta.url),
  }).toString().trim();
  const verify = execSync(`cargo run -q --features keygen --bin glint-keygen -- verify "${key}" "${pub}"`, {
    cwd: new URL("../../src-tauri", import.meta.url),
  }).toString().trim();
  check("emailed key verifies in the app (ed25519-dalek)", verify.startsWith("VALID"));
}

// 2. Wrong verification token is rejected.
const bad = await handler.fetch(kofiRequest({ verification_token: "nope" }), env, { waitUntil() {} });
check("wrong token -> 401", bad.status === 401);

// 3. A different product is ignored (no key for non-Glint items).
emailed = null;
const other = await handler.fetch(
  kofiRequest({ shop_items: [{ direct_link_code: "somethingelse" }] }),
  env,
  { waitUntil() {} }
);
check("non-Glint item -> 200 ignored, no email", other.status === 200 && emailed === null);

console.log(failures === 0 ? "\nALL WORKER TESTS PASSED" : `\n${failures} FAILURE(S)`);
process.exit(failures === 0 ? 0 : 1);
