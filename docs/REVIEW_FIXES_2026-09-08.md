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

## 미수정 (기록된 후속 후보)

- audit 체인이 비대칭(무키)이라 파일 쓰기 권한 보유자가 전체 로그을
  일관되게 재작성 가능 — 법정 공개 시 문서화 필요(`docs/` 권고).
- `walk_frames` 손상 프레임에서 전체 파일 하드 실패(FFmpeg은 resync) —
  실샘플 코퍼스 확보 후 resync 정책 결정 권장.
- `mmls/fls` 무타임아웃, `unique_path` TOCTOU, 비UTF-8 경로 lossy 처리,
  ffprobe 객체 재직렬화 시 키 알파벳 정렬(QA 재현성 바이트 영향) 등
  LOW/MEDIUM 잔여 — 로드맵 M5 후보로 이월.
