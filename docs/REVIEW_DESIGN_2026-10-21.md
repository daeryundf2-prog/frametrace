# 디자인 개선 작업 기능 리뷰 (2026-10-21, 작업 트리 미커밋 변경, 기준 849a5bf)

## 2차 확인 (현재 상태 — 1차 지적은 해소됨)
- 1차 리뷰 때(작업 중간본) 사라졌던 **class/id 149개 스타일이 모두 복구**됨. 지금은 HEAD 대비 선택자 집합이 같고(00-layout 237→237, controls 200→200, evidence_viewer 396→397, finalize·tools 동일), 지워진 선택자 0, 아직 쓰이는데 스타일 없는 이름 0.
- 메뉴/시트/모달 z-index, 토스트 `pointer-events`, display 방식 변경 없음(1차 때 지적한 층 순서·클릭 가로채기 문제 없음).
- 변경은 사실상 **재색칠**: 9개 파일, +186/−166. JS 3곳은 색 문자열만.

### 기능 검사 (이 상태의 사본 `../ft-design`, 별도 빌드)
- `check-css`(대비 88쌍 × 두 테마), `check-js`, `verify-contract-pin`, `verify-i18n`, `cargo build`, `cargo test` 통과.
- 하네스 26종 중 CI 목록에 있는 것 전부 통과: verify-buttons, verify-layout, check-a11y(브라우저 필수), verify-embed, verify-viewer-dialogs, verify-finalize-sheet, verify-start-screen, verify-player-keys, verify-player-popup-escape, verify-range-sync, verify-marks-sync, verify-marks-roundtrip, verify-viewer-undo, verify-facets, verify-audio-facet, verify-jobs-e2e, verify-shell-jobs, verify-live-index, verify-progress, verify-rebuild-regressions, verify-review-batch --unit, verify-join --live.
- `verify-virtual-grid` U17(5만 건 클릭·마크 200 ms): 첫 실행 실패(210/409 ms) → 재실행 통과(25/105 ms). 같은 시각 다른 빌드에서도 마크가 336 ms로 실패 — **PC 부하에 따른 흔들림**, 디자인 원인 아님.
- 실행 안 된 4개(`verify-viewer-popup`·`verify-topfilters`·`verify-download`는 실제 포트 보호로 거부, `verify-dual`은 인자 필요) — CI 목록 밖, 디자인 무관.
- 화면 갤러리 24장 × 두 테마(1366×768) 확인: 빈 상태·실행 중·프로젝트 메뉴·작업 시트·그리드·상세·인스펙터·단축키·클립·이어붙이기·다중 선택·토스트·결과물(차단 포함)·최종화 후·팝아웃. 깨진 배치 없음.

## 1차 리뷰 (작업 중간본, 지금은 해당 없음)
중간본에서는 CSS 재작성으로 class/id 149개 스타일이 사라지고 z-index 순서·토스트 클릭 통과가 바뀌어 있었음. 2차 확인 시점에 모두 원래 구조로 돌아옴.

## 남은 권고
- 이 작업은 PR #5 브랜치(`tools-bundle`) 작업 트리에서 **미커밋**으로 진행 중 — 별도 브랜치로 커밋해 PR #5와 섞이지 않게 할 것.
- 적대적 리뷰 결과는 아래에 이어서 기록.

## 적대적 리뷰 (Opus, `../ft-design` 사본, 대비 직접 계산 + 격리 서버 Playwright 1366×768·1920×1080 × 두 테마 + forced-colors + 요약 보고서 흑백 PDF)
**판정: 조건부 머지 차단** — Blocker 없음, High 3건(토큰·문서 몇 줄)을 고치고 check-css에 새 쌍을 넣으면 머지 가능.

