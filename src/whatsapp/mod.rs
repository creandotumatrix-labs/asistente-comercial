pub mod client;
pub mod types;

pub use client::WhatsappClient;
pub use types::{InboundPayload, InboundText};

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verify Meta's `X-Hub-Signature-256: sha256=<hex>` header against an
/// HMAC-SHA256 of the raw request body keyed by the app secret. Constant-time
/// comparison via `verify_slice`.
pub fn verify_signature(app_secret: &str, body: &[u8], header: &str) -> bool {
    let Some(expected_hex) = header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(expected) = hex::decode(expected_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&expected).is_ok()
}
