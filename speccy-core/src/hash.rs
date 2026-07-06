//! SHA-256 hashing helpers.
//!
//! `sha2` 0.11's digest output no longer implements `LowerHex`, so hex
//! encoding lives here rather than being repeated at each call site.
//!
//! # Examples
//!
//! ```
//! use speccy_core::hash::sha256_prefixed;
//!
//! assert!(sha256_prefixed(b"speccy").starts_with("sha256:"));
//! ```

use sha2::Digest;
use sha2::Sha256;

/// Lowercase hex encoding of a byte slice: two characters per byte.
///
/// # Examples
///
/// ```
/// use speccy_core::hash::to_hex;
///
/// assert_eq!(to_hex(&[0x00, 0x0f, 0xff]), "000fff");
/// ```
#[must_use = "the hex-encoded string is the result of the computation"]
pub fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(hex_digit(b >> 4));
        out.push(hex_digit(b & 0x0f));
    }
    out
}

/// SHA-256 of `bytes`, truncated to its first `n` digest bytes and lowercase
/// hex-encoded (`2 * n` characters). Used for short, collision-tolerant
/// directory keys; `n` is clamped to the 32-byte digest length.
///
/// # Examples
///
/// ```
/// use speccy_core::hash::short_hex;
///
/// assert_eq!(short_hex(b"speccy", 3).len(), 6);
/// ```
#[must_use = "the hex-encoded string is the result of the computation"]
pub fn short_hex(bytes: &[u8], n: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let take = n.min(digest.len());
    to_hex(digest.get(..take).unwrap_or_else(|| digest.as_slice()))
}

/// SHA-256 of `bytes` as a `sha256:`-prefixed lowercase hex digest.
///
/// # Examples
///
/// ```
/// use speccy_core::hash::sha256_prefixed;
///
/// let digest = sha256_prefixed(b"");
/// assert_eq!(
///     digest,
///     "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
/// );
/// ```
#[must_use = "the digest string is the result of the computation"]
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", to_hex(&hasher.finalize()))
}

/// Map a 4-bit nibble to its lowercase hex character.
fn hex_digit(nibble: u8) -> char {
    // `nibble` is always in `0..16`, so `from_digit` never returns `None`;
    // the fallback keeps the function total without a panic.
    char::from_digit(u32::from(nibble), 16).unwrap_or('0')
}
