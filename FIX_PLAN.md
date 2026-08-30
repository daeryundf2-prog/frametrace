# FrameTrace 수정계획 (2026-08-30 리뷰 기반)

> **[후계됨]** 이 문서의 후속 계획은 `docs/ROADMAP.md`입니다(2026-08-30 재채점 7.3 반영).
> 아래 항목 중 Phase 0/F1-1~4/F1-7/F2-7/F7-1 및 런처·보고서 확장은 이미 완료되어
> ROADMAP §1 스냅샷에 기록되어 있습니다. 남은 항목은 ROADMAP M1~M3에 재배치되었습니다.

> 전제: 리뷰 시점 main HEAD = `933f223`. 로컬에 1건의 핫픽스가 이미 적용되어 있음
> (`assets/evidence_viewer.css` 선두 stray `"` 제거 — `git diff` 참고).
> 공수: S=30분 이하, M=반나절, L=1일 이상.

---

## Phase 0 — 긴급 핫픽스 (즉시, 배포 산출물 품질 문제)

### F0-1. [P0] CSS stray quote 커밋 — 공수 S
- **문제**: `assets/evidence_viewer.css:1` 선두의 `"`가 CSS 문자열 토큰을 열어 토큰 정의(`:root`)와
  스타일시트 앞부분을 무효화. 생성물 `review/evidence-viewer.html`의 테마/레이아웃이 깨짐
  (강제 다크모드 브라우저에서는 카드 메타 텍스트가 시각적으로 소멸 — 실측 확인).
- **조치**: 로컬에 이미 수정됨(선두 `"` 삭제). `git add assets/evidence_viewer.css && git commit`으로 확정.
- **검증**: `make-review` 재실행 → `review/evidence-viewer.html` 열어서
  배경 `#eef1f0`, 본문 `#1f2724`, 좌우 3:2 그리드 확인.
  (개발자 콘솔에서 `getComputedStyle(document.body).backgroundColor === "rgb(238, 241, 240)"`)

### F0-2. [P0] CSS 검증 파이프라인 부재 — 공수 M
- **문제**: CI(`.github/workflows/windows-ci.yml`)의 프런트 검증이 `node --check`(JS 문법)뿐이라
  CSS 파싱 오류가 통과됨. F0-1이 45커밋 동안 생존한 이유.
- **조치** (둘 다 권장):
  1. **최소 게이트(무의존성)** — `src/html_report.rs` 테스트에 추가:
     ```rust
     #[test]
     fn viewer_css_starts_with_root_rule() {
         assert!(VIEWER_CSS.trim_start().starts_with(':'),
             "viewer css must not begin with a stray string token");
     }
     ```
  2. **CSSOM 파싱 게이트** — CI `node --check` 스텝 옆에 `scripts/check-css.mjs` 추가:
     ```js
     // Node 21+ 내장 CSSOM 대안: 규칙 균형 + :root 존재 기계 검증
     import { readFileSync } from "node:fs";
     for (const f of ["assets/evidence_viewer.css"]) {
       const css = readFileSync(f, "utf8");
       const open = (css.match(/{/g) ?? []).length, close = (css.match(/}/g) ?? []).length;
       if (open !== close) throw new Error(`${f}: unbalanced braces ${open}/${close}`);
       if (!css.trimStart().startsWith(":root")) throw new Error(`${f}: :root rule missing`);
     }
     ```
     (정교하게 하려면 devDependency `lightningcss` 또는 `css-tree`로 full parse — 옵션)
- **검증**: CI 녹색 + 의도적으로 `"`를 넣었을 때 CI 적색 확인(레드 테스트).

---

## Phase 1 — 프런트 P1 버그 (뷰어/대시보드)

### F1-1. [P1] 대시보드 sticky thead 위치 버그 — 공수 S
- **문제**: `src/html_report.rs:103-105`의 `th { position: sticky; top: 76px; }`가 Chromium에서
  헤더를 첫 데이터 행 아래로 밀어냄(실측: th y=367 vs 첫 행 y=330, scrollY=0).