| 심각도 | 위치 | 문제 | 수정 |
|---|---|---|---|
| High | `tokens.css:87,143` `--media-border` / `viewer/controls.css:110` | 미디어 툴바 버튼 테두리 대비 1.31·1.27:1 (기준 3:1, HEAD 2.47:1보다 나빠짐) — 버튼 경계가 안 보임 | 컨트롤 외곽선용 토큰 분리, 3:1 이상(예 `#6b737c`) |
| High | `viewer/controls.css:123` | 툴바 비활성 버튼이 글자색만 다름(활성 대비 1.73배 차이, 배경·테두리 동일, `.muted` 라벨과 같은 색) — 재생 불가 파일에서 IN/OUT 등 비활성 구분 어려움 | 비활성 글자 더 어둡게(≈3:1) + 점선/투명 테두리 |
| High | `DESIGN.md:19-47` | "Single Source of Truth"라면서 tokens.css와 값 대부분 불일치, 없는 토큰(`--surface-3`), **마크 의미 반대**(문서: 검토됨=초록·확인 필요=주황 / CSS: 검토됨=회색·중요=주황·확인 필요=보라) | tokens.css에서 표 재생성, 가능하면 check-css가 대조 |
| Medium | `scripts/check-css.mjs:128-146` | 새 토큰 14개(media-*, cal-l1~3, mark-hl)가 88쌍에 없음 → 위 High가 게이트 통과 | 쌍 추가 + 계약 §5 토큰 표 갱신 |
| Medium | `evidence_viewer.css:163,248` | 라이트 테마 영상 스테이지·썸네일 "다시 시도" 포커스 링 2.19/2.02:1 (`--media-ring`이 툴바에만 적용) | 미디어 영역 `:focus-visible`에 `--media-ring` |
| Medium | `viewer/70-player.js:295-296` | 팝아웃 목록 선택 항목에 포커스 링 = 배경색 → 1:1, 안 보임(실측) | 다른 색(`#eef0f2`) + offset |
| Medium | `app/00-layout.css:259-263` | 셸 토스트 정보·완료·경고·오류가 색으로만 구분, 고대비에서 동일; 뷰어 경고/오류 같은 "!" | 종류별 아이콘(✓ ! ✕) 또는 글자 접두어 |
| Low | `evidence_viewer.css:237-239` + `kit.css:161` | 고대비에서 현재 카드·활성 항목 구분 약함(기존) | `@media (forced-colors:active)` Highlight 외곽선 |
| Low | `evidence_viewer.css:317,319` | 카빙/후보 배지 휘도비 1.16 (적록색약 구분 어려움, 글자 있음) | 색상 차이 확대 |
| Low | `tokens.css:92-94,148-150` | 달력 단계 색 차이 작음(숫자 표시로 정보 유지) | 다크 l1 밝게 |
| Low | `viewer/70-player.js:286-307`, `evidence_viewer.css` 여러 곳 | 13px 미만 글자 남음 — DESIGN.md "13px 미만 금지"와 불일치; 팝아웃 영상 없는 항목 3.65:1 | 13px, `#8a8f95` 이상 |
| Low | `evidence_viewer.css:42` vs `:372` | 다크에서 경고 검증 항목 연한 배경 사라짐(대비는 충분) | 다크 규칙에 `.warn` |
| Low | `viewer/90-sync.js:2001-2004` | 요약 보고서 흑백 인쇄 읽힘·상태는 글자; 안내 상자 테두리 1.53:1, SHA 11px(기존) | `@media print` 테두리 검정, 12~13px |

문제없음: 두 해상도·두 테마 잘림 없음, 배지·토스트·형광펜·종류 배지·텔레메트리 대비 기준 통과, 레거시 밖 왼쪽 색 줄·옛 청록색 0건, reduced-motion 정상, check-css 88쌍 통과.

## 수정 기록 (2026-10-26, `design-refresh`)
대비는 check-css.mjs와 같은 WCAG 공식으로 계산(light / dark).

