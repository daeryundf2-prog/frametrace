# Windows/GUI 리스크 검토

이 문서는 Windows 검증과 최종 GUI shell 구현 전에 반드시 고려해야 하는 운영 리스크를 정리한다.

핵심 결론: GUI는 단순히 버튼을 붙이는 단계가 아니다. 대용량 증거 처리, 외부 도구, 원본 증거 보호, 장시간 작업, 보고서 신뢰성을 다뤄야 하므로 아래 항목을 제품 요구사항으로 반영해야 한다.

## 최우선 구현 항목

Windows GUI 또는 engine 보강에 들어가기 전에 아래 6개를 우선 설계한다.

1. 진행률과 예상 남은 시간
2. 장시간 작업 중단/재개
3. 작업 전 디스크 공간 preflight
4. 외부 도구 dependency checker
5. audit chain 검증
6. 대량 evidence list virtualization

## 1. 진행률과 예상 남은 시간

문제:

- HDD, SD, E01, raw image는 수백 GB에서 TB 단위일 수 있다.
- 스캔, 해시, `ffprobe`, carving, E01 export는 장시간 작업이 된다.
- 사용자가 현재 작업 상태와 남은 시간을 모르면 앱이 멈춘 것으로 오해한다.

필요 기능:

- job별 진행률
- 경과 시간
- 예상 남은 시간
- 처리 속도
- 현재 처리 중인 파일 또는 offset
- 초반 ETA 안정화 전 `계산 중` 상태

예상 UI:

```text
스캔 중 · 18,420 / 92,000 파일 · 214GB 처리 · 예상 42분 남음
진행률 63% · 경과 18분 12초 · 420MB/s · 현재 작업 ffprobe metadata 수집
```

구현 기준:

- 파일 스캔: 처리 파일 수 / 전체 파일 수
- 해시: 처리 바이트 / 전체 바이트
- E01 export: 생성된 raw 바이트 / 예상 raw 바이트
- `ffprobe`: 처리 영상 수 / 전체 영상 수
- carving: 읽은 image 바이트 / 전체 image 바이트
- 최근 30-60초 평균 처리량으로 ETA 계산

## 2. 장시간 작업 중단/재개

문제:

- 대용량 작업 중 PC 절전, 앱 종료, 오류, 사용자의 취소가 발생할 수 있다.
- 처음부터 다시 처리하면 실무 효율이 크게 떨어진다.

필요 기능:

- job checkpoint
- 마지막 처리 파일 또는 image offset 저장
- 재시작 시 이어하기
- 실패, 취소, 중단, 완료 상태 구분
- 재실행 시 중복 row/log/artifact 방지

적용 대상:

- `scan-folder`
- `import-e01`
- `inspect-image`
- `carve-file`
- `validate-artifact` bulk mode
- proxy/thumbnail/export batch
- package generation

구현 기준:

- case DB의 job/event table을 중심 상태 저장소로 사용한다.
- command 재실행은 idempotent에 가깝게 동작해야 한다.
- GUI는 실패 job을 `재시도`, `이어하기`, `폐기`로 구분해야 한다.

## 3. 원본 증거 보호

문제:

- GUI에서 사용자가 원본 SD/HDD/E01 위치에 파생물을 만들면 forensic integrity가 깨진다.
- 특히 export/package 경로 선택 UI에서 실수가 나오기 쉽다.

필요 기능:

- source path와 case output path 분리 강제
- 원본 drive 또는 source folder 내부 output 금지
- case folder가 source 내부이면 경고 또는 차단
- write-protection 상태 입력 유도
- output은 기본적으로 case folder 내부에만 생성

GUI 규칙:

- 원본 경로는 항상 read-only source로 표시한다.
- `원본 수정 없음` 상태를 source panel에 계속 보여준다.
- vendor player `.exe`는 자동 실행하지 않는다.
- unknown executable은 suspicious item으로만 표시한다.

## 4. 디스크 공간 부족

문제:

- E01 raw export는 원본 크기만큼 공간을 요구할 수 있다.
- proxy, thumbnail, clip export, package는 case folder를 크게 만든다.
- 작업 중 공간 부족이 나면 partial artifact와 불완전 로그가 남을 수 있다.

필요 기능:

- 작업 전 필요 공간 추정
- case drive 남은 공간 표시
- export/package/carve/import 전 preflight check
- partial file 정리 또는 `partial-failed` 상태 기록
- 재시도 안내

우선 적용:

- `import-e01`
- `carve-file`
- `make-proxy`
- `export-video`
- `package-case`

