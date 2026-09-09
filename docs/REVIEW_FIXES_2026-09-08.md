# 코드 리뷰 수정 로그 (2026-09-08)

> 대상: 병렬 심층 리뷰(serde 마이그레이션 계약 / 보안·강건성 2개 트랙)에서
> 나온 발견 중 이번에 수정한 항목. FFmpeg `libavformat/dhav.c` 원본 소스로
> 크로스체크했다. 게이트: `cargo test` 121 + IT 4 + smoke 3 전부 녹색,
> 워크스테이션 실기기 스모크(정상 200 / 공격 403)로 동작 확인.

## 수정됨

### CRITICAL

1. **DAV 프레임 간격 anomaly의 타임스탬프 단위 오류** — `dav.rs` / `anomaly.rs`
   - FFmpeg `get_pts()`는 `date`초×1000(ms 도메인)에 `timestamp` diff를
     더하고 65535에서 랩한다. 즉 `timestamp`는 자유주행 **밀리초 카운터**지
     초가 아니다. 기존 구현은 (a) 초로 잘못 더하고 (b) `date`의 초 필드와
     **이중 계산**했다 — 실장비 파일에서 유령 gap(최대 ~65s) 또는 실제 gap
     누락으로 이어지는 잘못된 포렌식 결과.
   - 수정: `timestamp_secs` → `timestamp_ms`(의미론 주석 포함), 갭 계산은
     packed date 초 차이 + sub-second(ms) 보정만 사용. 랩 경계(65000→500)를
     지나는 fixture로 red-test 갱신.
2. **워크스테이션 HTTP API의 CSRF/DNS-rebinding 무방비** — `serve.rs`
   - `POST /api/open-folder`이 `explorer.exe <path>`를 실행(Windows에서는
     실행파일 **시작** 벡터), 어느 웹사이트나 `text/plain` 폼 POST로
     preflight 없이 호출 가능했다. `Host` 미검증이라 리바인딩으로 증거
     스트리밍 반출도 가능했다.
   - 수정: `request_is_localhost_trusted` — Origin(부재 또는 loopback)과
     Host(loopback)를 모든 라우트(`/media` 포함)에서 검증, 위반 시 403.
     연결 응답 후 write half-close. 단정 2건(rebinding 403, csrf 403) 추가.

### HIGH

3. **`set_json_field` 최초-등장 키 매칭이 중첩 객체 오염** — `scan.rs`
   - 증거 파일의 ffprobe 메타 태그가 `index_status`라는 이름을 가지면
     (공격자가 `ffmpeg -metadata`로 심을 수 있음) stale 마커 교체가
     **기록된 ffprobe 증거를 다시 쓰고** 실제 stale 추적을 잃었다.
   - 수정: 깊이-추적 스캐너 `find_top_level_key` — depth-1 멤버만 매치.
     중첩 키 무시 red-test 2건 추가.
4. **바이너리 심기(planting) 폴백** — `tool_policy.rs`
   - PATH/tools-bin에서 못 찾으면 bare name을 반환했고, `Command::new`의
     OS 탐색이 CWD(Windows)와 빈/상대 PATH 항목(Unix)까지 뒤져서, 이
     리졸버가 막으려던 바로 그 벡터를 되살렸다.
   - 수정: `resolve_bare_tool_path` — 못 찾으면 `Err`(실행 거부).
     red-test 추가.
5. **DAV ES 추출의 채널 혼합** — `dav.rs`
   - 멀티채널 DAV에서 모든 채널의 비디오 페이로드를 한 ES로 이어붙여
     재생 불가능한 "성공" 산출물을 만들었다.
   - 수정: 첫 비디오 프레임의 채널 고정. 멀티채널 fixture red-test 추가.
6. **파티션 자동선택 오류** — `tsk.rs`
   - GPT에서 첫 allocated 파티션 = EFI System Partition, DOS 확장 레이아웃에서
     = Extended Table 컨테이너를 골라 (거의) 빈 목록을 성공으로 제시했다.
   - 수정: 데이터 FS 서술어(NTFS/exFAT/FAT/HFS+/APFS/ext) 우선, EFI/컨테이너
     제외. 두 레이아웃 fixture red-test 추가.
