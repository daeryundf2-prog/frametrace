# Cleanup receipt

Working directory: /Users/shinyoohag/Desktop/frametrace
Evidence directory: /Users/shinyoohag/Desktop/frametrace/.omo/ulw-loop/frame-production-exec-20260623/evidence/final-qa-after-default-output-symlink-fix

## Process check

Invocation: `ps -Ao pid=,comm= | egrep "(cargo|rustc|frametrace|Chrome|Chromium|chromium|playwright|agent-browser|ulw-qa)" || true`

```text
  689 /Applications/Google Chrome.app/Contents/MacOS/Google Chrome
  891 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/chrome_crashpad_handler
  930 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
  943 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
  952 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
 3246 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
 3488 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
 5631 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
 7765 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
26855 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
37922 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
39518 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
39528 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
58371 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
61419 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper
61466 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
62347 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
79967 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
81689 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
84510 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
85000 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
86734 /Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/148.0.7778.168/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)
```

Interpretation: empty output means no matching QA-spawned command/browser/worker processes remained after execution.

## tmux check

Invocation: `tmux ls 2>&1 || true`

```text
dfl_strategy_wave10_hard_examples_qa: 1 windows (created Thu Jun  4 21:31:04 2026)
gajae_code: 1 windows (created Fri May 29 12:47:28 2026)
gajae_mimo: 1 windows (created Sat May 30 20:48:18 2026)
task1_inv_t: 1 windows (created Wed Jun  3 18:24:37 2026)
task4_leader_privacy_qa: 1 windows (created Mon Jun  1 18:51:48 2026)
```

Interpretation: no ulw-qa tmux session was created for this CLI-shaped QA run; no leftover QA tmux session is present in the output above.

## Evidence temp-dir check

Invocation: `find "$EVID" -type d \( -name "tmp" -o -name ".tmp" -o -name "*temp*" \) -print`

```text
```

Interpretation: empty output means no QA-spawned temp directory remains under the evidence tree.

## Git status check

Invocation: `git status --short --branch`

```text
## codex/frametrace-forensic-hardening...origin/codex/frametrace-forensic-hardening [ahead 8]
?? .omo/plans/
?? .omo/ulw-loop/.current-media-session
?? .omo/ulw-loop/frame-full-ga-20260617222935/
?? .omo/ulw-loop/frame-gui-20260617102845/
?? .omo/ulw-loop/frame-master-rcga-cleanup-receipt.txt
?? .omo/ulw-loop/frame-master-rcga-review-blocker.md
?? .omo/ulw-loop/frame-media-validation-20260617024104/
?? .omo/ulw-loop/frame-production-exec-20260623-brief.md
?? .omo/ulw-loop/frame-production-exec-20260623/
?? .omo/ulw-loop/frame-production-seq-20260623-brief.md
?? .omo/ulw-loop/frame-production-seq-20260623/
?? .omo/ulw-loop/frame-windows-prereq-gate-20260622/
?? .omo/ulw-loop/frame-winui-20260617-playback-cli.txt
?? .omo/ulw-loop/frame-winui-cleanup-receipt.txt
?? .omo/ulw-loop/frame-winui-latest-playback-case.txt
?? .omo/ulw-loop/frame-winui-playback-cli-pass.txt
?? .omo/ulw-loop/frame-winui-workstation-status-release.json
?? .omo/ulw-loop/frame-winui-workstation-status.json
```