## 5. 대량 파일 UI 성능

문제:

- 실무에서는 1,000개가 아니라 10,000-100,000개 파일도 가능하다.
- 일반 table/list 렌더링은 GUI freeze를 만든다.

필요 기능:

- virtualized list
- SQLite 기반 filter/search/sort
- lazy thumbnail loading
- 화면에 보이는 row만 렌더링
- filter/sort 중 UI thread block 방지

검증 기준:

- 1,000개 evidence는 즉시 사용 가능해야 한다.
- 10,000개 evidence도 filter/search가 실사용 가능해야 한다.
- 100,000개 evidence는 progressive loading 또는 제한 안내가 필요하다.

## 6. 외부 도구 의존성

문제:

- Windows에서 FFmpeg, libewf, Sleuth Kit 설치 경로가 다양하다.
- PATH에 없거나 버전이 다르면 기능 실패가 발생한다.

필요 기능:

- 시작 시 dependency check
- binary path 수동 지정
- 기능별 missing 상태 표시
- 실행한 binary와 version을 audit log에 기록
- missing tool이면 해당 기능만 비활성화

도구별 영향:

- `ffmpeg.exe`: export, proxy, thumbnail
- `ffprobe.exe`: scan metadata, validation
- `ewfinfo.exe`, `ewfverify.exe`, `ewfexport.exe`: E01 inspect/import
- `mmls.exe`, `fls.exe`, `icat.exe`: raw image filesystem inspection/recovery

## 7. Windows 경로 문제

문제:

- 한글, 공백, 긴 경로, drive letter, UNC path, `file:///C:/...` URL 변환이 문제를 만든다.

필요 테스트:

