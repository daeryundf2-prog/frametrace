use std::env;
use std::fs;
use std::path::Path;

/// Bundles `assets/viewer/*.js` into one classic script for the
/// single-file evidence viewer. Source lives split per concern, but the
/// deliverable must stay a plain concatenation — ES module imports do
/// not resolve under file:// where the exported review page is opened.
fn main() {
    let viewer_dir = Path::new("assets/viewer");
    println!("cargo:rerun-if-changed={}", viewer_dir.display());

    let mut parts: Vec<_> = fs::read_dir(viewer_dir)
        .expect("assets/viewer missing")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().is_some_and(|ext| ext == "js")).then_some(path)
        })
        .collect();
    // Numeric filename prefixes define the concatenation order — the
    // viewer is one IIFE, so declaration order is load-bearing.
    parts.sort();

    let mut bundle = String::new();
    for path in &parts {
        println!("cargo:rerun-if-changed={}", path.display());
        bundle.push_str(&fs::read_to_string(path).expect("viewer part unreadable"));
        if !bundle.ends_with('\n') {
            bundle.push('\n');
        }
    }

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("evidence_viewer.js");
    fs::write(&out, bundle).expect("failed to write viewer bundle");
}