7. **E01 마법사의 스테일 raw 재사용** — `serve.rs`
   - 케이스 재사용 시 이전 E01의 `evidence.raw`를 무조건 재사용해 다른
     E01을 골라도 옛 증거를 끝까지 분석했다.
   - 수정: `*.raw.source` 바인딩 파일로 E01 경로가 같을 때만 재사용.

### MEDIUM (일부)

 8. **`verify-audit`가 락 없이 읽음** — `audit.rs`: 검증도 appender와 같은
   fs2 배타락을 잡아 동시 append 중 반쓰기 줄을 보고 오탐(torn write)을
   내는 경쟁을 제거.

## 2차 하드닝 (이월 MEDIUM/LOW 소진, 2026-09-08)

9. **`mmls/fls` 타임아웃** — `tsk.rs`: `run_capture`가 `run_with_timeout`
   경유. `TskInspectOptions.timeout_secs`(기본 120s, `--timeout` 플래그) —
   손상 이미지가 검수 단계를 영구히 정지시키는 DoS 제거.
10. **`unique_path` TOCTOU** — `util.rs`: `O_EXCL` 플레이스홀더로 경로를
    **원자적으로 예약**. 동시 8-레이서 red-test로 전원 상이 경로 보장.
    디렉토리 대상(package-case)은 `unique_dir`(create_dir 클레임),
    "출력 부재" 하드 계약 경로(export-dav/hik)는 `unique_available_path`.
11. **디렉토리 fsync** — `util.rs`/`audit.rs`: `write_text_atomic`의 rename
    후, 감사로그 최초 생성 후 부모 디렉토리 fsync — "원자적" 쓰기의
    크래시 퍼시스턴스가 실제로 성립.
12. **`/media` 교정 3종** — `serve.rs`: 0바이트 파일 `Content-Length: 1`
    프로토콜 위반 수정, 경로 컴포넌트의 `+`→공백 치환 제거(`+` 포함
    파일명 접근 가능), `=` 없는 쿼리 쌍이 이후 파라미터 스캔을 중단하던
    것 수정. `X-Content-Type-Options: nosniff` 전 응답 추가.
13. **`body_value` `\uXXXX`** — `serve.rs`: hex 스칼라 디코딩(한글 마크
    라운드트립), hex 아닌 자리는 소비하지 않아 종결 따옴표 보존.
