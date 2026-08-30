# FrameTrace 로드맵 (2026-08-30 개정)

> 이전 계획(FIX_PLAN.md)의 후속 문서. 2026-08-30 전체 리뷰(종합 7.3/10, 상용 비교 재채점)와
> E01 실무 스케일 요구사항(케이스당 1천~1만 건)을 반영해 전면 재작성했다.
> 상용 비교 기준점: Magnet DVR Examiner(독포맷 DVR 복원), Amped FIVE(영상 분석·증강),
> Oxygen Forensic Detective(종합), Autopsy(무료 대항마).

## 0. 포지셔닝 (변경 없음 — 모든 우선순위의 기준)

**"이미 확보된 영상 증거(표준 파일·E01·복구물)를 판독 → 검증 → 방어 가능한 보고서로 마무리하는 로컬 워크스테이션."**

- 경쟁하지 않는 축: 독포맷 DVR 100종 파싱(DME), 조작 위변조 탐지(Authenticate). 이 영역은 연동/보완으로 대응.
- 이기는 축: 체인 해시 감사로그 기반의 **처리 이력 증명**, 무료·로컬 전용·한국어 판독 UI.
- 스코어 목표: 현재 7.3 → **M3 완료 시 8.0 이상** (영역별: 테스트 6.0→7.5, 복원 5.0→6.5, 제품성 6.0→7.5).

## 1. 현재 상태 스냅샷 (2026-08-30)

완료: 검토 뷰어 UX 전반(P0 CSS 붕괴 해소, 4~5열 밀도, 좌우/상하 스플리터, 대형 플레이어, 시어터),
썸네일 외부화(1만 건 HTML 195MB→13.5MB 실측), 단일 exe 검수 런처(폴더/E01 INPUT, 미디어 스트리밍),
결과 보고서 확장(발견·분석 기법, 처리 체인, 판독 마크), 1만 건 뷰어 실측 통과(로드 1.3초).

미해결(이전 계획에서 이월): CSS 게이트(F0-2), 감사로그 원자화(F2-1), 프로세스 타임아웃(F2-2),
원자적 쓰기(F2-3), unique_path 덮어쓰기 버그(F2-4), ffprobe duration(F2-5), E01 부분파일 정리(F2-6),
file:// 모드 JS file_url 인코딩(F1-6), 죽은 코드 정리(F1-5), 문서 동기화(F6-*).

## 2. 마일스톤 개요

| 마일스톤 | 주제 | 목표 점수 | 공수(1인 기준) | 게이트 |
|---|---|---|---|---|
| **M1 v0.2 "신뢰성"** | 테스트화·파이프라인 속도·무결성 마무리 | 7.3 → **7.6** | 6~7일 | CI 녹색 + 신규 테스트 + 성능 예산 충족 |
| **M2 v0.3 "복원력"** | DAV 파서 1종 + 실 E01 검증 + 복원 고도화 | 7.6 → **7.8** | 8~10일 | 검증 코퍼스 통과 |
| **M3 v0.4 "제품화"** | 배포·설치·보고서 마감 | 7.8 → **8.0+** | 5~6일 | 제3자 클린 설치 → 첫 케이스 완주 |
| **M4 v0.5+ "심화"** | 이상 징후 플래그·FIVE 연동·대량 조작감 | 8.0 → 8.3+ | 별도 확정 | — |

원칙: 각 마일스톤 끝에 릴리스 태그 + 회귀 게이트 전량 통과. 게이트는
`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` /
`FRAMETRACE_IT=1 cargo test -- --ignored` / `node --check` + CSS 게이트 / 성능 예산 스크립트.

---

## 3. M1 v0.2 "신뢰성" — 세부 계획 (6~7일)

> **상태: 완료 (2026-08-30).** M1-1~M1-6 전항목 구현·검증 — 단위 테스트 77→90,
> ffmpeg 실실행 통합 테스트 CI 편입, 썸네일/검증 병렬화(워커풀+감사 직렬화),
> 감사로그 배타락+fsync 원자화+불완전 꼬리 정책, probe 120초 타임아웃과
> import-e01/export-video/recover-inode --timeout, 원자적 쓰기, unique_path 폴백 수정,
> CSS 게이트(scripts/check-css.mjs), ffprobe format.duration 우선, 문서 동기화.

