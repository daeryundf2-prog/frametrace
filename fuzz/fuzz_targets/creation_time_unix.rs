#![no_main]

use libfuzzer_sys::fuzz_target;

// The timeline creation_time parser is a hand-rolled ISO-8601 subset that
// indexes into the string by position (YYYY-MM-DD[T ]HH:MM:SS...). Arbitrary
// strings — including non-UTF-8 input — must return None, never panic.
fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let _ = frametrace::timeline::parse_creation_time_unix(&text);
});
