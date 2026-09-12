# FrameTrace 로드맵 (2026-09-13 v3 — 잔여 항목 정리)

> v2 로드맵(2026-09-08, 종합 8.2) 이후 실측 검증 세션 결과를 반영한 현재 상태 문서.
> 목적: 코드로 해결 가능한 항목은 전부 소진됐고, 남은 것은 외부 자산/결정이 필요한
> 항목뿐임을 명확히 기록한다.

## 1. 이번 세션 실측 검증 (2026-09-13, macOS arm64)

| 항목 | 방법 | 결과 |
|---|---|---|
| **실제 E01 intake** | `ewfacquire`로 encase6 E01 합성(16 MiB, MD5+SHA256 acquisition digest) → `init-case` → `inspect-e01` → `import-e01` | ✅ ewfverify 통과, raw export sha256이 acquisition digest와 일치 (`b0cd3967…`) — chain-of-custody 해시 라운드트립 실증 |
| **E01 → carve** | export된 raw에 대해 `carve-file` 실행 | ✅ 심어둔 DHAV 시그니처 1건을 `carved-candidate`로 발견, `candidate-unvalidated` 라벨 정상 |
| **합성 DAV intake** | DHAV 헤더 + 실제 H.264 ES(ffmpeg 생성) 3종(continuous/event/parking) → `export-dav` | ✅ 3종 전부 `es-extract-remux`로 MP4 생성, ffprobe `mov,mp4` 포맷 인식, duration 0.92s |
| **합성 HIK intake** | IMKH 40바이트 헤더 + MPEG-PS(ffmpeg 생성) 3종 → `export-hik` | ✅ 3종 전부 `imkh-strip-remux`로 MP4 생성, ffprobe 인식, duration 1.0s |
| **출력 confinement** | case 디렉터리 밖 `--output` 지정 시도 | ✅ "must be inside the case directory"로 거부 — 보안 계약 정상 |
| **감사로그** | `verify-audit` on e01-audit.jsonl, timeline-log.jsonl | ✅ chained 검증 통과 (structural-only, 키 미설정 상태) |
| **키 rotation E2E** | `rotate-audit-key --key-id k1 --log` → `k2` → `verify-audit` | ✅ 2세대 signed marker가 keyring의 retired 키로 전부 `integrity-keyed` 검증 (tests/cli_smoke.rs) |
| **릴리즈 패키징** | `scripts/build-release.sh` | ✅ `frametrace-0.5.0-aarch64-apple-darwin.zip` + SHA256SUMS 생성·검증 통과. 스크립트 마지막 조건식이 exit 1을 반환하던 미세 결함 수정 |
| **fuzz 스모크** | nightly + ASAN, 4개 타깃 | ✅ ~9M executions, 크래시 0 (audit_jsonl_verify 3.66M, creation_time_unix 3.67M, index_jsonl_records 0.95M, carve_signature_scan 0.76M) |
| **워크스테이션 E2E** | `FRAMETRACE_IT=1` HTTP 파이프라인 | ✅ 로컬 통과 |
| **CI** | push된 head( a1c6aa9 ) | ✅ Windows + macOS dual-OS green |

### 합성 샘플의 한계 (정직성 기록)

DAV/HIK 샘플은 FFmpeg-aligned 스켈레톤이다 — **레코더 산출물이 아니다**.
실증된 것은 intake/remux 경로뿐이고, 실제 필드 검증은 여전히 실기기 DAV·
Hikvision 익스포트 코퍼스가 필요하다(`validate-dav-samples.ps1` 런북 준비됨).
E01은 libewf가 실제로 쓴 정품 포맷이므로 intake·verify·export 경로는 실증됐지만,
파일시스템 복원 정확도는 mmls/sleuthkit 의존이며 손상·부분 E01 같은
엣지는 실제 인수 이미지가 필요하다.

## 2. 잔여 항목 (전부 외부 조건 — 코드로 해결 불가)

| # | 항목 | 차단 요인 | 필요한 것 |
|---|---|---|---|
| 1 | 실물 DAV/Hikvision 코퍼스 검증 | 레코더 산출물 부재 | 실기기 익스포트 3종 이상 |
| 2 | 손상/부분 E01 + Ex01 검증 | 인수 이미지 부재 | 실제 케이스 이미지 |
| 3 | Windows 실기기 풀레인 | 이 호스트는 macOS | Windows 머신 (`WINDOWS_IMPLEMENTATION_HANDOFF.md` 런북) |
| 4 | 대용량 스케일 런 (1만 건 E01) | 대용량 증거 부재 | 실제 사건급 볼륨 |
| 5 | 릴리즈 서명 | cosign/cargo-sbom 미설치 + OIDC 키 결정 | 서명 키 인프라 결정 (스크립트는 준비됨, `--sign`/`--sbom` 플래그) |
| 6 | OS keystore 통합 (Keychain/DPAPI) | 무거운 의존성 도입 결정 | `--key-source env:`로 이미 커버 — 필요 시에만 |
| 7 | WinUI/풀 GUI | 제품 결정 | 브라우저 런처 실무 피드백 우선 (v2 §4 결정 3) |
| 8 | 제3자/법정 검증 | 외부 프로세스 | 법실무 자문 + 독립 검증 |

## 3. 점수 궤적

```
v2 기록 8.2 → v3 ~9.0   코드 가능 항목 소진 + 실측 검증 다수 확보
9.0 → 9.5+             위 잔여 항목(실물 코퍼스·필드 실적) 필요
```
