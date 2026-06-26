# Command receipt: cargo_test_cli_output_policy_nocapture

Surface: CLI
Working directory: /Users/shinyoohag/Desktop/frametrace
Invocation: `cargo test --locked --test cli_output_policy -- --nocapture`
Exit code: 0
Raw exact output artifact: `/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.txt`
Raw exact output byte count: 696

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running tests/cli_output_policy.rs (target/debug/deps/cli_output_policy-0625b628ff4fa6eb)

running 5 tests
test make_review_rejects_symlinked_review_directory_without_writing_target ... ok
test scan_folder_rejects_symlinked_db_directory_without_writing_case_state_outside ... ok
test make_review_rejects_symlinked_review_outputs_without_writing_target ... ok
test make_report_rejects_symlinked_reports_directory_without_writing_target ... ok
test make_report_rejects_symlinked_report_outputs_without_writing_target ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s


```
