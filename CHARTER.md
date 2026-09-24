# CHARTER — frametrace

## Role
Evidence plane — Rust 기반 미디어 복구/분석 (내부 Alpha).

## Do
- 미디어 파일 복구·프레임 분석, streaming SHA-256
- SQLite/JSONL 산출물 패키징

## Don't
- 최종 symlink 컴포넌트 미검사 출력 경로 허용 금지 (tool_policy 전체 canonicalize)
- 인증 없는 `"app":"frametrace"` 문자열 기반 workstation 신뢰 금지
- 생산 환경 다중 사용자 노출 금지 (내부 Alpha)

## Contracts
- Consumes: —
- Produces: 복구 아티팩트 (lazy-evidence-case-v1로 래핑 권장)
- Vendored: `contracts/` (lazy-contracts, hash-pinned)

## Claims allowed
`observed`, `heuristic`. Rust 구현에서 path confinement는 `lazy-contracts/path-confine` 규칙을 포팅해 적용.
