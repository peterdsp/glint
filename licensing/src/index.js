// Glint automatic licensing - Cloudflare Worker.
//
// Ko-fi posts a Shop Order webhook here on every purchase. For a Glint order we
// mint an Ed25519-signed license key (the exact format the app verifies with
// its embedded public key) and email it to the buyer. Nothing manual.
//
// The signing uses tweetnacl; a key minted here verifies against ed25519-dalek
// in the app because both implement standard Ed25519 (RFC 8032).
//
// Secrets (wrangler secret put ...):
//   LICENSE_PRIVATE_KEY      base64 seed from glint-keygen (.secrets/glint-license-private.key)
//   KOFI_VERIFICATION_TOKEN  Ko-fi -> Settings -> API/Webhooks -> Verification Token
//   RESEND_API_KEY           an API key from resend.com (or swap sendEmail for your provider)
// Vars (wrangler.toml [vars]):
//   FROM_EMAIL               e.g. "Glint <licenses@peterdsp.dev>" (domain verified in Resend)
//   GLINT_ITEM_CODES         the Glint Ko-fi item code(s) from its share URL ko-fi.com/s/XXXX

import nacl from "tweetnacl";

const enc = new TextEncoder();

function b64urlEncode(bytes) {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function b64decode(s) {
  return Uint8Array.from(atob(s.trim()), (c) => c.charCodeAt(0));
}

// Mint a license key: `base64url(payload).base64url(signature)`, payload matching
// the app's LicenseInfo { email, plan }.
function mintLicense(seedB64, email) {
  const seed = b64decode(seedB64);
  const pair = nacl.sign.keyPair.fromSeed(seed);
  const payload = enc.encode(JSON.stringify({ email, plan: "kofi" }));
  const sig = nacl.sign.detached(payload, pair.secretKey);
  return `${b64urlEncode(payload)}.${b64urlEncode(sig)}`;
}

async function emailKey(env, to, name, key) {
  const body = {
    from: env.FROM_EMAIL,
    to: [to],
    subject: "Your Glint license key",
    text: [
      `Hi ${name || "there"},`,
      "",
      "Thanks for buying Glint! Here is how to get going:",
      "",
      "1. Download Glint for your OS:",
      "   https://github.com/peterdsp/glint/releases/latest",
      "2. Install and open it.",
      "3. Open Settings and paste this license key:",
      "",
      key,
      "",
      "That unlocks Glint for good on all your machines. Enjoy!",
      "",
      "Glint",
    ].join("\n"),
  };
  const res = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.RESEND_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`email send failed: ${res.status} ${await res.text()}`);
  }
}

export default {
  async fetch(request, env, ctx) {
    if (request.method !== "POST") {
      return new Response("Glint licensing webhook is up.", { status: 200 });
    }

    let data;
    try {
      const form = await request.formData();
      data = JSON.parse(form.get("data"));
    } catch (e) {
      return new Response("bad payload", { status: 400 });
    }

    // Authenticate the webhook.
    if (!env.KOFI_VERIFICATION_TOKEN || data.verification_token !== env.KOFI_VERIFICATION_TOKEN) {
      return new Response("unauthorized", { status: 401 });
    }

    // Only fulfil Glint shop orders.
    if (data.type !== "Shop Order") return new Response("ignored (not a shop order)", { status: 200 });

    const codes = (env.GLINT_ITEM_CODES || "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const items = Array.isArray(data.shop_items) ? data.shop_items : [];
    const isGlint = codes.length > 0 && items.some((it) => codes.includes(it.direct_link_code));
    if (!isGlint) return new Response("ignored (not the Glint item)", { status: 200 });

    const email = (data.email || "").trim();
    if (!email) return new Response("no buyer email", { status: 200 });

    // Idempotency: Ko-fi may retry. Skip if we already handled this message.
    if (env.LICENSES) {
      const seen = await env.LICENSES.get(`msg:${data.message_id}`);
      if (seen) return new Response("already processed", { status: 200 });
    }

    let key;
    try {
      key = mintLicense(env.LICENSE_PRIVATE_KEY, email);
      await emailKey(env, email, data.from_name, key);
    } catch (e) {
      // Return 500 so Ko-fi retries; the buyer still gets a key on the retry.
      return new Response(`fulfilment error: ${e.message}`, { status: 500 });
    }

    if (env.LICENSES) {
      ctx.waitUntil(
        env.LICENSES.put(`msg:${data.message_id}`, "1", { expirationTtl: 60 * 60 * 24 * 400 })
      );
      ctx.waitUntil(env.LICENSES.put(`email:${email.toLowerCase()}`, key));
    }

    return new Response("licensed", { status: 200 });
  },
};
