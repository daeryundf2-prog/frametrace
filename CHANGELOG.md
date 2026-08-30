# Changelog

## 0.4.0 — M3 "제품화" + M2 완료 (2026-08-30)

- 이중 바이너리: `frametrace-app.exe`(콘솔 없는 검수 런처, 시작 실패 시 메시지 박스) + `frametrace.exe`(CLI).
- 휴대용 배포: `scripts/make-portable.ps1` → exe·문서·tools 안내를 포함한 zip.
- 보고서: "인쇄 / PDF로 저장" 버튼, 인쇄 레이아웃 강화(표 머리 반복, 제목-표 페이지 나눔 규칙).
- 런처: 뷰어에서 내려받은 판독 마크 JSON을 업로드해 `import-marks` → 보고서 갱신.
- 접근성: 그리드 카드 키보드 포커스/활성화, 히스토그램 라벨 10px+aria-label,
  `검증 대기` 칩 대비(AA), 태그 칩 팔레트 색(#1c5d8f), 단축키 모달 포커스 이동, 태그 입력 포커스 유지.
- DAV(구현은 0.3.0에서 시작, 본 버전에서 검증 확장): 실 H.264 기반 리먹스 E2E 통합 테스트.

## 0.3.0 — M2 "복원력" (2026-08-30)

- Dahua DAV 1종 파서: DHAV 컨테이너 워커(프레임 타입/채널/페이로드), 비디오 ES 추출,
  ffmpeg h264/hevc 리먹스, `export-dav` 명령(export-log 체인 기록).
  **실장비 샘플 검증은 대기** — 합성 컨테이너 + 실 H.264 ES로 E2E 검증 완료.
- 뷰어: 삭제 영상 후보(kind `candidate`) 레코드·출처 필터·`action: "recover"` 선택 내보내기.
- CLI `recover-batch`: 선택 목록 일괄 inode 복구(개별 tsk-audit + 배치 체인 기록).
- ffprobe JSON 파싱을 serde로 전환(손작성 파서 제거), format.duration 우선 순위 유지.
- libewf 라운드트립 통합 테스트 추가 — libewf 설치 시 CI에서 자동 활성화(현재 skip).

## 0.2.0 — M1 "신뢰성" (2026-08-30)

- 단일 exe 검수 런처(`serve.rs`): INPUT 위저드(폴더/E01), 파이프라인 구동, /media Range 스트리밍,
  경로 컨테인먼트. 결과 보고서 확장(발견·분석 기법, 처리 체인, 판독 마크).
- 테스트 77 → 90+ (serve/report 단위 + 서버 소켓 통합), ffmpeg 실실행 통합 테스트 CI 편입.
- 썸네일/validate-batch 병렬화(워커풀, 감사 append 직렬화로 체인 보존).
- 감사로그: 배타락 + append + fsync 원자화, 불완전 꼬리(중단 쓰기) 거부/별도 보고.
- 프로세스 타임아웃: probe 120초, `import-e01`/`export-video`/`recover-inode --timeout`.
- 케이스 상태파일 원자적 쓰기, `unique_path` 덮어쓰기 버그 수정, ewfexport 부분 raw 정리.
- 뷰어: 썸네일 외부화(1만 건 HTML 195MB→13.5MB), 4~5열 밀도, 좌우 드래그 스플리터, 대형 플레이어,
  강제 다크모드 방어, 대시보드 헤더 버그 수정. CSS 게이트(scripts/check-css.mjs).
- ffprobe `format.duration` 우선, 빈 케이스 make-review 안내, 문서 동기화(스키마 v3, 전제조건 등).
