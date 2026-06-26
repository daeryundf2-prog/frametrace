# Command receipt: cargo_test_cli_default_output_policy_nocapture

Surface: CLI
Working directory: /Users/shinyoohag/Desktop/frametrace
Invocation: `cargo test --locked --test cli_default_output_policy -- --nocapture`
Exit code: 0
Raw exact output artifact: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.txt`
Raw exact output byte count: 666

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/cli_default_output_policy.rs (target/debug/deps/cli_default_output_policy-e6ec3e8fd6391d9f)

running 4 tests
test package_case_rejects_symlinked_default_reports_directory_without_writing_outside ... ok
test carve_file_rejects_symlinked_default_carved_directory_without_writing_outside ... ok
test export_video_rejects_symlinked_default_clip_directory_without_writing_outside ... ok
test derived_media_commands_reject_symlinked_default_directories_without_writing_outside ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s


```
