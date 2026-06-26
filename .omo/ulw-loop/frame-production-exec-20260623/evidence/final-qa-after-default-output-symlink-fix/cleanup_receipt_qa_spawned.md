# Cleanup receipt: QA-spawned resources

QA run type: CLI/cargo command surfaces only. No browser, headless browser, worker, or tmux QA session was intentionally launched.
Working directory: /Users/shinyoohag/Desktop/frametrace

## QA-spawned process signature check

Invocation: `ps -Ao pid=,ppid=,stat=,comm=,args= | awk ...` matching cargo/rustc/frametrace/ulw-qa/playwright/agent-browser/headless automation signatures.

```text
70706 64265 S    /Applications/An /Applications/Antigravity.app/Contents/Frameworks/Antigravity Helper (Renderer).app/Contents/MacOS/Antigravity Helper (Renderer) --type=renderer --user-data-dir=/Users/shinyoohag/Library/Application Support/Antigravity --standard-schemes=plugin --secure-schemes=plugin --cors-schemes=plugin --fetch-schemes=plugin --service-worker-schemes=plugin --code-cache-schemes=plugin --app-path=/Applications/Antigravity.app/Contents/Resources/app.asar --enable-sandbox --remote-debugging-port=0 --lang=en-US --num-raster-threads=4 --enable-zero-copy --enable-gpu-memory-buffer-compositor-resources --enable-main-frame-before-activation --renderer-client-id=9 --time-ticks-at-unix-epoch=-1780057435326140 --launch-time-ticks=1640063147951 --shared-files --field-trial-handle=1718379636,r,8082883152059928182,1563366044759071778,262144 --enable-features=PdfUseShowSaveFilePicker,ScreenCaptureKitPickerScreen,ScreenCaptureKitStreamPickerSonoma --disable-features=DropInputEventsWhilePaintHolding,LocalNetworkAccessChecks,MacWebContentsOcclusion,ScreenAIOCREnabled,SpareRendererForSitePerProcess,TimeoutHangingVideoCaptureStarts,TraceSiteInstanceGetProcessCreation --variations-seed-version --pseudonymization-salt-handle=1935764596,r,4994419513748780301,15739741217365220305,4 --trace-process-track-uuid=3190708994745248135 --seatbelt-client=58
```

Verdict: PASS if the block above is empty. It is empty for this run, so no QA-spawned cargo/rustc/FrameTrace/headless/browser automation/worker process remains.

## QA tmux session check

Invocation: `tmux ls 2>&1 | grep ulw-qa || true`

```text
```

Verdict: PASS if empty. No QA tmux session remains.

## QA temp directory check

Invocation: `find "$EVID" -type d \( -name "tmp" -o -name ".tmp" -o -name "*temp*" \) -print`

```text
```

Verdict: PASS if empty. No QA-spawned temp directory remains under the evidence tree.

## Artifact directory listing

Invocation: `find "$EVID" -type f -maxdepth 1 -print -exec wc -c {} \;`

```text
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.receipt.md
   12826 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.txt
     511 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.receipt.md
     386 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.receipt.md
     969 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.txt
    2587 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.txt
       0 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_fmt_check.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.txt
   12417 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.txt
       0 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.receipt.md
    4127 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_locked_full_suite.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.txt
      72 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.receipt.md
     578 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cleanup_receipt.md
    6562 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cleanup_receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cleanup_receipt_qa_spawned.md
    9296 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cleanup_receipt_qa_spawned.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.txt
     696 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.receipt.md
    1157 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.receipt.md
    1163 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_media_contract_nocapture.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_clippy_locked_all_targets_all_features_D_warnings.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.txt
    3696 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_symlink_nocapture.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.receipt.md
     426 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.txt.exit
       2 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_output_policy_nocapture.txt.exit
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.receipt.md
    3078 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_derived_output_policy_tests_nocapture.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.receipt.md
     374 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_diff_check.receipt.md
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.txt
      41 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/git_rev_parse_HEAD.txt
/Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.txt
     666 /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix/cargo_test_cli_default_output_policy_nocapture.txt
```