14. **stale 중복 스펠링 수렴** — `scan.rs`: 사라진 파일이 `\\?\`/클린 두
    스펠링으로 색인돼 있으면 한 건의 stale로 수렴(영구 2중 계상 제거).
15. **ffprobe 객체 키 순서 보존** — `serde_json` `preserve_order` 활성화:
    재파싱된 `raw_json`의 키가 알파벳 정렬되어 JSONL 바이트와 sqlite
    컬럼이 같은 증거를 다르게 표기하던 QA 재현성 문제 해소.
16. **E01 glob 메타문자 거부** — `e01.rs`: libewf가 마지막 인자를 세그먼트
    glob으로 해석하므로 `x[*].E01` 같은 증거명이 잘못된 세그먼트 조합을
    무음 선택할 수 있음 — 메타문자 포함 시 명시적 에러.

### 최종 검증 (2026-09-08, release 바이너리)

- 게이트: fmt PASS / clippy `-D warnings` PASS / 단위+스모크 128 /
  IT 4(실 ffmpeg+libewf+DAV+Hik) / CSS·JS 게이트 PASS.
- E2E: init→scan(+한글·`+` 파일명)→validate→review→report→anomalies→
  package(10 files)→감사로그 2/2 PASS.
- 워크스테이션 실기기: 정상 200, 리바인딩 Host 403, CSRF Origin 403,
  `nosniff` 헤더 확인.

## 3차 리뷰 (2026-09-08, 신규 미커버 모듈 + 회귀 검증)

병렬 심층 리뷰가 재현으로 확인한 발견(구현자 아닌 검증자 관점)과 수정:

17. **[C1] 기본 경로 export/proxy/thumbnail 0바이트 산출물 회귀** — 2차
    하드닝의 `unique_path` O_EXCL 예약 플레이스홀더가 ffmpeg `-n`(never
    overwrite)과 충돌: ffmpeg가 플레이스홀더를 거부하면서도 exit 0으로
    빠져 **0바이트 파일이 성공+정상 체인 해시(e3b0c4...)로 기록**됐다.
    export-batch는 전 항목 "already exists" 실패. 수정: `-y`(예약이 곧
    배타권)+ 실행 후 크기>0 검증(0바이트는 산출물 삭제 후 에러).
    재현: clip 50,986B / proxy 28,965B / thumb 10,627B / batch 2/2 ok.
18. **[C2] 소스별 순차 스캔이 타 소스의 살아있는 증거를 stale 마킹** —
    `merge_existing_with_scan`이 현재 스캔에 없으면 무조건 stale.
    README의 다중 소스 워크플로(srcA 스캔→srcB 스캔)가 서로를 stale로
    만들어 뷰어·리포트가 증거 상태를 그르쳤다. 수정: **디스크 존재
    검사 후** 실제로 없는 파일만 stale. 재현: srcA→srcB 후 둘 다
    active, srcA 파일 삭제 후 재스캔 시 srcA만 stale.
19. **[H1] export-dav/hik 조기 실패가 job을 영구 `running`으로 누적**
    — 나머지 6개 job 래핑 커맨드만 fail_job을 호출했다. 수정: inner
    함수 분리 + 전 조기 반환에 fail_job. 재현: 존재 않는 DAV →
    jobs 테이블 `failed` 행 확인.
20. **[H2] export --start 음수/EOF 초과 무검증** — 음수 start는 전체
    영상을 전달하며 로그에 보그값 기록, EOF 초과는 "성공한" 0프레임
    클립. 수정: 유한·비음수 검증(씨네마 썸네일 레인과 동일). 재현:
    `--start=-1.5` → 명시적 거부.
21. **[H3] 전 항목 실패 배치도 `complete` 기록** — export/validate/
    recover-batch가 ok/fail 집계와 무관하게 complete_job. 수정:
    ok==0 && 전원 실패 시 fail_job + 에러.
22. **[M1] torn 감사 로그 줄이 리포트/뷰어 스크립트를 통째로 무효화**
    — `jsonl_to_array`가 줄별 JSON 검증 없이 join. 수정(리포트+뷰어):
    유효하지 않은 줄 스킵 (체인 무결성의 판정 표면은 verify-audit).
23. **[M5] release QA에 감사 체인 검증 부재** — 제품의 핵심 주장인
    tamper-evident chain이 readiness 게이트에 없었다. 수정:
    `audit_chain` 체크 추가 — 케이스 전체(evidence+artifacts)에서 체인
    스키마(`previous_entry_sha256`) 로그를 수집·검증. 재현: 5/5 PASS.

검증: 게이트 전량 녹색(128+4), release E2E로 C1·C2·H1·H2·H3·M5 재현
시나리오 통과 확인. 리뷰가 확인한 정상 영역: SQL 전 파라미터화,
write_scan_index 단일 트랜잭션, 마이그레이션 멱등, busy_timeout,
병렬-감사-직렬화(M1-3) 주장, ffprobe serde 형태, escapeHtml 규율.

## 남은 후속 (외부 전제 조건)

- audit 체인 무키(비대칭) — 파일 쓰기 권한자의 전체 재작성은 감지 불가;
  법정 공개 시 문서화 권고.
- DAV `walk_frames` 손상 프레임 resync 정책 — 실장비 코퍼스 확보 전
  착수 금지(로드맵 원칙).
- SQLite↔JSONL 고스트 행 분기(M2): 재현 경로는 크래시 창/수동 수리로
  좁고, 교차 저장 비교 명령은 후보로 기록.
