//! SHA-256 digests via the vetted `sha2` crate (RustCrypto).
//!
//! Earlier revisions carried a hand-rolled compression function; for a
//! forensic product a widely reviewed implementation is the more
//! defensible choice. The public surface (`digest_reader`,
//! `digest_bytes`) and the lowercase-hex output are unchanged, so every
//! recorded digest remains byte-identical.

use sha2::Digest;
use std::io::{self, Read};

/// Streams `reader` through SHA-256 and returns the lowercase hex digest.
pub fn digest_reader<R: Read>(mut reader: R) -> io::Result<String> {
    let mut hasher = sha2::Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_lower(&hasher.finalize()))
}

/// Hashes `bytes` in one shot and returns the lowercase hex digest.
pub fn digest_bytes(bytes: &[u8]) -> String {
    hex_lower(&sha2::Sha256::digest(bytes))
}

fn hex_lower(digest: &[u8]) -> String {
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{digest_bytes, digest_reader};
    use std::io::{Cursor, Read};

    /// Reader that never returns more than `chunk` bytes per `read` call,
    /// so the streaming path is exercised across arbitrary boundaries.
    struct Chunked<'a> {
        data: &'a [u8],
        position: usize,
        chunk: usize,
    }

    impl Read for Chunked<'_> {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let remaining = self.data.len() - self.position;
            let take = remaining.min(out.len()).min(self.chunk);
            out[..take].copy_from_slice(&self.data[self.position..self.position + take]);
            self.position += take;
            Ok(take)
        }
    }

    #[test]
    fn hashes_empty_input() {
        let digest = digest_reader(Cursor::new(Vec::<u8>::new())).unwrap();
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn hashes_identically_across_chunk_boundaries() {
        let data: Vec<u8> = (0..200_000u32).map(|v| (v % 251) as u8).collect();
        let expected = digest_bytes(&data);
        for chunk in [1usize, 3, 63, 64, 65, 1000, 65536] {
            let digest = digest_reader(Chunked {
                data: &data,
                position: 0,
                chunk,
            })
            .unwrap();
            assert_eq!(digest, expected, "mismatch at chunk size {chunk}");
        }
    }

    #[test]
    fn hashes_abc() {
        let digest = digest_reader(Cursor::new(b"abc")).unwrap();
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(digest_bytes(b"abc"), digest);
    }
}