| 심각도 | 지적 | 조치 | 측정 |
|---|---|---|---|
| High | 툴바 버튼 테두리 1.31·1.27:1 | 새 토큰 `--media-control-border` `#6e747b`(양 테마)를 툴바 버튼·select, 썸네일 "다시 시도", 전체화면 나가기 버튼 외곽선에 적용. `--media-border`는 구분선 전용(장식)으로 남김 | media-surface 위 3.54 / 3.73, media-bg 위 3.84 / 4.04 |
| High | 비활성 툴바 버튼 구분 약함 | 새 토큰 `--media-disabled-fg` `#6a7077` + 투명 배경 + 점선 `--media-border` 테두리(활성은 실선 3:1). busy 비활성은 실선 유지. `disabled`/`aria-disabled` 동작은 그대로 | 글자 3.34 / 3.52(≥3), 활성 글자 14.64와 4.4배·`.muted` 라벨 8.46과 2.5배 차이 |
| High | DESIGN.md 불일치 | §2 토큰 표를 tokens.css에서 재생성(전 토큰, 새 media·cal·mark-hl 포함), `--surface-3`·`--on-danger` 제거, 마크 의미를 CSS대로(검토됨=회색 체크, 중요=호박/주황 깃발, 확인 필요=보라 물음표 — `evidence_viewer.css` `.mark-chip`·`#btnMark*`에서 확인), 글꼴 스택·`--fs-lg` 20px·`--lh-ctl` 1.35·destructive 버튼·active 1px 정정. check-css.mjs가 DESIGN.md의 토큰 이름·표 값을 tokens.css와 대조(불일치 시 실패) | — |
| Medium | check-css에 새 토큰 없음 | 16쌍 추가(88→104쌍 × 2테마): media-fg on media-bg/surface/hover/active, media-muted on surface/hover, media-ring·media-control-border on media-bg/surface(≥3), media-disabled-fg on surface(≥3, media-muted보다 1.5배 이상 어두워야 함), text on cal-l1~3, muted on cal-l1, mark-hl-fg on mark-hl-bg. 계약 §5.1 표·§5.2 수치 갱신 | 최저: media-disabled-fg 3.34, media-control-border 3.54, cal-l3 text 8.79(dark), mark-hl 7.23(dark) |
| Medium | 미디어 영역 포커스 링 2.19/2.02:1 | `.media-stage`(자신·안쪽), `.thumb` 안 컨트롤, `.fs-exit`의 `:focus-visible`을 `--media-ring` | light 8.51(media-bg) / 7.85(media-surface) |
| Medium | 팝아웃 선택 항목 포커스 링 1:1 | `.li.on:focus-visible` 링을 `#0f1a2b`, offset −4px(선택 배경 안쪽) | 8.19 on `#8db4ea` |
| Medium | 셸 토스트 색으로만 구분 | `.h-toast::before` 종류별 글리프 i / ✓ / ! / ✕(`content: … / ""`로 읽기 제외, DOM·aria 불변), forced-colors에서 글리프 테두리. 뷰어 오류 토스트 글리프 "!" → "✕"(경고는 "!") | — |
| Low | 고대비 현재 카드·활성 항목 | `@media (forced-colors: active)`: `.card.active`, `.tree-item.active`, `.rail-chip.active`에 `3px double Highlight`(포커스 시 실선) | — |
| Low | 카빙/후보 배지 휘도비 1.16 | carved `#9a4c0a`, candidate `#701812` | 휘도비 1.87, 흰 글자 6.15 / 13.6 이상 |
| Low | 달력 단계 차이 | 다크 `--cal-l1-bg` `#182232`→`#1c2739` | surface 대비 1.05→1.12, 글자 11.82 |
| Low | 13px 미만·팝아웃 영상 없는 항목 | 팝아웃(70-player.js) 글자 11·12px→13px, `.li.novid` `#6f7276`→`#8a8f95`, 팝아웃 버튼 테두리 `#33383d`(1.41)→`#6e747b`. `evidence_viewer.css`의 12px 11곳(날짜 입력·레일 칩·필터 칩·선택 막대·메뉴·태그 입력·타임라인·대화상자 필드)은 **보류** — 상당수가 뒤에 오는 controls.css/kit.css에 덮이고, 고정 폭(날짜 입력 136px 등)·칩 줄바꿈에 영향이 있어 배치 변경 금지 범위 밖. 별도 작업으로 실측 후 조정 | novid 5.41 (hover 4.68) |
| Low | 다크 경고 검증 항목 배경 | 다크 규칙에 `.validation-item.warn { background: var(--warn-bg) }` | — |
| Low | 요약 보고서 인쇄 | 안내 상자 테두리 `#e3cf9e`→`#9a7b2c`, `@media print` 표·상자 테두리 검정, SHA 11→12px | 1.43→3.72 |

검증: `check-css`(104쌍 × 2테마 + DESIGN.md 대조), `check-js`, `verify-i18n`, `verify-contract-pin`, `cargo build --locked`, `cargo test --locked`(lib 1243 통과 등 전부 통과) 통과. 하네스(격리 서버, 별도 빌드): verify-buttons, verify-layout, check-a11y, verify-embed, verify-finalize-sheet, verify-start-screen, verify-player-keys, verify-player-popup-escape, verify-shell-jobs(19), verify-progress(16), verify-rebuild-regressions 통과. verify-viewer-dialogs는 첫 실행에서 `ft-theme` ack 1건이 300 ms 대기 안에 안 와 실패(색과 무관한 타이밍) → 재실행 2회 연속 통과. 하네스 기대값 수정 없음.

