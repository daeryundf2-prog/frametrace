#![no_main]

use libfuzzer_sys::fuzz_target;

// The hash-chained audit log is parsed line-by-line with a mix of serde_json
// field extraction and hand-rolled text surgery (entry_sha256 placement,
// previous-hash chaining). Arbitrary bytes model a corrupt, torn, or
// adversarially edited log: verification must return Ok/Err, never panic.
fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = frametrace::audit::verify_chained_jsonl_text(&text, "fuzz-input");
});