- **조치**: 테이블을 스크롤 컨테이너로 감싸고 컨테이너 기준 sticky로 변경:
  - CSS: `.table-wrap { overflow: auto; max-height: calc(100vh - 220px); }`,
    `th { position: sticky; top: 0; }`
  - 마크업(html_report.rs:305): `wrap.innerHTML = `<div class="table-wrap"><table>…</table></div>`
- **검증**: 1,000행 스트레스 데이터에서 헤더가 테이블 최상단 유지 + 스크롤 시 헤더 고정 확인.

### F1-2. [P1] 뷰어 빈-레코드 크래시 + els 맵 정리 — 공수 S
- **문제**: `assets/evidence_viewer.js:731`가 `els.detailBadges`를 참조하는데 els 맵(332-366)에
  키가 없음 → 레코드 0개면 TypeError로 뷰어 전체 다운.
  또한 `recordGrid` 키가 344/359 두 번 정의(후발 선승, 무해하지만 정리 대상).
- **조치**:
  ```js
  // els 정의에 추가
  detailBadges: document.getElementById("detailBadges"),
  // 359행의 recordGrid 중복 정의 삭제
  ```
- **검증**: 빈 `videos` 배열로 `__FRAMETRACE_DATA__`를 넣은 테스트 페이지에서
  "색인된 증거가 없습니다." 폴백이 콘솔 에러 없이 표시되는지.

### F1-3. [P1] Enter 단축키 no-op — 공수 S
- **문제**: 모달(`assets/evidence_viewer.html:147`)은 Enter="현재 증거 미리보기"라고 약속하지만
  `evidence_viewer.js:1034`는 `render()`만 호출(실질 no-op).
- **조치** (택1, 권장은 a):
  - (a) `case "Enter": els.mediaStage.querySelector("video")?.play().catch(() => {}); break;`
    — 재생으로 구현. 영상이 없으면 무시.
  - (b) 모달에서 Enter 행 삭제로 실제 동작과 일치.
- **검증**: j/k로 이동 후 Enter → 영상 재생 시작.

### F1-4. [P1] Space 키가 포커스된 버튼을 가로챔 — 공수 S
- **문제**: `keydown` 가드(1023-1027)가 INPUT/SELECT/TEXTAREA만 예외. 버튼 포커스 + Space →
  버튼 활성화 대신 증거 선택 토글(접근성 위반).
- **조치**: 가드에 추가 —
  ```js
  if (tag === "BUTTON" && (event.key === " " || event.key === "Enter")) return;
  ```
- **검증**: 어떤 버튼이든 Tab으로 포커스 → Space로 버튼이 눌리는지.

### F1-5. [P2] 죽은 코드/중복 제거 — 공수 S
- `evidence_viewer.js:776` `insertAdjacentHTML("afterend", "")` 제거.
- `evidence_viewer.js:589` 미사용 `tags` 변수 제거(602-604 `tagsHtml`과 중복).
- `evidence_viewer.css`: 87-106행 블록(=49-55행 중복), `.pager` 2회(55/128), `.time-text` 2회(154/158) 정리.
  ※ F0-1 이후 "안 적용되길래 아래에 복붙"으로 생긴 흔적이므로, 제거 후 시각 회귀 확인 필수.
- `evidence_viewer.css:82-84,145` `body.stack` — 구현되지 않은 모드. 제거하거나 토글 구현(보류 권장).
- **검증**: 시각 회귀 = 수정 전/후 스크린샷 비교(동일 데이터).

### F1-6. [P2] JS 측 file_url 인코딩 불일치 — 공수 M
- **문제**: `evidence_viewer.js:127-138` `fileUrl()`이 `encodeURI` 사용 → `#`/`?` 포함 경로에서
  URL 절단. Rust 측(`util.rs:195-206`)은 바이트별 인코딩으로 이미 올바름. carved/filesystem 레코드만
  JS가 URL을 자체 생성.
- **조치** (권장 a):
  - (a) `cli/handlers.rs`에서 carve/filesystem 로그를 읽을 때 Rust `path_to_file_url`로 계산한
    `file_url` 필드를 주입 → JS `fileUrl()` 제거(원본 video와 동일 경로).
  - (b) 단기 패치: JS에서 `encodeURI` 대신 세그먼트별 `encodeURIComponent("/".join)` 처리.
- **검증**: `file#name.mp4`, `파일 명.mp4` 등 특수문자 파일에서 영상 재생.

### F1-7. [P2] 강제 다크모드 방어 — 공수 S
- **문제**: 뷰어는 라이트 테임 전용인데 `color-scheme` 미선언 → Chromium 자동 다크가 색을 반전해
  가독성 붕괴(실측).