- `C:\Cases\한글 사건\증거 원본\`
- `C:\Cases\Client CCTV 001\`
- 긴 경로
- `\\server\share\...` 형태
- removable drive letter 변경

필요 기능:

- JSON/TSV/HTML path escaping 검증
- SQLite Unicode path 보존
- local HTML viewer의 file URL 생성 검증
- PowerShell quoting 문서화
- drive letter 변경 시 missing source 안내

## 8. 로컬 뷰어 재생 제한

문제:

- HTML viewer는 서버 없이 열리지만 브라우저가 코덱이나 local file access를 제한할 수 있다.
- 같은 파일도 Chrome, Edge, Windows Media Foundation에서 재생 결과가 다를 수 있다.

필요 기능:

- 재생 실패와 파일 부재를 구분
- proxy 생성 후 proxy 재생 옵션
- 최종 WinUI shell에서 Windows Media Foundation 또는 VLC 계열 fallback 검토
- source file, proxy file, exported clip의 상태를 별도 표시

## 9. 검증 상태 오해

문제:

- `verified-playable`은 `ffprobe`가 video stream을 본 상태이지 법정 제출 가능성을 뜻하지 않는다.
- 복구 후보가 검증 완료 영상처럼 보이면 보고서 품질이 떨어진다.

필요 기능:

- UI 용어를 `컨테이너 재생 가능`처럼 보수적으로 표현
- `candidate-unvalidated`, `duplicate-candidate`, `validation-failed`, `verified-playable` 구분
- examiner playback review 체크 별도 저장
- 보고서에 검증 한계 문구 유지

## 10. 제조사 독자 포맷

문제:

- CCTV/DVR/NVR/블랙박스 독자 포맷은 단순 FFmpeg 처리로 끝나지 않을 수 있다.
- 제조사별 폴더 구조, event type, GPS/speed, channel metadata가 다르다.

필요 기능:

- parser plugin 구조
- parser confidence 표시
- 실패 샘플 분류
- vendor player 필요 여부 기록
- metadata 출처와 추정 여부 분리

우선 후보:

- Dahua DAV/DHAV
- Hikvision export/path signals
- BlackVue channel suffix
- Thinkware/iNavi event folders
- Wisenet/Hanwha NOV
- Genetec G64/G64x
- Avigilon AVE
- Milestone/XProtect BLK

## 11. 깨진 영상과 부분 복구 파일

문제:

- carving이나 deleted inode recovery 결과는 partial file일 가능성이 높다.
- repair를 원본 후보에 덮어쓰면 forensic trail이 망가진다.

필요 기능:

- repair attempt를 별도 command로 분리
- 원본 candidate 보존
- repaired output은 derived artifact로 저장
- repair 전/후 hash 기록
- command, parameters, tool version 기록
- 실패해도 원본 candidate 상태 유지

## 12. 시간/타임존/메타데이터 신뢰성

문제:

- 블랙박스와 CCTV 장비 시간은 틀린 경우가 많다.
- 파일시스템 시간, 컨테이너 시간, 장비 metadata 시간이 서로 다를 수 있다.

필요 기능:

- 시간 출처별 분리 표시
- file modified time
- container metadata time
- vendor metadata time
- examiner time correction note
- 보고서에 시간 출처와 보정 여부 표시

금지:

- 출처가 다른 시간을 하나의 `사건 시간`으로 조용히 합치지 않는다.

## 13. 보안 리스크

문제:

- 의뢰인 디스크에는 악성 실행파일, 스크립트, 이상한 HTML, 깨진 코덱 샘플이 있을 수 있다.

필요 기능:

- 외부 `.exe` 자동 실행 금지
- report/viewer metadata HTML escaping 유지
- suspicious file type 표시
- vendor player 실행은 명시적 수동 절차로만 문서화
- case folder 외부 쓰기 제한

## 14. 로그와 감사 체인 검증

문제:

- 로그를 쓰는 것만으로는 부족하다.
- 나중에 누락, 순서 변경, 변조 여부를 확인할 수 있어야 한다.

필요 기능:

- `verify-audit` 명령 — **구현 완료** (`frametrace verify-audit <log>`; 불완전 마지막 줄(중단된 쓰기)은 별도 오류로 보고)
- package 생성 전 audit chain check
- report에 chain status 표시
- 깨진 JSONL line, 누락 hash, 순서 오류 표시

대상 로그:

- `evidence/logs/e01-audit.jsonl`
- `evidence/logs/tsk-audit.jsonl`
- `evidence/logs/validation-log.jsonl`
- `artifacts/carved/carve-log.jsonl`
- `artifacts/clips/export-log.jsonl`
- `artifacts/proxies/proxy-log.jsonl`
- `artifacts/thumbnails/thumbnail-log.jsonl`

## 15. 보고서와 PDF 산출물

문제:

- HTML report는 검토에 좋지만 실제 납품/제출은 PDF가 필요할 수 있다.
- 표 overflow, 한글 폰트, 페이지 나눔이 깨질 수 있다.

필요 기능:

- PDF export
- 한글 폰트 포함 또는 명시
- page break rule
- 긴 path/hash wrapping
- report language 설정
- 검증 한계/복구 한계 자동 포함

## 16. 작업 큐와 취소

문제:

- GUI에서 여러 작업을 동시에 돌리면 같은 case DB/log/artifact에 충돌이 생길 수 있다.

필요 기능:

- job queue
- 같은 case에 위험 작업 동시 실행 제한
- cancel-safe state
- failed job retry
- stdout/stderr viewer
- job별 log link

동시 실행 주의:

- 같은 source에 대한 scan 중복
- 같은 artifact에 대한 export/proxy/validation 중복
- package 생성 중 다른 job 실행
- report 생성 중 log write

## 17. 우선순위 매트릭스

| 우선순위 | 항목 | 이유 |
| --- | --- | --- |
| P0 | 원본 증거 보호 | 실수하면 forensic integrity가 깨진다 |
| P0 | dependency checker | Windows에서 가장 먼저 막힐 가능성이 높다 |
| P0 | 진행률/ETA | 대용량 작업 실무 사용성의 기본이다 |
| P0 | 대량 리스트 virtualization | 1,000개 이상 파일에서 GUI freeze를 막아야 한다 |
| P0 | audit chain 검증 | 보고서/패키지 신뢰성에 직접 연결된다 |
| P1 | 중단/재개 checkpoint | 장시간 작업의 실무 안정성을 높인다 |
| P1 | 디스크 공간 preflight | E01/raw/export/package 실패를 줄인다 |
| P1 | Windows path hardening | 한글/공백/긴 경로 문제를 줄인다 |
| P1 | 검증 상태 UX | 복구 후보와 검증 가능 영상을 혼동하지 않게 한다 |
| P2 | PDF export | 납품 품질을 높인다 |
| P2 | repair attempt | 깨진 후보 처리력을 높인다 |
| P2 | vendor metadata expansion | 실제 제조사 대응력을 높인다 |

## Windows GUI 진입 조건

최종 WinUI GUI 구현을 시작하기 전에 최소한 아래가 설계되어 있어야 한다.

- job progress event schema
- dependency check schema
- audit verify command
- source/output path safety rule
- large-list query model
- validation status wording
- failed job retry/cancel model

이 조건이 없으면 GUI는 예쁘게 만들 수는 있어도 실무에서 불안정하게 느껴질 가능성이 높다.