가치: "1만 건을 실제로 다루는 도구"의 신뢰성을 코드로 증명한다. 종합 점수 기여분 +0.3.

### M1-1 신규 코드 테스트화 (1.5일) — 테스트 6.0 → 7.0
- `src/serve.rs`: 순수 함수 단위 테스트 — `percent_decode`(한글/`%`/`+`), `parse_range`,
  `path_is_under`(대소문자/접두사 경계/UNC), `body_value`(문자열·bool·이스케이프),
  `build_selection_file`(vid_ 필터, 0건 오류, 이스케이프된 id).
- 통합 테스트 1개: 에페머럴 포트로 서버 기동 → `GET /`, `GET /api/status`, `GET /api/env`,
  미로드 상태 `/review/x` 404, 컨테인먼트 403 검증. (파이프라인 기동은 M1-2의 ffmpeg 게이트로.)
- 보고서: `render_case_report`에 기법/체인 섹션 마커 존재 테스트(빈 입력·풀 입력 각 1건).
- 수용기준: 위 전부 `cargo test` 통과. 신규 코드 라인 커버리지 0 → 핵심 경로 커버.

### M1-2 ffmpeg 실실행 통합 테스트 + CI (1일) — 테스트 7.0 → 7.5, CI 7.5 → 8.0
- `tests/integration_tools.rs` 신설. 게이트: `#[ignore]` + `FRAMETRACE_IT=1`.
  케이스: ffmpeg로 testsrc MP4 3종(+손상 파일 1개) 생성 → scan --hash → validate-batch →
  make-review(썸네일 생성·캐시 재사용 확인) → make-report(기법/체인 섹션 렌더) → package-case.
  단정: 상태 배지 조합(확인 3/실패 1), 썸네일 파일 존재, 패키지 manifest 해시 일치.
- `.github/workflows/windows-ci.yml`: `choco install ffmpeg` 추가, `FRAMETRACE_IT=1 cargo test -- --ignored` 스텝 추가,
  setup-node를 cargo test **이전**으로 이동, `scripts/check-css.mjs`(아래 M1-5) 추가.
- 수용기준: CI에서 "외부 도구를 실행하는 테스트"가 최초로 실행·통과. 일부러 ffmpeg 제거한 잡에서 적색 확인(레드 테스트).

### M1-3 파이프라인 병렬화 (1.5일) — 대용량 7.5 → 8.0
- `generate_review_thumbnails`와 `validate_batch`에 워커 풀(N = min(8, 논리코어)) 적용.
  제약: **감사로그 append는 단일 스레드로 직렬화**(체인이 직전 줄 해시에 의존) — 결과 계산은 병렬,
  `append_chained_jsonl` 호출은 큐/뮤텍스로 순차화.
- 수용기준(성능 예산): 1,000건 썸네일 생성 ≤ 3분(현재 추정 7~15분), 1,000건 ffprobe 검증 ≤ 4분.
  `qa reproducibility`로 병렬 전후 결과 동일성 확인, 병렬 실행 후 `verify-audit` 통과.

### M1-4 무결성 마무리 3종 (1.5일) — 무결성 8.5 → 9.0
- F2-1 감사로그 원자화: O_APPEND 단일 write + `<log>.lock`(fs2) 배타락 + fsync,
  `verify-audit`에 불완전 마지막 줄 정책. 테스트: 절단 상태 append → verify, 자식 프로세스 2개 동시 append.
- F2-2 외부 프로세스 타임아웃: `wait-timeout` crate. 프로브류(ewfinfo/ewfverify/ffprobe) 기본 120s,
  변환류는 `--timeout` 옵션. 타임아웃 시 부분산출물 삭제 + 감사로그 기록.
  테스트: sleep 더미 실행파일 센티넬 방식(가용 툴 없는 CI에서도 동작).