- **조치**: `assets/evidence_viewer.html` head에
  `<meta name="color-scheme" content="light">` + CSS `:root { color-scheme: light; }`.
  대시보드(html_report.rs)도 동일.
- **검증**: 다크모드 에뮬레이션에서 배경/텍스트가 토큰 값 그대로 유지되는지.

---

## Phase 2 — 포렌식 신뢰성 (엔진)

### F2-1. [P1] 감사로그 append 원자화·동시성 — 공수 M
- **문제**: `src/audit.rs:20-44`가 전체 로그 read → `write_text`(=`fs::write`, util.rs:37-42)로
  재작성. 크래시 시 체인 단손, 동시 프로세스(뷰어+CLI)에서 항목 유실 가능. 체인감사가
  셀링포인트인 제품의 최약점.
- **조치**:
  1. 쓰기를 `OpenOptions::new().create(true).append(true).open()` + 단일 `write_all`로 변경
     (읽기는 마지막 줄 해시 계산용으로 유지).
  2. 크로스프로세스 배타락: crate `fs2`로 `<log>.lock` 파일 잠금(Windows LockFileEx 매핑).
  3. 쓰기 후 `file.sync_data()`(fsync).
  4. `verify-audit`에 "마지막 줄이 불완전하면(NOT 개행 종결) 경고 + 잘라내기 제안" 정책 추가.
- **테스트**: ① 기존 체인 테스트 유지 ② 파일 중간을 절단한 상태에서 append → verify 시나리오
  ③ 두 스레드(스레드는 같은 프로세스라 락 검증엔 부족 → 자식 프로세스 2개 spawn 테스트) 동시 append.

### F2-2. [P1] 외부 프로세스 타임아웃 — 공수 M
- **문제**: ffmpeg/ffprobe/ewf*/icat 호출 전체에 타임아웃 없음(코드베이스 유일 타임아웃은 SQLite
  busy_timeout). 악성/손상 미디어 파싱 hang 시 케이스 전체 정지.
- **조치**: crate `wait-timeout` 도입.
  - 프로브류(ewfinfo/ewfverify/ffprobe): 기본 타임아웃 120s, `--timeout` 플래그로 조정.
  - 변환류(ffmpeg/ewfexport/icat): 기본 무제한, `--timeout` 옵션 제공(대용량 정상 처리 보호).
  - 타임아웃 시: 자식 kill → 부분 출력 파일 삭제(tsk.rs:301-309 패턴) →
    감사로그에 `timeout_seconds` 기록 → 사용자 메시지에 재시도/옵션 안내.
- **테스트**: `sleep` 더미 실행파일을 툴로 주입해 타임아웃 동작 확인(가용 툴 없는 환경에서도
  돌도록 sentinel 바이너리 방식 재사용, e01.rs:409-414 패턴).

### F2-3. [P1] 케이스 파일 쓰기 원자화 — 공수 S
- **문제**: `util.rs:37-42` `write_text` = bare `fs::write` → `case.json`, `db/video_index.json`,
  `db/videos.jsonl` 크래시 시 절단 파일.
- **조치**: `write_text_atomic` 추가 — 같은 디렉터리에 `<name>.tmp-<pid>` 쓰기 → `fs::rename`
  (Windows에서 std rename은 대체 허용) → 필요시 sync_data. 세 인덱스 쓰기에 적용.
- **테스트**: 기존 테스트 통과 + "rename 대상이 이미 존재" 케이스.

### F2-4. [P1] `unique_path` 폴백이 원본 경로 반환 — 공수 S
- **문제**: `util.rs:66-69` — 10,000회 시도 후 **이미 존재하는 원본 경로를 그대로 반환** →
  호출부가 쓰면 기존 파일 덮어씀(포렌식 산출물 파괴). TOCTOU도 있음.
- **조치**: 시그니처를 `Result<PathBuf, String>`으로 변경, 소진 시 Err.
  또는 소진 시 타임스탬프 이름(`<stem>_<unix_nanos>.<ext>`) 1회 시도 후 Err.
- **테스트**: 10,001개 충돌 시나리오는 비현실적이므로, 내부 반복 상수를 주입 가능하게 하여
  소진 경로 테스트.

### F2-5. [P2] ffprobe duration이 첫 `"duration"` 매치 — 공수 S
- **문제**: `ffprobe.rs:88` `find_json_string(&raw, "duration")` — 스트림 레벨 duration을
  먼저 잡을 수 있어 포맷 전체 길이와 불일치 가능(증거 메타 정확도).
