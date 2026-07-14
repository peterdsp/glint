# Glint automatic licensing (Cloudflare Worker)

When someone buys Glint on Ko-fi, Ko-fi calls this Worker, which mints an
Ed25519-signed license key and emails it to the buyer. No manual step.

```
Ko-fi purchase  ->  Ko-fi webhook  ->  Worker  ->  mint key + email buyer
                                                      |
                                        buyer pastes key in Settings
                                                      |
                                        app verifies with embedded public key
```

Keys minted here verify in the app because the Worker (tweetnacl) and the app
(ed25519-dalek) both implement standard Ed25519. Proven with:

```sh
cd licensing && npm install
node scripts/check.mjs ../.secrets/glint-license-private.key
# then run the printed `glint-keygen verify ...` -> VALID
```

## One-time setup

1. **Install + log in**
   ```sh
   cd licensing
   npm install
   npx wrangler login
   ```

2. **Email sending (Resend)** - sign up at resend.com, verify your sending
   domain (add the DNS records to Cloudflare), create an API key. Set
   `FROM_EMAIL` in `wrangler.toml` to an address on that domain.

3. **Secrets**
   ```sh
   # the license private key (base64 seed) from glint-keygen new
   npx wrangler secret put LICENSE_PRIVATE_KEY   # paste .secrets/glint-license-private.key
   npx wrangler secret put RESEND_API_KEY        # from resend.com
   npx wrangler secret put KOFI_VERIFICATION_TOKEN  # step 5
   ```

4. **(Optional) KV** for retry-idempotency and buyer key lookup
   ```sh
   npx wrangler kv namespace create LICENSES
   # paste the id into wrangler.toml and uncomment the [[kv_namespaces]] block
   ```

5. **Deploy + point Ko-fi at it**
   ```sh
   npx wrangler deploy   # prints your Worker URL
   ```
   - Ko-fi -> Settings -> **API / Webhooks**: set the **Webhook URL** to the
     Worker URL, copy the **Verification Token** into the secret above.
   - Publish the Glint shop item, open its share link `ko-fi.com/s/XXXXXXXX`,
     and put `XXXXXXXX` in `GLINT_ITEM_CODES` in `wrangler.toml`, then
     `npx wrangler deploy` again. This makes the Worker fulfil only Glint
     orders (not your other products).

## Test it

Ko-fi's webhook page has a **Send test** button. A test Shop Order with a
matching item code and the right verification token should email a key to the
address on the test payload.

## Notes

- The buyer's key is tied to their email but is otherwise perpetual and works on
  all their machines. To stop key-sharing later, add per-key device activation
  (store activations in the `LICENSES` KV and cap them) - the client already
  sends nothing, so it is an additive change.
- To swap Resend for another provider, replace `emailKey()` in `src/index.js`.
