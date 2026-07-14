// Prove the Worker's tweetnacl signing is cross-compatible with the app's
// ed25519-dalek verification. Reads the license private key, derives the public
// key, and mints a sample license key using the SAME code path as the Worker.
//
//   node scripts/check.mjs <private_key_b64|path-to-key-file>
//
// Then feed the printed key + pubkey to `glint-keygen verify`.
import nacl from "tweetnacl";
import fs from "node:fs";

const enc = new TextEncoder();
const b64urlEncode = (bytes) => {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};
const b64decode = (s) => Uint8Array.from(atob(s.trim()), (c) => c.charCodeAt(0));
const stdEncode = (bytes) => {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
};

let arg = process.argv[2];
if (!arg) {
  console.error("usage: node scripts/check.mjs <private_key_b64|path-to-key-file>");
  process.exit(2);
}
// allow passing a file path
let seedB64 = arg;
try {
  if (fs.existsSync(arg)) seedB64 = fs.readFileSync(arg, "utf8").trim();
} catch {}

const seed = b64decode(seedB64);
const pair = nacl.sign.keyPair.fromSeed(seed);

const payload = enc.encode(JSON.stringify({ email: "buyer@example.com", plan: "kofi" }));
const sig = nacl.sign.detached(payload, pair.secretKey);
const key = `${b64urlEncode(payload)}.${b64urlEncode(sig)}`;

console.log("PUBLIC KEY (tweetnacl):", stdEncode(pair.publicKey));
console.log("SAMPLE LICENSE KEY:");
console.log(key);
console.log("\nVerify it with the app's crypto:");
console.log(`  cd ../src-tauri && cargo run -q --bin glint-keygen -- verify "${key}" "${stdEncode(pair.publicKey)}"`);