- **조치**: `stream_section` 패턴을 따라 `format_section()` 추가 → `format.duration` 우선,
  없으면 video 스트림 duration 폴백. (F4-1 serde 도입 시 자동 해소되는 클래스.)
- **테스트**: duration이 스트림과 포맷에서 다른 synthetic ffprobe 출력 픽스처.

### F2-6. [P2] e01 부분 `.raw` 정리 — 공수 S
- **문제**: `e01.rs:149` — ewfexport 실패/중단 시 부분 raw가 남음(tsk.rs:302는 이미 정리함).
- **조치**: tsk.rs 패턴 복제(실패 시 `remove_file`).
- **검증**: 실패 주입 테스트(존재하지 않는 ewfexport sentinel로 출력 부재 확인).

### F2-7. [P2] make-review 빈 케이스 에러 메시지 — 공수 S
- **문제**: 스캔 전 make-review → `failed to read …/video_index.json: (os error 2)` 원시 에러.
- **조치**: 인덱스 부재 시 "no case index yet — run `scan-folder` first" 안내 문구로 변환.

---

## Phase 3 — 테스트·CI 강화

### F3-1. [P1] 외부 도구 실실행 통합 테스트 계층 — 공수 L
- **문제**: ffmpeg/ffprobe/libewf/TSK를 **실행하는 테스트가 세상 어디에도 없음**(단위는
  arg-builder 수준). 제품 본질 기능이 전부 미검증.
- **조치**:
  1. `tests/integration_tools.rs` 신설. 게이트: `#[ignore]` + 환경변수
     (`FRAMETRACE_IT=1 cargo test -- --ignored`).
  2. ffmpeg로 픽스처 생성(테스트 안에서 `testsrc` MP4 생성 → 케이스 파이프라인):
     - scan → validate → make-review: 5개 상태 배지 검증
     - make-thumbnail: JPEG 실제 생성 + 재사용(cached) 확인
     - export-video mp4: 산출 클립 duration ±0.5s 검증
     - 손상 MP4(64B) → validation-failed 분기
  3. libewf/TSK는 CI 설치가 어려우므로 로컬 전용 계층으로 문서화만.
- **검증**: FRAMETRACE_IT=1 로컬 녹색, 미설정 시 skip(0 tests로 표시).

### F3-2. [P1] CI에 ffmpeg 설치 + IT 계층 실행 — 공수 S
- `windows-ci.yml`에 `choco install ffmpeg -y` 추가 후 `FRAMETRACE_IT=1 cargo test --locked -- --ignored`.
- (별도 이슈로) TSK/libewf Windows 배포 조사 — 핸드오프 §5와 연계.

### F3-3. [P2] QA 실패 분기 테스트 — 공수 S
- `qa.rs:60-83`의 hash_mismatch, 정확도 미달 브랜치 단위 테스트.
- smoke(cli_smoke.rs:70-86): accuracy manifest에 실제 sha256 채우고, reproducibility가
  "자기 자신 비교"라는 사실을 주석+별도 비교 대상으로 개선.

### F3-4. [P2] smoke에 verify-audit 연결 — 공수 S
- `cli_smoke.rs` 라이프사이클 말미에 `verify-audit case/audit.jsonl` 실행 추가
  (15개 커맨드가 실제로 체인을 만들었는지 end-to-end 검증).

### F3-5. [P2] Windows junction 테스트 — 공수 S
- `package.rs:396` symlink 거부 테스트가 `#[cfg(unix)]`라 Windows에서 미실행.
  Windows 분기 추가: `std::os::windows::fs::symlink_dir`(개발자모드 필요 → 실패 시 skip) 또는
  junction(`fs::symlink_metadata`로 판별) 테스트.

### F3-6. [P2] CI 정합성 — 공수 S
- setup-node를 `cargo test` **이전**으로 이동(html_report의 node 검증이 pinned node 사용하게).
- `node --check` 대상 2개 하드코딩 → glob/스크립트로 자동 수집.

### F3-7. [P3] (선택) 커버리지 가시화 — 공수 M
- `cargo-llvm-cov` → 아티팩트 업로드. 목표치: 전체 line 50%+, engine 코어 70%+.

---

## Phase 4 — 리팩터링 (구조 부채)