- F2-3/F2-4/F2-6: `write_text_atomic`(temp+rename) 도입, `unique_path` 소진 시 Err 반환,
  e01 부분 raw 정리. 각 1건 이상 단위 테스트.

### M1-5 프런트 회귀 방지 + 잔여 소소한 버그 (0.5일)
- F0-2 CSS 게이트: `VIEWER_CSS.trim_start().starts_with(':')` rust 테스트 + CI `scripts/check-css.mjs`(중괄호 균형·`:root` 존재).
- F2-5 ffprobe duration: `format.duration` 우선 파싱(픽스처 테스트).
- F2-7 빈 케이스 make-review 안내 문구, F1-5 죽은 코드 제거(중복 CSS 블록, `body.stack`, 미사용 변수),
  F1-6 file:// 모드 JS `fileUrl()`의 `encodeURI` → 세그먼트별 인코딩(워크스테이션 모드는 이미 /media 프록시로 해결됨).

### M1-6 문서 동기화 (0.5일) — 문서 7.5 → 8.0
- F6-1~6: 핸드오프 유령 커밋 제거, schema.md v3 갱신, "Sleuth Kit later" 모순 제거 + verify-audit 문서화,
  뷰어 2종 관계 명시, README 문서 인덱스 보강, 전제조건(ffmpeg/libewf/TSK/Node 설치 링크) 정리.
- FIX_PLAN.md 삭제 또는 "완료 아카이브"로 격하(본 문서가 후속).

---

## 4. M2 v0.3 "복원력" — 세부 계획 (8~10일)

가치: "E01 1천~1만 건" 실무 시나리오의 실 복원력 상승. 복원 5.0 → 6.5. 종합 +0.2.

### M2-1 Dahua DAV 파서 1종 (4~5일) — 복원 5.0 → 6.5

> **상태: 구현 완료 (2026-08-30), 실장비 검증 대기.** src/dav.rs(DHAV 워커·ES 추출·
> h264/hevc 리먹스) + `export-dav` 명령(export-log 체인 기록). 실장비 DAV 샘플이
> 아닌 **문서화된 컨테이너 스켈레톤의 합성 픽스처 + 실 H.264 ES**로 E2E 검증
> (합성 DAV → export-dav → ffprobe 검증됨 확인). 단위 4건 + IT 1건.
- 근거: `docs/MANUFACTURER_PARSER_RESEARCH.md`의 Dahua 레인. 스코프를 "완전 파싱"이 아닌
  **인덱싱+복원 파이프라인 1본**으로 제한:
  1. `detector.rs`: .dav 탐지(확장자 + 파일헤더 시그니처) 레인 추가.
  2. `carve.rs`: DAV 프레임 시그니처 카빗(파일헤더/프레임 헤더 매직) — 오프셋·채널·시간 추출.
  3. 복원: 프레임 스트림에서 h264/h265 ES 추출 → ffmpeg로 mp4 리먹스(기존 `video_export`에 DAV 경로 추가).
  4. 뷰어: dav 레코드에 원본 채널/시각 메타 표기(이미 channel/recType 파서 존재).
- 선행조건(리스크): 실제 DAV 샘플 3종 이상 확보(전/후방, 이벤트, 파킹). 없으면 M2 착수 불가 —
  확보 경로(테스트 장비 녹화 / 공개 코퍼스)를 먼저 확인할 것.
- 수용기준: 코퍼스 3종 전부 — 색인됨, 썸네일 생성, mp4 내보내기 후 ffprobe 재검증 통과,
  `qa accuracy`에 DAV 매니페스트 추가(ground truth 대조).

### M2-2 실 E01 엔드투엔드 검증 (1.5일)

> **상태: 차단 (2026-08-30).** winget/choco/GitHub 릴리스 어디에도 libewf Windows
> 바이너리 없음(소스 배포만). 대응: libewf 설치 시 자동 활성화되는 IT 테스트를
> 미리 준비(e01_import_roundtrip_with_real_libewf — ewfacquire → import-e01 →
> verified/해시 단정). libewf 확보 후 즉시 실행.
- libewf Windows 바이너리 확보·설치 문서화(핸드오프 §5 이슈 소멸).
  ewfacquire로 테스트 이미지를 E01로 생성 → 런처 E01 체인(INPUT→검증→추출→조사→리뷰) 실측.
  inspect-image fls 1만 엔트리 성능 측정, recover-inode 샘플 복원 → validate → 리뷰 반영 확인.
