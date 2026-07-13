// glint-keygen - offline license tooling (issue: paid distribution).
//
//   cargo run --bin glint-keygen -- new
//       Generate a signing keypair. Prints the PUBLIC key to embed at build
//       time (GLINT_LICENSE_PUBKEY) and the PRIVATE key to keep secret.
//
//   cargo run --bin glint-keygen -- sign <email> <private_key_b64> [days]
//       Mint a license key for a buyer. Omit [days] for a perpetual license.
//
// The app never sees the private key; it only verifies with the embedded public
// key. Keep the private key somewhere safe (a password manager) - anyone with it
// can mint licenses.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use std::time::{SystemTime, UNIX_EPOCH};

fn b64url() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
}
fn std64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("new") => new_keypair(),
        Some("sign") => sign(&args[1..]),
        _ => {
            eprintln!("usage:");
            eprintln!("  glint-keygen new");
            eprintln!("  glint-keygen sign <email> <private_key_b64> [days]");
            std::process::exit(2);
        }
    }
}

fn new_keypair() {
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let priv_b64 = std64().encode(sk.to_bytes());
    let pub_b64 = std64().encode(sk.verifying_key().to_bytes());
    println!("PUBLIC KEY  (embed at build: GLINT_LICENSE_PUBKEY)\n  {pub_b64}\n");
    println!("PRIVATE KEY (keep secret - mints licenses)\n  {priv_b64}");
}

fn sign(args: &[String]) {
    let (email, priv_b64) = match (args.first(), args.get(1)) {
        (Some(e), Some(p)) => (e, p),
        _ => {
            eprintln!("usage: glint-keygen sign <email> <private_key_b64> [days]");
            std::process::exit(2);
        }
    };
    let sk_bytes = std64().decode(priv_b64.trim()).expect("private key must be base64");
    let sk_arr: [u8; 32] = sk_bytes.as_slice().try_into().expect("private key must be 32 bytes");
    let sk = SigningKey::from_bytes(&sk_arr);

    let exp = args.get(2).and_then(|d| d.parse::<u64>().ok()).map(|days| {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        now + days * 86_400
    });

    // Payload mirrors license::LicenseInfo.
    let payload = match exp {
        Some(e) => format!(r#"{{"email":"{email}","plan":"kofi","exp":{e}}}"#),
        None => format!(r#"{{"email":"{email}","plan":"kofi"}}"#),
    };
    let sig = sk.sign(payload.as_bytes());
    let key = format!("{}.{}", b64url().encode(payload.as_bytes()), b64url().encode(sig.to_bytes()));

    println!("LICENSE KEY for {email}{}\n", exp.map(|_| " (time-limited)").unwrap_or(" (perpetual)"));
    println!("{key}");
}