### F4-1. [P1] serde_json 도입 — 공수 L (최대 체감 리팩터)
- **문제**: 손작성 JSON 직렬화→재파싱이 데이터 밴본. `extract_json_string` 5개 파일 복붙
  (audit.rs:137, scan.rs:684, validation.rs:172, selection.rs:211, qa.rs:453),
  ffprobe.rs:121 변형, scan.rs:348-377은 자기 JSON을 재파싱해 DB 로우 생성.
- **조치** (단계적):
  1. `Cargo.toml`: `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.
  2. `ffprobe.rs` — `#[derive(Deserialize)] struct FfprobeOutput { format: Format, streams: Vec<Stream> }`
     (F2-5 동시 해소).
  3. `model.rs` VideoRecord에 Serialize/Deserialize 파생. 단 **필드 순서/키 호환 유지**
     (기존 JSONL/TSV 호환 아티팩트 계약 — recovery-test-spec 참조).
  4. case_db 로우 빌드가 구조체에서 직접 → scan.rs:348-377 삭제.
  5. `extract_json_*` 계열 전부 삭제, `util::json_for_script`는 유지(스크립트 이스케이프 계약).
- **회귀 방지**: 기존 단위 테스트 전부 유지 + "구버전 JSONL → 신규 코드 로드" 호환 테스트 추가.

### F4-2. [P2] 에러 타입 도입 — 공수 L (단계적)
- 1단계(즉시 가능): `handlers.rs:624-627` `resolve_batch_selector`의
  `or_else(|_| …)` 오류 삼킴 — 첫 resolve의 에러를 로그/포함한 메시지로. `handlers.rs:908`
  `unwrap_or(0)` 제거(시계 오류 명시 에러).
- 2단계: `thiserror`로 `EngineError` 정의(Tool{...}, Io{...}, Index{...}, Validation{...}),
  모듈 단위로 순차 마이그레이션(ffprobe → e01 → tsk → video_export …).

### F4-3. [P2] handlers.rs 분할 — 공수 M
- `register-source→start_job→run→complete/fail` 라이프사이클 6회 복붙(93-125, 190-230,
  388-420, 439-473, 509-536, 565-579) → `fn run_job<T>(...)` 헬퍼로.
- `generate_review_thumbnails`(1047-1144) → `artifacts.rs` 이동.
- `base64_encode`(1149-1171) → `util.rs`. `*_options_json` 6개(1232-1303) → `selection.rs`.
- 목표: handlers.rs 1,317줄 → ~600줄. 행위 변경 없음(기존 테스트가 회귀망).

### F4-4. [P3] sanitize_filename 한글 — 공수 S
- `video_export.rs:270-284` 비ASCII 전부 `_` → 직접경로 셀렉터 한글명이 `____.mp4`로.
- 조치: 비ASCII만 치환하지 말고 `vid_XXXXXX` ID 기반 이름 유지(이미 그렇게 하는 경로 확인) +
  원본명은 사이드카 메타로. 또는 Windows 허용 범위에서 한글 보존(NTFS는 유니코드 지원) —
  **도메인 판단 필요**: 포렌식 산출물은 ASCII 안전이 안전하므로 현행 유지+문서화 권장.

---

## Phase 5 — 접근성·UX 폴리시 (프런트)

| ID | 항목 | 조치 | 공수 |
|---|---|---|---|
| F5-1 | `needs_verification` 칩 대비 미달 | `--warn` 배경 위 흰 텍스트(≈3.4:1) → 텍스트 `#1f2724` 또는 배경 `#8a5d14`. DESIGN.md 토큰 갱신 병행 | S |
| F5-2 | 히스토그램 8px 라벨 | `.lbl` 8px→10px, `aria-label="{day} {count}건"` 추가(title만은 불충분) | S |
| F5-3 | 그리드 카드 키보드 불가 | `.card`에 `tabindex="0"` + `role="button"` + keydown(Enter/Space) 활성화, `:focus-visible` 아웃라인 추가 | M |
| F5-4 | 모달 접근성 | `role="dialog" aria-modal="true"`, 열림 시 닫기 버튼 포커스, Tab 순환 트랩 | S |
| F5-5 | 태그 입력 포커스 유실 | 커스텀 태그 추가 → render() 후 `customTagInput.focus()` 복원(연속 태깅 워크플로우) | S |
| F5-6 | 태그 칩 단색 `#155eef` | 팔레트 외 색. `--accent-2 #1c5d8f`로 교체(태그별 색상 분기는 보류) | S |
| F5-7 | i18n 분리(대시보드 EN/뷰어 KO) | 1단계: README에 현황 명시. 2단계: 프로토타입의 `data-i18n` 사전 방식을 생성 뷰어에 이식(토글) | M |
| F5-8 | 반응형 하한 | 데스크톱 전제이므로 `min-width: 1180px` + 그 이하 안내 배너 정도만 | S |