- 수용기준: E01 실측 로그가 `docs/WINDOWS_VALIDATION.md`에 기록됨. 런처 E01 모드가 libewf 설치 상태에서
  오류 없이 리뷰 생성까지 완주.

### M2-3 리커버리 UX 보강 (1.5일)

> **상태: 완료 (2026-08-30).** 뷰어에 삭제 영상 후보(kind candidate) 레코드·출처 필터·
> action recover 선택 내보내기 추가, CLI recover-batch 추가(감사 체인 기록).
> 실 이미지 기반 검증은 libewf/TSK 확보 후 M2-2와 함께 수행.
- inspect-image 결과(fls 목록)에서 뷰어로 "복구 후보"를 표기 — 현재는 recover-inode 실행분만 표시되므로,
  flsEntries 기반으로 "삭제 영상 후보 N건(복구 전)" 섹션 추가(레코드화는 하지 않고 요약+경로 표시).
- `recover-inode` 다중 inode 배치 명령(`recover-batch <case> <image> --selection`) 추가 —
  검수관이 뷰어에서 선택한 목록을 CLI로 넘겨 일괄 복구. selection 스키마 재사용.
- 수용기준: 복구 전/후 상태가 뷰어에서 구분 표시되고, 배치 복구가 감사로그에 체인 기록됨.

### M2-4 엔지니어링 부채 1차 (2일, M1~M2 사이 틈틈이)

> **상태: 1단계 완료 (2026-08-30).** ffprobe JSON 파싱을 serde 구조체로 전환하고
> 손작성 파서(format_section/stream_section/find_json_*) 제거. model 직렬화 전환과
> extract_json_* 5중복 제거는 M4 후보로 이월.
- F4-1 serde_json 도입 1단계: ffprobe 출력 파싱 + model 직렬화(호출부 5곳 중복 파서 제거).
  JSONL/TSV 호환 계약 유지 회귀 테스트 선행.
- F4-2 1단계: `resolve_batch_selector` 오류 삽식 수정 등 즉시 가능한 에러 처리 개선.

---

## 5. M3 v0.4 "제품화" — 세부 계획 (5~6일)

> **상태: 완료 (2026-08-30).** 이중 바이너리(frametrace-app.exe 콘솔 숨김 + 실패 시
> 메시지 박스), scripts/make-portable.ps1 휴대용 zip, 보고서 인쇄/PDF 버튼+인쇄 CSS,
> 런처 마크 파일 반영(import-marks→보고서 갱신), 접근성 배치(카드 키보드 포커스,
> 히스토그램 10px+aria, 검증 대기 칩 대비 AA, 태그 칩 팔레트색, 모달 포커스, 태그 입력
> 포커스 유지), CHANGELOG, 버전 0.4.0, **qa release 4/4 PASS**.
> 추가 발견·수정: ffprobe 오류 접두사의 메모리 주소가 인덱스 재현성을 깨는 실버그
> → sanitize_probe_error로 수정(재현성 QA PASS로 이어짐).

가치: 제3자가 클린 Windows에서 설치→첫 케이스 완주. 제품성 6.0 → 7.5. 종합 8.0 도달.

### M3-1 배포 패키지 (2일)
- 인스톨러(NSIS/Inno): exe + `tools/` 안내 + 문서. 코드사이닝은 비용 문제로 보류 가능(문서에 Hash 공개).
- 콘솔 창 정책 **결정 필요**(아래 7번): 권장안 — CLI는 `frametrace.exe` 유지, 런처는 `frametrace-app.exe`
 (`#![windows_subsystem = "windows"]`, 같은 크레이트 이중 바이너리)로 콘솔 숨김.
