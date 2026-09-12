//! HMAC-SHA-256 (RFC 2104) assembled from the vetted `sha2` crate rather
//! than a new dependency — the audit chain's keyed mode (design:
//! `docs/audit-hmac-design.md`) only needs the message-authentication
//! construction, and building it over `sha2::Sha256` keeps the trust base
//! identical to the existing unkeyed digests.
//!
//! HMAC is a symmetric MAC, not a signature: anyone holding the key can
//! forge entries. The construction here follows RFC 2104 exactly —
//! `H(K XOR opad || H(K XOR ipad || message))` with a 64-byte block —
//! and the module's tests are the RFC 4231 HMAC-SHA-256 vectors.

use sha2::Digest;

/// SHA-256 compression block size in bytes (RFC 2104 §2, B=64).
const BLOCK_BYTES: usize = 64;
/// SHA-256 output size in bytes.
const TAG_BYTES: usize = 32;

/// Computes `HMAC-SHA-256(key, message)` and returns the 32-byte tag.
///
/// Keys longer than the block size are first hashed down per RFC 2104;
/// shorter keys are zero-padded. No key length is rejected — the caller
/// (`audit_key`) enforces the 32-byte policy for configured keys.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; TAG_BYTES] {
    let mut key_block = [0u8; BLOCK_BYTES];
    if key.len() > BLOCK_BYTES {
        key_block[..TAG_BYTES].copy_from_slice(&sha2::Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_BYTES];
    let mut opad = [0x5cu8; BLOCK_BYTES];
    for index in 0..BLOCK_BYTES {
        ipad[index] ^= key_block[index];
        opad[index] ^= key_block[index];
    }

    let mut inner = sha2::Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = sha2::Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().into()
}

/// `hmac_sha256` rendered as lowercase hex, matching the audit chain's
/// digest formatting.
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    let tag = hmac_sha256(key, message);
    let mut out = String::with_capacity(TAG_BYTES * 2);
    for byte in tag {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Constant-time tag comparison. Timing side channels matter little for a
/// local file verifier, but a forensic tool should not ship a MAC check
/// that early-exits on the first differing byte.
pub fn tags_equal(a: &[u8; TAG_BYTES], b: &[u8; TAG_BYTES]) -> bool {
    let mut diff = 0u8;
    for index in 0..TAG_BYTES {
        diff |= a[index] ^ b[index];
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{hmac_sha256_hex, tags_equal};

    fn hex_decode(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2));
        (0..text.len() / 2)
            .map(|index| u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).unwrap())
            .collect()
    }

    fn check(key: &[u8], message: &[u8], expected_hex: &str) {
        assert_eq!(hmac_sha256_hex(key, message), expected_hex);
    }

    /// RFC 4231 test case 1: 20-byte key of 0x0b, "Hi There".
    #[test]
    fn rfc4231_case_1() {
        check(
            &[0x0b; 20],
            b"Hi There",
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        );
    }

    /// RFC 4231 test case 2: key "Jefe", "what do ya want for nothing?".
    #[test]
    fn rfc4231_case_2() {
        check(
            b"Jefe",
            b"what do ya want for nothing?",
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        );
    }

    /// RFC 4231 test case 3: 20-byte key of 0xaa, 50 bytes of 0xdd.
    #[test]
    fn rfc4231_case_3() {
        check(
            &[0xaa; 20],
            &[0xdd; 50],
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
        );
    }

    /// RFC 4231 test case 4: key 0x01..0x19, 50 bytes of 0xcd.
    #[test]
    fn rfc4231_case_4() {
        let key: Vec<u8> = (1u8..=25).collect();
        check(
            &key,
            &[0xcd; 50],
            "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
        );
    }

    /// RFC 4231 test case 6: 131-byte key (longer than the block, so the
    /// key-hashing path runs) over "Test Using Larger Than Block-Size...".
    #[test]
    fn rfc4231_case_6() {
        check(
            &[0xaa; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First",
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
        );
    }

    /// RFC 4231 test case 7: 131-byte key over the longer multi-line
    /// message — exercises the hashed-key path a second time.
    #[test]
    fn rfc4231_case_7() {
        check(
            &[0xaa; 131],
            b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.",
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
        );
    }

    /// A hex-decoded key round-trips through the same code path as a byte
    /// key (covers the decode helper used by other vectors).
    #[test]
    fn hex_decoded_key_matches_byte_key() {
        let key = hex_decode("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
        check(
            &key,
            b"Hi There",
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        );
    }

    #[test]
    fn tags_equal_is_exact() {
        let a = super::hmac_sha256(b"k", b"m");
        let mut b = a;
        assert!(tags_equal(&a, &b));
        b[31] ^= 1;
        assert!(!tags_equal(&a, &b));
    }
}
