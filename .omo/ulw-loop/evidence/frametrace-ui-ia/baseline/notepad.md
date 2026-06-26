Skills:
- playwright: required for real Chromium/browser screenshots and DOM assertions.

Tier: LIGHT - read-only first-screen browser QA baseline, no product changes.
Success criteria:
- Capture screenshots at 1280x900, 768x900, 375x900 via Chromium for http://127.0.0.1:4177/index.html.
- Assert required visible first-screen concepts and horizontal overflow at 375/768, with JSON/log evidence and cleanup receipt.

Execution:
- Served gui/evidence-viewer at http://127.0.0.1:4177/index.html using python3 -m http.server 4177 --directory gui/evidence-viewer.
- Ran Chromium via NODE_PATH=/Users/shinyoohag/.npm/_npx/420ff84f11983ee5/node_modules npx --yes -p @playwright/test playwright test .omo/ulw-loop/evidence/frametrace-ui-ia/baseline/baseline.spec.js --browser=chromium --workers=1 --reporter=line --output=.omo/ulw-loop/evidence/frametrace-ui-ia/baseline/test-results.
- Result: browser scenarios executed at 1280x900, 768x900, 375x900; 24 required concept assertions failed; 768 and 375 horizontal overflow failed.
- Cleanup: server terminal session 81141 stopped by Ctrl-C; lsof found no listener on 4177; Playwright closed browser contexts.

Self-review:
- Artifact sanity checked with jq and wc -c; every PASS/FAIL entry references non-empty artifacts.
- Scope check: product files were not edited; evidence writes are under the requested baseline directory.