### 확인 리뷰 후 2차 수정 (59f3b37 대상)
| 심각도 | 지적 | 조치 | 측정 |
|---|---|---|---|
| Medium(회귀) | 팝아웃 선택된 영상 없는 항목 글자 1.53:1 (`.li.novid`가 `.li.on` 뒤) | `.li.on.novid{color:#0f1a2b}` 추가 | 8.19 on `#8db4ea` |
| Medium | 라이트 툴바 `.on` 토글(듀얼·프록시) 채움 `--accent`가 막대 위 2.02:1 | `#mediaToolbar` `.on`과 `.media-toolbar button.on` 테두리를 `--media-ring`으로(채움은 그대로). check-css에 `--media-ring` on `--media-hover` 쌍 추가(media-surface 쌍은 기존) | 경계 7.85 / 8.27(막대), 6.78 / 7.15(hover) |
| Medium | DESIGN.md 대조가 개수 불일치 행을 조용히 건너뜀 | `--` 행에서 이름·light·dark 값 개수가 다르면 실패, 역방향으로 tokens.css의 모든 `#rrggbb` 토큰이 표에 행을 가져야 함. `%TEMP%` 변형 사본 5개로 확인: 행 삭제, 백틱 없는 값, 값 개수 부족, 값 변경, 없는 토큰 → 모두 rc=1(원본 rc=0) | — |
| Low | 비활성 점선 테두리 `--media-border` 1.31:1(안 보임) | 새 토큰 `--media-disabled-border` `#4a5057`; check-css가 ≥1.5이고 컨트롤 외곽선보다 낮은지 검사 | 2.05 / 2.16 (외곽선 3.54 / 3.73) |
| Low | 다크 달력 l1→l2 단계 1.10으로 줄어듦 | 다크 `--cal-l2-bg` `#223352`, `--cal-l3-bg` `#2c4569`; check-css가 surface→l1→l2→l3 단계가 줄지 않는지 검사 | 다크 단계 1.12 / 1.19 / 1.30 (라이트 1.08 / 1.12 / 1.17), 글자 9.94 / 7.64 |
| Low | DESIGN.md 13px 주장·현재 카드 서술 | 남은 12px 위치를 예외 목록으로 명시, 현재 카드는 "1px 테두리 `--accent` + 2px 그림자 링 + ▶ 배지(+고대비 double Highlight)"로 정정 | — |

check-css는 105쌍 × 2테마.

검증(2차): `check-css`(105쌍 × 2테마 + DESIGN.md 양방향 대조), `check-js`, `cargo build --locked` 통과. 하네스: verify-buttons, verify-layout, check-a11y, verify-player-popup-escape, verify-viewer-dialogs 모두 통과(1회 실행, 재시도 없음).

### tools-bundle 03fd739(PR #6 포함) 병합 후 디자인 정리
- PR #6 `81-picker-plus.css`는 이미 토큰만 사용(옛 청록·하드코딩 색·왼쪽 색줄 없음). 보강: 선택된 목록 행은 배경만이 아니라 1px `--accent` 외곽선(호버 행과 구분; 대비 6.89 / 5.79 on accent-soft), 바로 가기 활성 항목 inset 1px `--accent`, 고대비에서 둘 다 2px Highlight 외곽선, 끌어놓기 영역 테두리 Highlight, 경로 상태(ok/warn/err)는 연한 배경색 외에 글리프 ✓ / ! / ✕ 접두(읽기 제외, DOM 불변)와 고대비 테두리.
- 폴더 구조 트리(`60-details.js`): 이모지 📂📁🛡️(컬러 글리프, 인라인 `font-size:12px`)를 스프라이트 `#i-folder` 단색 아이콘(`.tree-ic`, 평소 `--muted`, 활성 시 글자색)으로 교체. 선택자·data 속성·들여쓰기 그대로.
- 56-tools round 16(교체 확인문·감사 경고)은 기존 `.h-tool*` 토큰 스타일과 아이콘+sr-only 단어를 그대로 씀 — 변경 없음.
- DESIGN.md 12px 예외 목록에 기존 `#customTagInput`(인라인 12px, 고정 104px) 추가.
- 검증: check-css(105쌍), check-js, verify-i18n(773키), verify-contract-pin, cargo build/test --locked(lib 1276 통과), `scripts/ci-harnesses.sh` 27종 전부 통과(verify-facets·verify-audio-facet 포함, 총 1619 s).
