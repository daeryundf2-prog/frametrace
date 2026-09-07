# macOS 풀 레인 검증 영수증 (2026-09-08)

> 대상: v0.5.0 이후 main (serde 2단계 + DAV gap anomaly 커밋 포함).
> 머신: macOS arm64, ffmpeg/ffprobe/libewf(20140816)/Sleuth Kit/mtools/homebrew Node.
> 모든 수치는 `target/release` 바이너리로 실측.

## 게이트

| 게이트 | 결과 |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test` | 118 passed (lib 115 + smoke 3) |
| `FRAMETRACE_IT=1 -- --ignored` | 4 passed (ffmpeg/libewf 실실행) |
| `scripts/check-css.mjs` + `node --check` | PASS |

## 풀 CLI 라이프사이클 (case-001, 6개 증거)

init-case → register-source → scan-folder --hash(4: 정상 3 + 손상 1) →
make-thumbnail / make-proxy / export-video(1s mp4) →
export-batch --dry-run(2 ok, 1 skipped-by-design) → validate-batch(3/3 ok,
손상 파일은 validation-failed 분기) → import/export-marks(2건 라운드트립) →
make-review(썸네일 3 created + 1 unavailable) → make-report →
qa report-defense PASS → package-case(16 files, manifest 해시 포함).

## E01 / 이미지 복구 레인 (실도구)

1. `ewfacquire -u -t … -f raw`(libewf 20140816 문법)로 1 MiB payload.raw → E01 생성.
2. `inspect-e01` → ewfinfo 메타 기록.
3. `import-e01` → ewfverify MD5 일치 확인 → raw 익스포트(1,048,576 B) + SHA-256.
4. `carve-file` → raw 내 MP4 1건 카빙(carve_000001) → validate-artifact로
   `ffprobe-video-stream-confirmed`.
5. mtools로 FAT 이미지(kept.mp4 + 삭제 deleted.mp4) 작성 →
   `inspect-image`(fls: r/r 6 kept, r/r * 8 deleted) →
   `recover-inode 8 --recover-deleted` → 복구 inode_8.bin SHA-256이
   원본 REAR fixture(206f4348…)와 **완전 일치**.

## 독점 포맷 레인

- DAV(합성 DHAV, 실 h264 ES 21프레임): `export-dav` → es-extract-remux,
  출력 1.92s 재생 확인.
- Hikvision IMKH(40B 헤더 + MPEG-PS): `export-hik` → imkh-strip-remux,
  출력 2.00s 재생 확인.

## 성능 예산 (1,000건 실측)

| 항목 | 예산 | 실측 | 판정 |
|---|---|---|---|
| scan --no-ffprobe 1,000건 | - | <1s | PASS |
| 썸네일 1,000건 생성 | ≤180s | 13s | PASS |
| ffprobe 검증 1,000건 | ≤240s | 32s | PASS |
| benchmark-db 10,000 rows | - | 591ms | PASS |
| make-review(썸네일 캐시) | - | 1s | PASS |

## QA 전판

- `qa accuracy` PASS (corpus 6행, 해시 없는 행은 공란 — 매처 계약 확인)
- `qa reproducibility` PASS
- `qa report-defense` PASS
- `qa release` PASS (4/4)
- `qa anomalies`: 0 candidate findings (합성 fixture는 정상 시계라 기대치)

## 감사 체인

- case-001: anomaly/e01/tsk/validation 로그 4/4 `verify-audit` PASS.
- case-perf: 1/1 PASS.
- 변조 레드 테스트: 로그 1문자 변경 → `entry hash mismatch` 즉시 적색.

## 워크스테이션 런처

`frametrace-app`(인자 없음) → 127.0.0.1:8477 기동, `GET /` 200,
`/api/status` 5단계 파이프라인(케이스 준비→소스 등록→스캔·색인→재생성
검증→리뷰 생성) idle 상태 응답 확인.

## 유의사항

- macOS libewf 20140816의 `ewfacquire`는 무조건 unattended `-u` 필요,
  목표 파일은 위치 인자가 아닌 `-t`(확장자 제외). Windows 체인
  (scripts/build-libewf-tools.ps1)과 문법이 다르므로 로컬 실행 시 유의.
- FAT fixture는 mformat/mcopy/mdel(mtools)로 마운트 없이 작성.
- DAV/Hik 레인은 합성 샘플 기준 — 실장비 코퍼스 검증은 계속 대기(로드맵 원칙 유지).