- 도구 설치 자동화 문서: winget/choco로 ffmpeg 설치 명령, libewf/TSK 바이너리 배치 경로(`tools/bin`) 자동 탐지
  (tool_policy 검색 경로에 확장).

### M3-2 보고서 마감 (1.5일) — 보고 7.0 → 7.5
- `@media print` 강화 + 보고서 상단 "인쇄/PDF 저장" 버튼(서버리스 유지, 새 의존성 없음).
  페이지 나눔 규칙, 표 머리 반복, 케이스 요약 머리글.
- 마크 반영 UX: 뷰어 "마크 내려받기" → 런처 STEP4에 "마크 가져와서 보고서 갱신" 버튼
  (import-marks → make-report 호출, 이미 런처에 finalize 인프라 있음).

### M3-3 접근성·마무리 (1일) — UX 8.0 유지 확인
- F5-1 대비 수정(needs_verification 칩), F5-2 히스토그램 라벨 10px+aria-label, F5-3 카드 키보드 포커스,
  F5-4 모달 포커스 트랩, F5-5 태그 입력 포커스 복원, F5-6 태그 칩 팔레트 색.

### M3-4 릴리스 절차 (1일)
- 버전 0.4 태그, CHANGELOG, `qa release` 전 항목 통과, 성능 예산 재측정(1만 건: 로드 ≤1.5s 유지),
  검증 결과를 docs/WINDOWS_VALIDATION.md에 기록.

---

## 6. M4 v0.5+ "심화" — 후보 풀 (확정 미정)

정렬 기준: 판독 워크스테이션 포지셔닝 기여도.
1. **이상 징후 플래그**(조작 탐지의 현실적 하위집합): 타임스탬프 역행/격차, 프레임 타임 간격 이상,
   컨테이너-스트림 불일치, 해시 재검증 불일치 → "candidate-finding" 라벨로 보고서에 표기(과장 금지 원칙 유지).
2. **Amped FIVE/DME 연동 문서**: FrameTrace 패키지의 폴더 구조·해시 매니페스트를 상용 도구 입력으로
   넘기는 절차 문서화(자체 개발 대비 현실적 선택).
3. **j/k 부분 렌더 + 1000개씩 모드 개선**(가상 스크롤 여부는 1만 건 실데이터 체감 후 결정).
4. **i18n 토글**(뷰어 한/영 — 프로토타입의 data-i18n 사전 이식).
5. **WinUI 네이티브 셸**: 브라우저 런처로 실무 사용이 확인된 이후에만 재평가(핸드오프 §6과 연계).
6. **Hikvision 파서**(DAV 다음 레인 — 코퍼스 확보 가능할 때만).

## 7. 열린 결정사항 (사용자 확정 필요)

| # | 결정 | 권장안 |
|---|---|---|
| 1 | 콘솔 창 정책: 이중 바이너리(frametrace / frametrace-app) vs 콘솔 유지 | 이중 바이너리 (M3-1) |
| 2 | PDF 보고서: window.print() 최적화 vs 외부 변환기 의존 | window.print() (서버리스 유지) |
| 3 | DAV 샘플 코퍼스 확보 경로 | 실 장비 녹화 우선, 불가 시 공개 코퍼스 조사 |
| 4 | M2-4 serde 도입 범위: ffprobe/model만 vs 전면 | 1단계(ffprobe+model)만 — 전면은 M4 이후 |
| 5 | M1 착수 시점 | 즉시 (6~7일 물량, 리스크 최저·효과 최대) |

## 8. 추적 지표 (마일스톤마다 재측정)

- 스코어: M1 후 7.6 / M2 후 7.8 / M3 후 8.0+ (영역표는 리뷰 문서 기준 유지)
- 성능 예산: 1만 건 뷰어 로드 ≤1.5s · 1천 건 썸네일 ≤3분 · 1천 건 검증 ≤4분 · inspect-image 1만 엔트리 ≤2분
- 품질: 테스트 수 ≥ 120(현재 77) · 외부 도구 실행 테스트 ≥ 6 · CI 적색 0 유지
- 무결성: 병렬 실행 후 `verify-audit` 100% 통과
