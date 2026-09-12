#![no_main]

use libfuzzer_sys::fuzz_target;

// The videos.jsonl index read path (scan.rs::load_existing_record_lines)
// splits records with a hand-rolled depth/string/escape scanner, parses each
// record through serde, and later edits serialized lines in place with the
// byte-stable set_json_field stale-marker writer. All three stages must
// survive arbitrary input without panicking.
fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    for line in frametrace::scan::json_record_lines(&text) {
        let _ = serde_json::from_str::<frametrace::model::VideoRecord>(&line);
        let edited = frametrace::scan::set_json_field(&line, "index_status", "\"stale\"");
        let _ = frametrace::scan::set_json_field(&edited, "stale_since_unix", "0");
    }
});
