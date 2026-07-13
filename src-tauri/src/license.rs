// The App Store build bypasses trial/license, leaving this code unused there.
#![cfg_attr(feature = "appstore", allow(dead_code))]

// Trial + license engine for the direct (Ko-fi) build.
//
// Model:
//   - First launch starts a 7-day free trial (a timestamp in the config dir).
//   - After 7 days the app is "expired" until a valid license key is entered.
//   - License keys are offline, Ed25519-signed tokens: `<payload>.<sig>`, both
//     base64url. The app embeds only the PUBLIC key and verifies; keys are
//     minted out-of-band with the private key (see the `glint-keygen` binary).
//     No server, nothing to phone home to.
//
// The App Store build compiles with `--features appstore`, which bypasses all of
// this (Apple gates the purchase, and self-updating/licensing there is against
// the rules). See `state()` short-circuit in main.rs.

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

pub const TRIAL_DAYS: u64 = 7;
const DAY: u64 = 86_400;

/// Public key that license signatures are checked against, base64 (32 bytes).
/// Overridden at build time via `GLINT_LICENSE_PUBKEY`; the placeholder rejects
/// every key until you embed your real one (see docs/RELEASE.md).
pub fn pubkey_b64() -> &'static str {
    option_env!("GLINT_LICENSE_PUBKEY")
        .unwrap_or("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LicenseInfo {
    pub email: String,
    #[serde(default)]
    pub plan: String,
    /// Optional expiry (unix seconds); None = perpetual.
    #[serde(default)]
    pub exp: Option<u64>,
}

#[derive(Serialize)]
pub struct LicenseState {
    /// "trial" | "licensed" | "expired"
    pub state: String,
    pub days_left: u64,
    pub email: Option<String>,
    pub trial_days: u64,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
}

/// Always-licensed state, used by the App Store build (purchase gated by Apple).
pub fn licensed_state() -> LicenseState {
    LicenseState {
        state: "licensed".into(),
        days_left: 0,
        email: None,
        trial_days: TRIAL_DAYS,
    }
}

/// Verify a license key against a public key. Returns the payload on success.
pub fn verify(key: &str, pubkey_base64: &str) -> Result<LicenseInfo, String> {
    let pk_bytes = base64::engine::general_purpose::STANDARD
        .decode(pubkey_base64.trim())
        .map_err(|_| "bad public key encoding".to_string())?;
    let pk_arr: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "public key must be 32 bytes".to_string())?;
    let verifying = VerifyingKey::from_bytes(&pk_arr).map_err(|_| "invalid public key".to_string())?;

    let (payload_b64, sig_b64) = key
        .trim()
        .split_once('.')
        .ok_or("license key is malformed (expected payload.signature)")?;
    let payload = b64()
        .decode(payload_b64)
        .map_err(|_| "license payload is not valid base64".to_string())?;
    let sig_bytes = b64()
        .decode(sig_b64)
        .map_err(|_| "license signature is not valid base64".to_string())?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| "signature must be 64 bytes".to_string())?;
    let signature = Signature::from_bytes(&sig_arr);

    verifying
        .verify(&payload, &signature)
        .map_err(|_| "license signature does not match".to_string())?;

    let info: LicenseInfo =
        serde_json::from_slice(&payload).map_err(|_| "license payload is not valid".to_string())?;
    Ok(info)
}

/// Resolve the current state from the stored first-run time, the current time,
/// and any stored+verified license.
pub fn evaluate(first_run: u64, now: u64, license: Option<LicenseInfo>) -> LicenseState {
    if let Some(info) = license {
        let live = match info.exp {
            Some(exp) => now < exp,
            None => true,
        };
        if live {
            return LicenseState {
                state: "licensed".into(),
                days_left: info.exp.map(|e| e.saturating_sub(now) / DAY).unwrap_or(0),
                email: Some(info.email),
                trial_days: TRIAL_DAYS,
            };
        }
    }

    let elapsed = now.saturating_sub(first_run);
    let trial_secs = TRIAL_DAYS * DAY;
    if elapsed < trial_secs {
        // ceil so day 1 shows "7 days left", not 6.
        let left = (trial_secs - elapsed + DAY - 1) / DAY;
        LicenseState {
            state: "trial".into(),
            days_left: left.min(TRIAL_DAYS),
            email: None,
            trial_days: TRIAL_DAYS,
        }
    } else {
        LicenseState {
            state: "expired".into(),
            days_left: 0,
            email: None,
            trial_days: TRIAL_DAYS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn keypair() -> (SigningKey, String) {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(sk.verifying_key().to_bytes());
        (sk, pk_b64)
    }

    fn mint(sk: &SigningKey, info: &LicenseInfo) -> String {
        let payload = serde_json::to_vec(info).unwrap();
        let sig = sk.sign(&payload);
        format!(
            "{}.{}",
            b64().encode(&payload),
            b64().encode(sig.to_bytes())
        )
    }

    #[test]
    fn valid_key_verifies_and_wrong_key_fails() {
        let (sk, pk) = keypair();
        let info = LicenseInfo { email: "a@b.co".into(), plan: "kofi".into(), exp: None };
        let key = mint(&sk, &info);
        assert_eq!(verify(&key, &pk).unwrap().email, "a@b.co");

        // A different keypair must reject it.
        let (_, other_pk) = keypair();
        assert!(verify(&key, &other_pk).is_err());
        // Tampered payload must reject.
        let mut bad = key.clone();
        bad.replace_range(0..1, "Z");
        assert!(verify(&bad, &pk).is_err());
        // Malformed input.
        assert!(verify("not-a-key", &pk).is_err());
    }

    #[test]
    fn trial_then_expiry() {
        let start = 1_000_000u64;
        // Day 0: 7 left.
        let s = evaluate(start, start, None);
        assert_eq!(s.state, "trial");
        assert_eq!(s.days_left, 7);
        // Day 3: 4 left.
        let s = evaluate(start, start + 3 * DAY, None);
        assert_eq!(s.state, "trial");
        assert_eq!(s.days_left, 4);
        // Day 7+: expired.
        let s = evaluate(start, start + 7 * DAY + 10, None);
        assert_eq!(s.state, "expired");
        assert_eq!(s.days_left, 0);
    }

    #[test]
    fn license_overrides_expired_trial() {
        let start = 1_000_000u64;
        let now = start + 30 * DAY; // long past trial
        let perpetual = LicenseInfo { email: "x@y.z".into(), plan: "kofi".into(), exp: None };
        let s = evaluate(start, now, Some(perpetual));
        assert_eq!(s.state, "licensed");
        assert_eq!(s.email.as_deref(), Some("x@y.z"));

        // An expired license falls back to expired trial.
        let expired = LicenseInfo { email: "x@y.z".into(), plan: "kofi".into(), exp: Some(now - 1) };
        let s = evaluate(start, now, Some(expired));
        assert_eq!(s.state, "expired");
    }
}