---

## Phase 6 — 문서 동기화

| ID | 항목 | 근거 | 공수 |
|---|---|---|---|
| F6-1 | 핸드오프 유령 커밋 제거 | `WINDOWS_IMPLEMENTATION_HANDOFF.md:23`의 `f5c95f5`가 역사에 없음 → "main HEAD 기준"으로 교체. Phase 6/§9 완료기준에 export-batch/validate-batch/import-marks/export-marks/verify-audit + 태그 시스템 + 새 뷰어 반영 | S |
| F6-2 | schema.md 갱신 | v2/6테이블 → v3/`review_marks` 포함(`case_db/core.rs:6,190-199`) | S |
| F6-3 | "Sleuth Kit later" 모순 제거 | README:31, TECH_STACK.md:20,103 갱신 + README에 `verify-audit` 문서화(`cli/mod.rs:190`) | S |
| F6-4 | 뷰어 2종 관계 명시 | README:122 부근에 "gui/evidence-viewer=프로토타입(4패인), review/evidence-viewer.html=생성물(좌우분할+그리드+태그)" 설명. EVIDENCE_VIEWER_GUI.md:9-12, DESIGN.md §4에 동일 주석 | S |
| F6-5 | 문서 인덱스 누락 5건 추가 | VIEWER_UX_PLAN/schema/security-review/static-analysis/cleanup-review → README Planning Docs | S |
| F6-6 | 전제조건 보강 | Node.js 필요(recovery-test-spec:24/CI), FFmpeg/libewf/TSK Windows 설치 링크, rustup 자동 핀(rust-toolchain.toml) 안내 | M |

---

## Phase 7 — (옵션) 스케일링

| ID | 항목 | 조치 | 공수 |
|---|---|---|---|
| F7-1 | 썸네일 data-URL → 상대경로 | `review/thumbs/<id>.jpg`가 이미 디스크에 존재(handlers.rs:1052-1067). DATA.thumbs를 `thumbs/<id>.jpg`로 바꾸면 단일 HTML이 1,000건 기준 수십MB→수백KB. 단, "단일 파일 자기완결" 제품 약정과 상충 → package-case가 thumbs 포함하므로 패키지 관점에서는 안전. **결정 필요**: 자기완결 유지(현행) vs 경량화 | M |
| F7-2 | 1,000카드 렌더 성능 | 페이지 사이즈 1000일 때 innerHTML 전량 재구성. 측정 후 필요 시 카드 DOM 재사용. 지금은 보류 | L |

---

## 실행 순서와 마일스톤

1. **즉시(반나절)**: F0-1 커밋 → F0-2, F1-2, F1-7, F1-1, F1-3, F1-4, F2-3, F2-4, F2-6, F2-7
   (전부 S~M 공수, 배포 산출물/데이터 무결성 직결)
2. **1주차**: F2-1, F2-2, F3-1, F3-2 (포렌식 신뢰성 + 실측 테스트 기반 구축)
3. **2주차**: F4-1(serde) → F4-2 1단계 → F4-3, F1-5, F1-6, F3-3~F3-6
4. **3주차**: F5 전체, F6 전체, (결정 사항) F4-4, F7-1

## 각 단계 공통 검증 게이트

```
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
FRAMETRACE_IT=1 cargo test -- --ignored        # F3-1 이후
node scripts/check-css.mjs                      # F0-2 이후
# 수동: make-review → review/evidence-viewer.html 시각 확인 (다크모드 에뮬레이션 포함)
```

## 열린 결정사항 (사용자 판단 필요)

1. **F7-1** 썸네일: 단일 HTML 자기완결 유지 vs 상대경로 경량화
2. **F4-4** 산출물 파일명: ASCII 안전 유지(현행) vs 한글 보존
3. **F5-7** i18n: KO/EN 토글 투자 여부
4. **F2-2** 타임아웃 기본값: 프로브 120s 적절성
