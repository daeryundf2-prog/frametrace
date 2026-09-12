#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

// The carve signature scanner walks raw bytes in 1 MiB chunks with a 32-byte
// overlap, computing absolute offsets from chunk bookkeeping. Arbitrary bytes
// must never cause a panic or out-of-bounds access.
fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);
    let _ = frametrace::carve::find_video_signatures_in(&mut cursor, 64);
});
