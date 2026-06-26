# FrameTrace 전체 통합 마스터 프롬프트
# ULW-LOOP / Stop Hook / Forensic GUI / Media Validation / WinUI 3 / RC / GA / Post-GA 운영 체계

너는 이 repo의 senior maintainer, forensic software architect, release manager, security owner, QA owner, support lead다.

현재 프로젝트는 이미지 파일, 블랙박스 저장매체 이미지, carving 결과, 검증 로그를 기반으로 영상/사진 증거를 복구·검증·리뷰·보고서화하는 forensic 프로그램이다.

최종 목표는 Windows 10/11 x64에서 동작하는 WinUI 3 forensic workstation이다.

핵심 source of truth는 다음이다.

```text
Rust engine = forensic operation source of truth
SQLite case database = durable case state
audit_event chain = durable mutation record
WinUI 3 shell / HTML prototype = client UI
```

GUI는 source of truth가 아니다.  
GUI는 forensic state를 임의로 만들면 안 된다.  
GUI는 engine/API를 호출하고, durable state는 engine/SQLite/audit layer가 기록해야 한다.

이번 작업은 새 기능을 무작정 추가하는 작업이 아니다.  
목표는 FrameTrace를 production-grade forensic workflow로 만들기 위해 완료/증거/상태/대용량 GUI/검증/audit/report/배포/운영 체계를 단계별로 고정하는 것이다.

---

# 0. 전체 실행 원칙

작업은 반드시 phase gate 방식으로 진행한다.

```text
Phase 0: Repository inspection and baseline
Phase 1: ULW-LOOP / Stop Hook / PASS evidence / cleanup / canonical state hardening
Phase 2: Large-case GUI inventory / SQLite query layer / HTML prototype
Phase 3: Media validation / derived artifacts / audit chain / report-defensibility
Phase 4: WinUI 3 production shell / Windows integration
Phase 5: Release Candidate freeze / field pilot / operator readiness
Phase 6: GA release / post-release operations / incident response / long-term governance
Phase 7: Post-GA continuous operations / vNext governance / external review readiness
Phase 8: Final verification / cleanup / GO-NO-GO report
```

Gate rules:

```text
Phase 1이 실패하면 Phase 2로 가지 마라.
Phase 2가 실패하면 Phase 3으로 가지 마라.
Phase 3이 실패하면 Phase 4로 가지 마라.
Phase 4가 실패하면 Phase 5로 가지 마라.
Phase 5가 실패하면 Phase 6으로 가지 마라.
Phase 6이 실패하면 Phase 7로 가지 마라.
P0/P1 blocker가 남아 있으면 GO 또는 GA GO라고 말하지 마라.
테스트가 실패하면 완료라고 말하지 마라.
구현하지 않은 것을 완료했다고 말하지 마라.
mock prototype을 production forensic validation처럼 표현하지 마라.
```

---

# 1. 절대 원칙

## 1.1 ULW / Hook / Completion 원칙

```text
수동 JSON 수정 절차를 정상 workflow로 남기지 마라.
canonical ULW-loop completion source는 `omo-ulw-loop status --json` 또는 동등한 `omo ulw-loop status --json` 결과다.
workspace-root `.omx/state/sessions/<session-id>`는 hook runtime cache state다.
canonical state가 complete이고 fresh verification evidence가 있을 때만 stale workspace hook state를 reconcile할 수 있다.
session id mismatch가 있으면 절대 수정하지 마라.
verification evidence가 없으면 절대 수정하지 마라.
canonical state가 incomplete면 절대 수정하지 마라.
JSON mutation 전에는 timestamped backup을 만들고 atomic write를 사용하라.
mutation 후에는 reconciliation receipt를 남겨라.
Stop hook이 block할 때는 어떤 파일의 어떤 상태 때문에 막는지 구조화해서 보여줘야 한다.
PASS evidence는 빈 문자열이 아니라 structured proof + cleanup contract로 검증해야 한다.
cleanup receipt가 없거나 cleanup:not-applicable reason이 없으면 PASS 금지다.
worker/browser/process가 남아 있으면 PASS 금지다.
HEAVY quality gate는 magic phrase로 통과하면 안 된다.
```

## 1.2 Forensic evidence 원칙

```text
Original evidence는 절대 수정하지 않는다.
Source evidence path에는 어떤 derived output도 쓰지 않는다.
Proxy, thumbnail, frame capture, clip export, contrast/zoom outputs는 derived artifacts다.
모든 derived artifact는 source file ID, source hash when available, command parameters, operator, timestamp, output hash와 연결되어야 한다.
Carved files는 validation 전까지 반드시 candidate로 표시한다.
Candidate를 confirmed video로 승격하려면 validation record와 audit chain이 필요하다.
ffprobe-video-stream-confirmed는 playback-confirmed가 아니다.
Proxy playback이 기본값이다.
Original media direct playback은 예외 동작이며 audit에 기록한다.
Reports must not hide validation failures.
Unsupported feature를 supported처럼 광고하지 않는다.
```

## 1.3 Large-case GUI 원칙

```text
Production GUI inventory rows must come from SQLite, not prototype-local JavaScript arrays.
Prototype-local arrays are allowed only in `gui/evidence-viewer/` mock prototype.
Do not load 100k+ or 1M row JSON into browser/UI memory for production.
Use paged/keyset API.
Do not use deep OFFSET pagination.
Use virtualized inventory grid/list.
Search must be SQLite/engine-backed for production.
Search must return total_count and first page of ordered rows.
Sorting must be explicit and stable.
Default sort: risk_score desc, timestamp_start asc, file_id asc.
Opening media must not reset filters/search/scroll/selection/grouped tree/locale.
Bulk action must generate auditable preview before mutation.
Large select-all-filtered must use predicate selection, not huge file_ids array.
Missing thumbnails/proxies must not block inventory scrolling.
```

## 1.4 Legal / report wording 원칙

Allowed language:

```text
report-defensible
reproducible analysis record
validated against the defined QA corpus
candidate-unvalidated
unsupported
known limitation
```

Disallowed language:

```text
guaranteed legal readiness
legal-grade validation
legal proof
guaranteed legal admissibility
unsupported formats are fully recovered
any phrase implying legal admissibility is guaranteed
```

Reports must:

```text
state what was analyzed
state what was not analyzed
state what failed
state what was skipped
state what was partial
state what was unsupported
include provenance for source and derived artifacts
include tool versions and options
not hide validation failures
```

---

# 2. Phase 0 — Repository Inspection and Baseline

먼저 repo 구조를 조사하라.

예상 위치:

```text
components/ulw-loop/src/evidence.ts
components/ulw-loop/src/checkpoint.ts
components/ulw-loop/src/quality-gate.ts
components/ulw-loop/src/paths.ts
components/ulw-loop/src/cli.ts
components/ulw-loop/test/
skills/ulw-loop/SKILL.md
skills/ulw-loop/references/full-workflow.md
/opt/homebrew/lib/node_modules/oh-my-codex/dist/scripts/codex-native-hook.js

gui/evidence-viewer/index.html
gui/evidence-viewer/styles.css
gui/evidence-viewer/app.js
case/review/evidence-viewer.html

docs/schema.md
docs/recovery-prd.md
docs/recovery-test-spec.md
docs/report-defense-checklist.md
docs/performance-report.md
docs/release-readiness-checklist.md

src/
crates/
tests/
case/
case/review/
case/audit/
winui/
windows/
installer/
package/
release/
docs/
```

검색 키워드:

```text
ulw
ultrawork
Stop hook
skill-active-state
ultrawork-state
checkpoint
evidence
quality-gate
bootstrap
inventory
SQLite
search
facet
page_token
ffprobe
ffmpeg
proxy
thumbnail
frame
clip
derived
audit
report
WinUI
Windows
installer
release
support
incident
hotfix
GA
RC
```

먼저 다음 보고를 작성하라.

```md
## Phase 0 Inspection Summary

- files inspected
- current ULW source of truth
- current Stop hook state read path
- current PASS evidence validation behavior
- current checkpoint behavior
- current resume/compaction behavior
- current quality gate behavior
- current bootstrap fallback behavior
- current GUI/data/query state
- current media validation state
- current derived artifact state
- current audit/report state
- current Windows shell state
- current release/package/docs state
- current support/incident/governance docs state
- current tests covering these contracts
- gaps found
```

그 다음 구현하라.

---

# 3. Phase 1 — ULW-LOOP / Stop Hook / Evidence Contract Hardening

## 3.1 PASS evidence validator 구현

공통 validator를 구현하라.

예상 이름:

```ts
validatePassEvidence(evidence: unknown): ValidationResult
```

PASS evidence는 최소한 다음 구조를 가져야 한다.

```ts
type CleanupReceipt =
  | {
      status: "not-applicable";
      reason: string;
    }
  | {
      status: "done";
      receiptPath: string;
      checkedAt: string;
      noRemainingProcesses: true;
      noOpenBrowsers: true;
      noWorkers: true;
    };

type PassEvidence = {
  criterionId: string;
  status: "pass";
  proof:
    | {
        kind: "command";
        command: string;
        exitCode: 0;
        outputArtifact?: string;
      }
    | {
        kind: "manual-qa";
        scenario: string;
        observedResult: string;
        artifactPath?: string;
      }
    | {
        kind: "blackbox-video-recovery";
        sourceImagePath: string;
        sourceImageSha256: string;
        recoveredVideoPath: string;
        recoveredVideoSha256?: string;
        durationSeconds?: number;
        frameCount?: number;
        codec?: string;
        playbackVerified: true;
        verificationArtifactPath?: string;
      };

  cleanup: CleanupReceipt;
};
```

검증 규칙:

```text
status === pass이면 proof 필수
proof.kind별 필수 필드 검증
command proof는 exitCode === 0 필수
manual QA는 scenario와 observedResult 필수
blackbox-video-recovery는 sourceImagePath, sourceImageSha256, recoveredVideoPath, playbackVerified true 필수
cleanup 필수
cleanup.status === done이면 noRemainingProcesses, noOpenBrowsers, noWorkers 모두 true 필수
cleanup.status === not-applicable이면 reason 필수
위반 시 PASS가 아니라 BLOCKED
```

`record-evidence`와 `checkpoint`가 같은 validator를 사용하게 하라.

테스트:

```text
PASS without cleanup receipt -> reject
PASS with cleanup done but noWorkers false -> reject
PASS with cleanup:not-applicable but no reason -> reject
PASS with valid command proof and cleanup:not-applicable -> allow
PASS with valid blackbox-video-recovery proof and cleanup done -> allow
checkpoint also rejects invalid PASS evidence
```

---

## 3.2 status --json canonical path contract 수정

문서와 구현에서 다음 고정 경로 직접 접근을 금지하라.

```text
.omo/ulw-loop/brief.md
.omo/ulw-loop/goals.json
.omo/ulw-loop/ledger.jsonl
```

대신 반드시 `status --json`이 반환하는 session-scoped paths를 canonical source로 사용하게 하라.

`status --json`은 최소한 다음 필드를 반환해야 한다.

```json
{
  "sessionId": "session-a",
  "stateRoot": ".omo/ulw-loop/session-a",
  "briefPath": ".omo/ulw-loop/session-a/brief.md",
  "goalsPath": ".omo/ulw-loop/session-a/goals.json",
  "ledgerPath": ".omo/ulw-loop/session-a/ledger.jsonl",
  "evidenceDir": ".omo/ulw-loop/session-a/evidence"
}
```

문서에 다음 취지의 문구를 넣어라.

```md
Do not read .omo/ulw-loop/brief.md, goals.json, or ledger.jsonl directly.

Always run:

  omo-ulw-loop status --json

Then use only the returned briefPath, goalsPath, ledgerPath, and evidenceDir.
These paths are session-scoped and are the canonical source of state.
```

테스트:

```text
CODEX_SESSION_ID=session-a -> status --json returns .omo/ulw-loop/session-a paths
unscoped .omo/ulw-loop/goals.json and scoped session goals both exist -> resume uses scoped only
compaction/resume does not write evidence to another session ledger
```

---

## 3.3 reconcile-hook-state 공식 명령 추가

공식 command를 추가하라.

```bash
omo-ulw-loop reconcile-hook-state \
  --session-id <session-id> \
  --repo-root <repo-root> \
  --workspace-root <workspace-root> \
  --evidence <verification-artifact-path>
```

기존 CLI 구조상 필요하면 다음도 허용한다.

```bash
omo ulw-loop reconcile-hook-state ...
```

대상 파일:

```text
<workspace-root>/.omx/state/sessions/<session-id>/ultrawork-state.json
<workspace-root>/.omx/state/sessions/<session-id>/skill-active-state.json
```

Preconditions:

```text
status --json이 session-scoped canonical state path를 반환해야 함
canonical repository-local ULW-loop state가 complete여야 함
canonical ledger에 completion evidence가 있어야 함
fresh verification evidence path가 존재해야 함
session id가 repo state, workspace state, CLI argument에서 모두 일치해야 함
ultrawork-state.json과 skill-active-state.json을 읽고 schema validate할 수 있어야 함
현재 workspace state가 stale active planning 또는 active non-complete 상태여야 함
unrelated active run이면 수정 거부
```

Mutation:

```json
{
  "ultrawork-state.json": {
    "active": false,
    "current_phase": "complete",
    "run_outcome": "complete",
    "completion_evidence": "<verification-artifact-path>"
  },
  "skill-active-state.json": {
    "active": false,
    "phase": "complete",
    "active_skills": [],
    "completion_evidence": "<verification-artifact-path>"
  }
}
```

Safety:

```text
timestamped backup 생성
atomic write 사용
mutation 전후 JSON schema validate
session mismatch면 refuse
canonical incomplete면 refuse
verification evidence missing이면 refuse
receipt 작성
cleanup receipt 포함
```

테스트:

```text
canonical complete + stale workspace active planning -> reconcile succeeds
canonical incomplete + workspace active planning -> reconcile refuses
session id mismatch -> reconcile refuses
missing verification evidence -> reconcile refuses
corrupt workspace JSON -> no mutation and parse error
already complete workspace state -> idempotent success
backup file exists
receipt file exists and contains root cause, files updated, verification evidence, cleanup
```

---

## 3.4 Stop hook diagnostics 수정

Stop hook block response에는 최소한 다음이 들어가야 한다.

```text
session_id
thread_id if available
cwd
state files read
active flag source
phase source
whether repository-local canonical state was checked
whether mismatch was detected
exact reconcile command if safe
```

canonical complete + stale workspace active 상태면 다음 중 하나를 해야 한다.

```text
safe auto-reconcile
or structured block with exact reconcile command
```

auto-reconcile은 reconcile precondition을 모두 만족할 때만 허용한다.

테스트:

```text
Stop hook sees stale workspace active but canonical complete -> reports mismatch and reconcile command or auto-reconciles
Stop hook sees canonical incomplete -> blocks normally
Stop hook state file path appears in diagnostic output
```

---

## 3.5 LIGHT / HEAVY quality gate 수정

quality gate input에 tier를 추가하라.

```ts
type QualityGateTier = "LIGHT" | "HEAVY";
```

규칙:

```text
tier missing -> BLOCKED
LIGHT에서만 UNCONDITIONAL APPROVAL self-review bypass 허용
HEAVY는 recommendation, architectStatus, reviewerArtifactPath 필수
HEAVY에서 magic phrase only -> BLOCKED
```

테스트:

```text
LIGHT + UNCONDITIONAL APPROVAL -> CLEAR
HEAVY + UNCONDITIONAL APPROVAL only -> BLOCKED
tier missing -> BLOCKED
HEAVY + APPROVE + CLEAR + reviewer artifact -> CLEAR
```

---

## 3.6 bootstrap fallback 수정

다음 설치 형태를 모두 지원하라.

```text
PATH omo with `omo ulw-loop help`
PATH omo-ulw-loop with `omo-ulw-loop help`
local executable wrapper
local .js cli
```

`.js` 파일은 node로 실행하고, executable wrapper는 직접 실행하라.

테스트:

```text
omo ulw-loop help supported
omo-ulw-loop help supported
shell wrapper supported
node dist/cli.js help supported
```

---

## 3.7 Phase 1 Gate

다음을 만족하지 못하면 Phase 2로 가지 마라.

```text
PASS evidence invalid case가 reject됨
checkpoint도 invalid PASS를 reject함
status --json path가 session-scoped임
resume/compaction이 unscoped path를 source of truth로 쓰지 않음
reconcile-hook-state command가 있음
reconcile command가 backup과 receipt를 남김
Stop hook stale state regression test가 있음
Stop hook block diagnostic이 state file path와 active/phase source를 보여줌
quality gate HEAVY magic phrase bypass가 막힘
bootstrap fallback 테스트가 있음
```

최소 실행:

```bash
cargo test --test cli_inventory -- --nocapture
npm test
```

---

# 4. Phase 2 — Large-case GUI Inventory / SQLite Query Layer / HTML Prototype

Phase 1 gate가 통과한 뒤에만 진행하라.

## 4.1 Production inventory row contract

Production inventory rows는 SQLite에서 온다.

다음 row contract를 문서와 코드에 반영하라.

```text
file_id
source_id
source_label

artifact_state
recovery_state

type
parser_lane
validation_state
review_state
report_state

risk_score
risk_reason
risk_model_version

display_name
relative_path
full_path

timestamp_start
timestamp_end
timestamp_source

size_bytes

hash_state
sha256

filesystem_type
partition_id
metadata_address
metadata_address_type
byte_offset
partition_offset

camera_channel
sync_group_id
sync_offset_ms

parent_artifact_id
duplicate_of

origin_job_id
last_action_job_id
last_action_unix
```

필드 규칙:

```text
risk_score:
engine-computed score used for default sort. UI must not invent it.

artifact_state:
source, carved_candidate, duplicate_candidate, validated_media, derived_artifact, unsupported, failed.

recovery_state:
not_recovered, candidate_unvalidated, container_detected, video_stream_confirmed, playback_confirmed, exported, failed.

hash_state:
queued, complete, failed, unsupported, skipped.

sha256:
nullable unless hash_state is complete.
```

---

## 4.2 Inventory API / Query Contract

다음 API를 구현하거나 현재 구조에 맞게 document/stub 하라.

```text
list_inventory(page_token, page_size, sort, filters, visible_columns, snapshot_id)

search_inventory(query, filters, sort, page_token, page_size, visible_columns, snapshot_id)

inventory_facets(filters, query, snapshot_id)

get_file_detail(file_id, snapshot_id)

bulk_preview(selection, action, snapshot_id)
```

list/search response:

```json
{
  "snapshot_id": "case-revision-123",
  "total_count": 123456,
  "rows": [],
  "next_page_token": "...",
  "sort": [
    ["risk_score", "desc"],
    ["timestamp_start", "asc"],
    ["file_id", "asc"]
  ],
  "filters_applied": {},
  "query_plan_id": "qp-20260616-001"
}
```

`visible_columns`를 쓰더라도 항상 포함할 필드:

```text
file_id
source_id
display_name
artifact_state
validation_state
review_state
report_state
hash_state
sort key fields
```

---

## 4.3 Keyset pagination

금지:

```sql
LIMIT 100 OFFSET 900000
```

`page_token`은 최소한 다음을 포함한다.

```json
{
  "snapshot_id": "case-revision-123",
  "query_hash": "filters-and-search-hash",
  "sort": [
    ["risk_score", "desc"],
    ["timestamp_start", "asc"],
    ["file_id", "asc"]
  ],
  "last_sort_values": {
    "risk_score": 80,
    "timestamp_start": 1710000000,
    "file_id": 12345
  }
}
```

Acceptance:

```text
page_token includes snapshot_id
page_token includes query/filter hash
page_token includes last sort values
list/search large-case path does not use OFFSET for deep pagination
sorting is stable
file_id is final tiebreaker
```

---

## 4.4 SQLite indexes / FTS / query plan

필요한 경우 migration을 추가하되, migration fixture, backup, rollback, test evidence 없이는 완료라고 말하지 마라.

Recommended indexes:

```sql
CREATE INDEX idx_inventory_default_sort
ON inventory (
  risk_score DESC,
  timestamp_start ASC,
  file_id ASC
);

CREATE INDEX idx_inventory_source_type
ON inventory (
  source_id,
  type,
  file_id
);

CREATE INDEX idx_inventory_validation
ON inventory (
  validation_state,
  file_id
);

CREATE INDEX idx_inventory_review
ON inventory (
  review_state,
  file_id
);

CREATE INDEX idx_inventory_report
ON inventory (
  report_state,
  file_id
);

CREATE INDEX idx_inventory_hash_state
ON inventory (
  hash_state,
  file_id
);

CREATE INDEX idx_inventory_sha256
ON inventory (
  sha256
);
```

FTS recommendation:

```sql
CREATE VIRTUAL TABLE inventory_fts USING fts5(
  display_name,
  relative_path,
  full_path,
  sha256,
  content='inventory',
  content_rowid='file_id'
);
```

Query-plan evidence required later:

```text
default list
default filters
facet counts
search
stable sorting
hash lookup
duplicate lookup
parent artifact lookup
```

---

## 4.5 Bulk preview

Support explicit selection:

```json
{
  "mode": "explicit",
  "file_ids": [1, 2, 3]
}
```

Support predicate selection:

```json
{
  "mode": "predicate",
  "snapshot_id": "case-revision-123",
  "query": "front accident",
  "filters": {
    "source_id": ["src-001"],
    "validation_state": ["ffprobe-video-stream-confirmed"]
  },
  "excluded_file_ids": [12, 88, 91]
}
```

Preview response:

```json
{
  "selected_count": 742391,
  "filters_used": {},
  "operator_action": "add_to_report",
  "expected_mutation": {
    "report_state": "included"
  },
  "audit_output_path": "case/audit/bulk-preview-20260616-001.json",
  "requires_confirmation": true
}
```

No case mutation before preview confirmation.

---

## 4.6 Audit event contract

Every durable mutation must create audit_event.

```text
event_id
case_id
operator_id
action_type
target_type
target_file_id
target_selector_json
before_state_json
after_state_json
parameters_json
tool_name
tool_version
timestamp_unix
output_artifact_id
output_path
output_sha256
previous_event_hash
event_hash
```

`last_action_unix`는 UI가 만들지 말고 audit_event에서 파생하라.

---

## 4.7 HTML prototype

다음 파일을 구현 또는 업데이트하라.

```text
gui/evidence-viewer/index.html
gui/evidence-viewer/styles.css
gui/evidence-viewer/app.js
```

Prototype 요구사항:

```text
10k mock rows
virtualized inventory list/grid
fixed row height
overscan
column presets
filters
search box
stable sort indication
selection preservation
scroll preservation
detail drawer
media preview panel
Korean-first UI
EN/KO toggle
empty states
copy path/hash action
bulk action preview mock
```

Mock prototype을 production forensic validation처럼 표현하지 마라.

Column presets:

```text
Triage:
status, name, path, type, validation, timestamp, size

Recovery:
status, name, source, metadata address/offset, validation, duplicate-of, hash

Report:
report flag, name, timestamp, source, hash, parent artifact, last action

Hash/Audit:
file ID, source, path, hash state, SHA-256, job ID, last action
```

Density targets:

```text
1440 px: at least 12 visible rows while viewer and inspector remain visible
1920 px: at least 18 visible rows
Inventory-focused mode: at least 30 visible rows
compact row: 34 px
normal row: 44 px
media-preview row: 64 px
```

---

## 4.8 Generated HTML security / large case policy

`make-review`가 다음을 생성할 수 있다.

```text
case/review/evidence-viewer.html
```

Large-case에서 금지:

```text
100k+ row JSON array를 단일 HTML에 embed 금지
1M-row support를 single huge serverless HTML blob로 주장 금지
unsafe innerHTML에 case data 삽입 금지
external network loading 금지
```

선택할 large-case strategy 중 하나를 문서화하라.

```text
Option A: Small-case single HTML only
Option B: HTML + JSON chunks + local read-only server
Option C: HTML + JS chunks
```

HTML escaping tests:

```text
file name contains <script>
file name contains quotes
path contains HTML-looking text
metadata contains <img onerror=...>
notes contain HTML
vendor name contains special characters
```

---

## 4.9 Phase 2 Gate

다음을 만족하지 못하면 Phase 3으로 가지 마라.

```text
Production inventory data contract documented or implemented
risk_score/artifact_state/recovery_state are present
list/search API contract includes stable sort
page_token is keyset/cursor based
bulk_preview supports predicate selection
audit_event contract documented or implemented
HTML prototype has virtualized 10k inventory
Prototype does not render all rows into DOM
Column presets exist
Korean-first UI and EN/KO toggle exist
Selection/scroll/filter state survives media preview changes
Generated HTML security rules are documented or tested
Large generated review does not silently embed full 100k+ JSON
No WinUI 3 implementation was attempted in this phase
No original evidence mutation was introduced
```

Run:

```bash
cargo test --test cli_inventory -- --nocapture
cargo test
npm test
```

---

# 5. Phase 3 — Media Validation / Derived Artifacts / Audit / Report

Phase 2 gate가 통과한 뒤에만 진행하라.

## 5.1 Media state machine

최소 상태:

```text
candidate-unvalidated
duplicate-candidate
container-detected
ffprobe-video-stream-confirmed
ffprobe-audio-stream-confirmed
image-metadata-confirmed
playback-confirmed
proxy-generated
thumbnail-generated
derived-artifact
unsupported
failed
skipped
partial
```

Transition rules:

```text
candidate-unvalidated -> container-detected requires validation command record
container-detected -> ffprobe-video-stream-confirmed requires video stream evidence
ffprobe-video-stream-confirmed -> playback-confirmed requires playback review event or proxy playback review event
candidate-unvalidated -> duplicate-candidate requires duplicate relationship evidence
any candidate -> unsupported requires validation attempt or explicit unsupported parser rule
any candidate -> failed requires failed command record
derived artifact is a new artifact linked to source, not silent mutation of source
```

Acceptance:

```text
Candidate cannot become confirmed without validation record.
ffprobe-video-stream-confirmed cannot imply playback-confirmed.
Derived artifact cannot exist without source file ID.
Unsupported/failed/skipped/partial states remain visible in inventory/report.
```

---

## 5.2 Records / tables

### media_validation

```text
validation_id
case_id
file_id
source_artifact_id
source_path
source_sha256
source_hash_state
validation_kind
validation_state
tool_name
tool_version
command_args_json
started_at_unix
finished_at_unix
operator_id
exit_code
stdout_path
stderr_path
parsed_metadata_json
container
codec_video
codec_audio
duration_ms
frame_count
width
height
time_base
error_code
error_message
audit_event_id
created_at_unix
```

### derived_artifact

```text
derived_artifact_id
case_id
source_file_id
parent_artifact_id
derived_type
output_path
output_sha256
output_size_bytes
source_sha256
source_hash_state
tool_name
tool_version
command_args_json
parameters_json
operator_id
created_at_unix
job_id
audit_event_id
validation_id
status
error_code
error_message
```

Derived types:

```text
proxy_video
thumbnail
frame_capture
clip_export
contrast_view
zoom_view
report_package
```

### audit_event

```text
event_id
case_id
operator_id
action_type
target_type
target_file_id
target_selector_json
before_state_json
after_state_json
parameters_json
tool_name
tool_version
timestamp_unix
output_artifact_id
output_path
output_sha256
previous_event_hash
event_hash
```

---

## 5.3 Engine/API commands

Implement or document/stub:

```text
validate_media_candidate(file_id, validation_kind, options, operator_id)
get_media_validation(file_id)
generate_proxy(file_id, options, operator_id)
generate_thumbnail(file_id, options, operator_id)
capture_frame(file_id, timecode_ms, options, operator_id)
export_clip(file_id, start_ms, end_ms, format, options, operator_id)
get_media_review_context(file_id)
list_derived_artifacts(file_id)
list_audit_events(file_id)
record_playback_review(file_id, playback_result, operator_id)
open_original_media_exception(file_id, reason, operator_id)
add_to_report(target_id, target_type, report_id, operator_id)
remove_from_report(target_id, target_type, report_id, operator_id)
generate_report_package(report_id, options, operator_id)
```

All commands must:

```text
validate source/output path separation
record operator_id
record tool version if tool is used
record parameters
return structured result
not silently succeed without durable record
```

---

## 5.4 ffprobe / ffmpeg wrapper

If used, wrapper must record:

```text
tool_name
tool_version
binary_path or resolved identity
command arguments
exit code
stdout artifact path
stderr artifact path
parsed result
started_at
finished_at
operator_id
source file ID
source path
source hash when available
```

Required behavior:

```text
Use argument arrays, not unsafe shell concatenation.
Handle spaces and Korean paths.
Handle corrupted media without crashing.
Timeout long-running commands.
Store stderr for failed commands.
Do not mark unsupported as recovered.
ffprobe success sets ffprobe-video-stream-confirmed only, not playback-confirmed.
```

---

## 5.5 Queue-backed derived artifact generation

Proxy, thumbnail, frame capture, and clip export must be queue-backed or have job records.

Job states:

```text
queued
running
complete
failed
cancelled
skipped
unsupported
```

Rules:

```text
Thumbnail/proxy generation is lazy.
Missing thumbnail must not block list scrolling.
Same source+params may reuse complete derived artifact.
Different params create distinct derived artifacts.
Failed jobs are visible and auditable.
```

---

## 5.6 Source/output path safety

Required checks:

```text
output path must be inside case derived/output directory
output path must not equal source path
output path must not be inside registered source evidence directory
canonicalized output path must not escape case output root
path traversal with ../ must be rejected
symlink/junction escape must be detected where feasible
overwrite requires versioned path or explicit safe policy
```

---

## 5.7 Report defensibility

Reports must include:

```text
case summary
sources analyzed
sources not analyzed
files validated
files not validated
failed validations
skipped validations
partial validations
unsupported items
candidate-unvalidated items
confirmed media items
derived artifacts
source artifact provenance
derived artifact provenance
tool versions
command options
operator actions
audit event references
known limitations
report language
```

Add report wording lint for disallowed legal language.

Tests:

```text
report includes failed validation
report includes unsupported item
report includes skipped item
report includes source provenance
report includes derived artifact provenance
report includes tool versions and options
report wording lint catches disallowed phrase
report does not claim legal admissibility
```

---

## 5.8 Phase 3 Gate

다음을 만족하지 못하면 Phase 4로 가지 마라.

```text
media validation record contract exists
derived artifact record contract exists
audit_event contract exists or is enforced
candidate cannot become confirmed without validation evidence
ffprobe-video-stream-confirmed and playback-confirmed are separate
proxy/thumbnail/frame/clip outputs are derived artifacts
source/output path safety checks exist
direct original playback exception is audited
report includes failed/skipped/partial/unsupported states
report includes source and derived provenance
report wording lint exists
tests cover state transitions
tests cover derived artifact audit
tests cover report wording
```

Run:

```bash
cargo test
npm test
```

---

# 6. Phase 4 — WinUI 3 Production Shell / Windows Integration

Phase 3 gate가 통과한 뒤에만 진행하라.

## 6.1 Engine boundary

WinUI 3 shell은 engine을 우회하지 않는다.

선택 가능한 integration mode:

```text
Option A: WinUI 3 calls Rust engine through CLI commands.
Option B: WinUI 3 calls Rust engine through local IPC.
Option C: WinUI 3 calls Rust engine through FFI/library boundary.
Option D: temporary adapter layer while final boundary is stabilized.
```

필수 contract:

```text
open_case
create_case
register_source
list_inventory
search_inventory
inventory_facets
get_file_detail
bulk_preview
bulk_commit
validate_media_candidate
generate_proxy
generate_thumbnail
capture_frame
export_clip
record_playback_review
open_original_media_exception
list_derived_artifacts
list_audit_events
add_to_report
remove_from_report
generate_report_package
```

Structured success response:

```json
{
  "ok": true,
  "data": {},
  "warnings": [],
  "audit_event_id": null,
  "job_id": null
}
```

Structured error response:

```json
{
  "ok": false,
  "error": {
    "code": "SOURCE_PATH_OUTPUT_FORBIDDEN",
    "message": "Output path overlaps source evidence path.",
    "details": {}
  },
  "warnings": [],
  "audit_event_id": null,
  "job_id": null
}
```

Acceptance:

```text
WinUI shell has documented engine boundary.
UI does not write directly to SQLite except approved read-only access.
All mutations go through engine command/API.
All mutation commands return audit_event_id or job_id when applicable.
Structured errors are surfaced without hiding failure.
```

---

## 6.2 WinUI shell skeleton

Required areas:

```text
case open/create screen
source registration screen
main forensic workstation layout
inventory pane
media viewer pane
inspector/detail drawer
filters/facets panel
job queue/activity panel
audit trail panel
report set panel
settings/preferences
language toggle KO/EN
```

Korean-first UI:

```text
Default locale: Korean
Toggle: KO / EN
Case data, paths, hashes, parser IDs, vendor names, raw metadata stay verbatim
Locale change must not mutate evidence
Locale change must preserve filters, search, selection, scroll, preview target, grouped tree state
```

---

## 6.3 Inventory integration

Required behavior:

```text
virtualized inventory control
paged/keyset loading
no full 100k+ row load
default sort risk_score desc, timestamp_start asc, file_id asc
engine-backed search
facet counts from engine
composable filters
selection preservation
scroll preservation
grouped tree expansion preservation
bulk select-all-filtered uses predicate selection
```

---

## 6.4 Media review integration

Video review UI:

```text
play
pause
frame-step
previous/next file
speed controls
timeline
current timecode inside viewing area
selected export range
event markers
front/rear or multi-camera synchronized review path
proxy status display
validation status display
frame capture
clip export
```

Rules:

```text
proxy playback is default
proxy missing -> show generate proxy action
proxy queued -> show queue status
proxy failed -> show failure and audit details
original direct playback -> exception flow with reason and audit event
frame capture/clip export must call engine and create derived artifact
```

---

## 6.5 Report / audit workflow

Report UI must support:

```text
add source artifact to report
add derived artifact to report
remove from report
show report flag in inventory
show report set contents
show missing validation warnings
show unsupported/skipped/failed/partial warnings
generate report package
open generated package location
```

Audit panel must show:

```text
event_id
timestamp
operator
action_type
target
parameters
tool name/version
output artifact
previous_event_hash
event_hash
```

---

## 6.6 Windows validation

Validate:

```text
Korean user profile path
Korean case path
Korean evidence path
spaces in paths
very long paths
external drive source path
read-only source path
permission denied source path
locked file
missing source after case reopen
output path canonicalization
path traversal rejection
symlink/junction behavior if supported
ffprobe/ffmpeg path resolution
WebView2 availability if used
DPI scaling
multi-monitor layout
keyboard navigation
case close releases file locks
no orphan ffmpeg/ffprobe/engine process after app close
```

---

## 6.7 Phase 4 Gate

다음을 만족하지 못하면 Phase 5로 가지 마라.

```text
WinUI shell does not become source of truth
all mutations go through engine/API
no direct source evidence modification
inventory is virtualized/paged
100k+/1M path does not load all rows
proxy playback default
direct original playback exception audited
frame capture/clip export create derived artifact
report does not hide failed/skipped/partial/unsupported
Windows Korean/space/long path validation performed or blockers filed
process/file lock cleanup verified
```

Run:

```bash
cargo test
npm test
dotnet test
```

프로젝트에 없는 명령은 skip reason을 기록하라.

---

# 7. Phase 5 — Release Candidate / Field Pilot / Operator Readiness

Phase 4 gate가 통과한 뒤에만 진행하라.

## 7.1 RC scope freeze

Create or update:

```text
docs/rc-scope.md
docs/blocker-register.md
docs/release-readiness-checklist.md
```

`docs/rc-scope.md` must include:

```text
included features
excluded features
unsupported formats
known limitations
platform support
minimum Windows version
required external tools
optional external tools
case size claims
performance claims
validation corpus claims
report wording constraints
```

`docs/blocker-register.md` schema:

```text
blocker_id
severity: P0/P1/P2/P3
area: evidence/gui/media/report/security/performance/migration/install/docs
description
impact
reproduction steps
owner
status
fix commit or evidence
verification command
release decision
```

Rules:

```text
P0/P1 unresolved -> NO-GO
Unsupported feature advertised as supported -> NO-GO
Evidence corruption -> NO-GO
Source path write -> NO-GO
Silent incomplete package -> NO-GO
Disallowed legal wording -> NO-GO
```

---

## 7.2 Installer / package validation

Validate:

```text
fresh install works
upgrade install works or unsupported is documented
uninstall works
portable launch works if supported
app starts without case
app opens existing case
app creates new case
missing dependencies handled
missing ffmpeg/ffprobe handled if optional
WebView2 dependency handled if used
version/build metadata recorded
app does not require admin unless documented
uninstall does not delete case data unless explicitly user-selected
```

Create/update:

```text
docs/install-validation.md
docs/package-manifest.md
docs/uninstall-policy.md
```

Package manifest:

```text
app version
engine version
schema version
build timestamp
git commit
dependency versions
external tool versions or detection policy
```

---

## 7.3 Operator field pilot

Create/update:

```text
docs/operator-pilot-report.md
docs/operator-workflow-checklist.md
```

Pilot workflow:

```text
open/create case
register source image
scan/import/carve
view inventory
filter candidate media
run validation
generate proxy
play proxy
record playback review
capture frame
export clip
mark item for report
bulk preview report action
commit report selection
open audit trail
generate report package
close case
reopen case
verify state persistence
```

For each step record:

```text
operator
timestamp
step
expected result
observed result
audit event id, if applicable
output artifact id, if applicable
issue id, if any
```

---

## 7.4 Accuracy / reproducibility / performance evidence pack

Create/update:

```text
accuracy-report.json
accuracy-report.html
reproducibility-report.json
reproducibility-report.html
performance-report.json
docs/performance-report.md
```

Accuracy evidence:

```text
corpus manifest
source hashes
ground truth
expected recovered items
expected unsupported items
false positives
false negatives
precision where applicable
recall where applicable
known limitations
tool versions
options used
```

Reproducibility evidence:

```text
run same case twice
input hashes
tool versions
engine version
schema version
command options
run 1 output hashes
run 2 output hashes
normalized diff
explained differences
unexpected differences
```

Performance targets:

```text
10,000-row fixture:
initial inventory render P95 <= 2 seconds
search P95 <= 500 ms
scroll P95 frame time <= 32 ms

100,000-row fixture:
initial inventory render P95 <= 2.5 seconds
search P95 <= 1 second
no UI freeze > 2 seconds

1,000,000-row synthetic SQLite fixture:
search P99 <= 3 seconds with engine-backed query
UI memory does not grow with total row count
UI memory target <= 600MB
```

---

## 7.5 Security / privacy / supply-chain review

Create/update:

```text
docs/security-review.md
docs/privacy-review.md
docs/supply-chain-review.md
```

Security areas:

```text
source/output path separation
path traversal
symlink/junction escape
unsafe command execution
ffmpeg/ffprobe argument handling
HTML injection in generated review
report package file inclusion
untrusted metadata rendering
temporary file cleanup
logs exposing sensitive paths
permission handling
dependency vulnerabilities
```

Privacy areas:

```text
local paths in reports
operator identity handling
case metadata exposure
unrelated user data leakage
redaction options
report package contents
logs and diagnostics
crash dumps
telemetry if any
```

Supply-chain areas:

```text
Rust dependencies
npm dependencies
.NET dependencies
external tools
ffmpeg/ffprobe provenance
build reproducibility notes
license obligations
package signing status
hashes of bundled binaries
```

---

## 7.6 Operator docs / known limitations

Create/update:

```text
docs/operator-manual.md
docs/quickstart.md
docs/troubleshooting.md
docs/known-limitations.md
docs/backlog.md
docs/unsupported-features.md
docs/report-defense-checklist.md
docs/release-notes.md
```

Operator manual must include:

```text
case creation
source registration
scan/import/carve
inventory search/filter
validation states
candidate-unvalidated meaning
ffprobe-video-stream-confirmed meaning
playback-confirmed meaning
proxy generation
frame capture
clip export
derived artifact meaning
audit trail review
report package generation
unsupported items
failed/skipped/partial items
known limitations
```

Unsupported feature review examples:

```text
browser artifacts
Windows Event Logs
advanced GPS reconstruction
multi-vendor proprietary codecs
encrypted volumes
damaged filesystem reconstruction
cloud sync artifacts
live acquisition
mobile device extraction
```

Each unsupported/backlog item must have:

```text
feature
status: supported / partial / unsupported / backlog / design candidate
user-visible wording
risk if misunderstood
test coverage
release note wording
```

---

## 7.7 Phase 5 Gate

다음을 만족해야 Phase 6으로 갈 수 있다.

```text
RC scope frozen
blocker register exists
install/upgrade/uninstall policy documented
Windows validation matrix exists
operator pilot completed or blockers filed
accuracy evidence exists or honest blocker filed
reproducibility evidence exists or honest blocker filed
performance evidence exists or honest blocker filed
security/privacy/supply-chain reviews exist
operator manual and known limitations exist
unsupported features are not advertised as supported
report-defense checklist matches actual report output
```

---

# 8. Phase 6 — GA Release / Post-release Operations / Long-term Governance

Phase 5 gate가 통과한 뒤에만 진행하라.

## 8.1 GA readiness review

Create or update:

```text
docs/ga-readiness-review.md
docs/ga-go-no-go.md
```

GA readiness review must include:

```text
release version
release channel
target platform
minimum Windows version
supported case types
supported input/source types
supported media validation capabilities
supported report capabilities
unsupported features
known limitations
open blockers
waived issues, if any
waiver rationale
risk owner
test evidence references
manual QA evidence references
operator pilot evidence references
```

Rules:

```text
P0/P1 cannot be waived for GA.
Evidence corruption cannot be waived.
Source path write cannot be waived.
False confirmed media cannot be waived.
Report hiding failures cannot be waived.
Disallowed legal wording cannot be waived.
100k/1M memory blow-up cannot be waived if large-case support is claimed.
```

---

## 8.2 Release artifact build and manifest

Create or update:

```text
docs/package-manifest.md
docs/release-artifacts.md
release/manifest.json
release/checksums.sha256
```

Manifest must include:

```text
product_name
release_version
release_channel
build_timestamp
git_commit
engine_version
gui_version
schema_version
ULW-loop version if applicable
Rust version
Node/npm version if applicable
.NET SDK/runtime version if applicable
WinUI/runtime version
external tool versions or detection policy
ffmpeg/ffprobe version or optional dependency policy
dependency lockfile hash
artifact file names
artifact sha256 hashes
signing status
known limitations doc path
release notes path
```

Acceptance:

```text
Release artifacts are listed.
Every artifact has SHA-256.
Signing status is explicit.
Version metadata is visible in app or package.
Manifest matches actual files.
No stale artifact is listed.
```

---

## 8.3 Final install / upgrade / uninstall / rollback verification

Create or update:

```text
docs/final-install-validation.md
docs/rollback-guide.md
```

Verify:

```text
fresh install
launch after install
open/create case
upgrade from previous supported version
rollback from failed upgrade
uninstall
reinstall
case data preservation
settings preservation or reset policy
external tool detection
missing dependency handling
non-admin install behavior
admin install behavior, if supported
portable mode, if supported
```

Rollback guide must include:

```text
what can be rolled back
what cannot be rolled back
case DB backup location
schema migration rollback policy
operator steps
known risks
verification after rollback
```

---

## 8.4 Support and triage policy

Create or update:

```text
docs/support-policy.md
docs/triage-policy.md
docs/issue-template.md
docs/operator-support-runbook.md
```

Triage severity:

```text
P0:
evidence corruption
source evidence write
false confirmed media
audit chain corruption
report hides validation failure
security exploit with evidence impact
data loss

P1:
Stop hook stale block recurrence
large-case memory blow-up for claimed supported size
incorrect validation state
missing derived artifact audit
migration failure with recovery path
installer corrupts app or case state

P2:
performance degradation
non-blocking UI failure
report formatting issue not affecting facts
documentation confusion with workaround

P3:
polish
minor wording
non-critical UX improvement
backlog feature request
```

Issue template must ask for:

```text
app version
engine version
schema version
OS version
case size
source type
steps to reproduce
expected result
observed result
logs path
audit event id if applicable
artifact id if applicable
whether source evidence was modified
whether report output was affected
```

---

## 8.5 Hotfix / patch policy

Create or update:

```text
docs/hotfix-policy.md
docs/patch-release-checklist.md
docs/versioning-policy.md
```

Hotfix allowed for:

```text
evidence safety fix
source/output path safety fix
false validation state fix
audit chain fix
report correctness fix
security fix
installer corruption fix
P0/P1 regression fix
```

Hotfix not allowed for:

```text
new parser
new recovery feature
new report claim
new GUI feature unrelated to blocker
unsupported feature expansion
```

Versioning policy:

```text
major: incompatible schema or major workflow change
minor: new supported capability with PRD/test corpus/evidence
patch: bug fix without new supported capability
hotfix: urgent P0/P1 fix
```

---

## 8.6 Incident response plan

Create or update:

```text
docs/incident-response.md
docs/evidence-safety-incident-playbook.md
docs/report-defect-incident-playbook.md
```

Incident categories:

```text
evidence corruption
source path write
false confirmed media
missed evidence due to regression
audit chain corruption
derived artifact provenance missing
report hides failed/skipped/partial/unsupported state
disallowed legal wording in report
privacy leak in report/log/package
security vulnerability
installer/migration data loss
```

Incident response must include:

```text
severity classification
immediate containment
affected versions
affected cases
operator notification
evidence preservation steps
log/audit collection
reproduction steps
fix owner
hotfix criteria
validation criteria
report correction policy
postmortem requirement
```

---

## 8.7 Security / privacy / supply-chain monitoring

Create or update:

```text
docs/security-monitoring.md
docs/dependency-update-policy.md
docs/privacy-operations.md
docs/supply-chain-monitoring.md
```

Security monitoring:

```text
dependency vulnerability review cadence
Rust dependency audit
npm dependency audit
.NET dependency audit
external tool version review
ffmpeg/ffprobe advisory monitoring
unsafe command execution review
generated HTML injection review
path traversal regression review
signing/certificate review if applicable
```

Privacy operations:

```text
what logs contain
what reports contain
how local paths are handled
how operator identity is handled
how crash dumps are handled
telemetry policy
redaction guidance
support bundle contents
```

Supply-chain monitoring:

```text
lockfile policy
dependency update review
external binary provenance
bundled binary hashes
license review
build reproducibility notes
release artifact checksums
```

---

## 8.8 Corpus and regression governance

Create or update:

```text
docs/corpus-governance.md
docs/regression-policy.md
docs/accuracy-validation-policy.md
docs/reproducibility-policy.md
```

Regression tiers:

```text
Tier 0:
unit tests and contract tests

Tier 1:
small synthetic fixtures

Tier 2:
known forensic sample corpus

Tier 3:
large-scale performance fixtures

Tier 4:
operator pilot cases
```

Accuracy policy:

```text
Do not claim accuracy beyond tested corpus.
False positive and false negative counts must be reported.
Unsupported items must be visible.
Ground truth changes require dual review.
```

Reproducibility policy:

```text
same input + same version + same options should produce same durable outputs
allowed differences must be normalized
unexpected differences are investigated
tool version drift must be recorded
```

---

## 8.9 Feature intake governance

Create or update:

```text
docs/feature-intake-policy.md
docs/feature-prd-template.md
docs/feature-test-spec-template.md
docs/feature-release-gate.md
```

Every new feature must have:

```text
feature summary
user/operator need
supported inputs
unsupported inputs
forensic risk
source/output safety analysis
audit requirements
report wording
test corpus
accuracy expectations
false positive risk
false negative risk
performance impact
migration impact
security/privacy impact
operator workflow impact
rollback plan
release gate
```

Feature cannot be marked supported until:

```text
PRD approved
test spec approved
corpus/fixtures available
accuracy validation complete
reproducibility validation complete
security/privacy review complete
operator workflow reviewed
docs updated
known limitations updated
report wording lint updated if needed
```

---

## 8.10 Phase 6 Gate

다음을 만족해야 Phase 7으로 갈 수 있다.

```text
GA readiness review exists
release artifacts have manifest and SHA-256
install/upgrade/uninstall/rollback evidence exists
support/triage policy exists
hotfix/patch/versioning policy exists
incident response plan exists
security/privacy/supply-chain monitoring docs exist
corpus/regression governance exists
feature intake governance exists
operator docs and known limitations are consistent
no P0/P1 unresolved
unsupported features are not advertised as supported
```

---

# 9. Phase 7 — Post-GA Continuous Operations / vNext / External Review Readiness

Phase 6 gate가 통과한 뒤에만 진행하라.

이 단계의 목표는 새 기능을 즉흥적으로 추가하는 것이 아니다.  
목표는 실제 사용자, operator, QA, security, release feedback을 받아도 forensic correctness와 report-defensibility가 무너지지 않게 장기 운영 체계를 고정하는 것이다.

## 9.1 Post-GA monitoring plan

Create or update:

```text
docs/post-ga-monitoring.md
docs/field-feedback-policy.md
docs/support-metrics.md
```

Post-GA monitoring must track:

```text
reported evidence safety issues
reported false confirmed media issues
reported missed validation failures
large-case memory/performance issues
report package defects
audit chain defects
installer/upgrade/uninstall defects
Windows path handling defects
ffmpeg/ffprobe compatibility issues
operator confusion patterns
documentation gaps
unsupported feature confusion
```

Support metrics should include:

```text
issue count by severity
issue count by area
time to triage
time to reproduce
time to fix
hotfix count
rollback count
operator documentation fixes
known limitation updates
```

Privacy rule:

```text
Do not collect case data, paths, hashes, logs, or operator identity automatically unless telemetry/privacy policy explicitly permits it.
Default assumption: no telemetry.
Support bundles must be user-generated and redaction-aware.
```

---

## 9.2 vNext feature governance

Create or update:

```text
docs/vnext-roadmap.md
docs/vnext-feature-gates.md
docs/feature-risk-register.md
```

Every vNext feature must be classified as one of:

```text
supported
partial
unsupported
backlog
design candidate
research
deprecated
removed
```

For each proposed feature, record:

```text
feature name
operator need
forensic risk
evidence safety risk
source/output path risk
audit requirements
report wording impact
accuracy validation requirement
reproducibility requirement
performance impact
migration impact
security/privacy impact
test corpus requirement
release gate
owner
status
```

Features that require strict gate:

```text
new file system parser
new DVR/blackbox vendor parser
new carving method
new timestamp inference method
GPS reconstruction
browser artifact support
Windows Event Log support
encrypted volume handling
cloud artifact handling
mobile device support
live acquisition
AI-assisted classification
automatic report narrative generation
```

Rules:

```text
No vNext feature may be marked supported without PRD.
No vNext feature may be marked supported without test spec.
No vNext feature may be marked supported without corpus/fixture evidence.
No vNext feature may be marked supported without report wording review.
No vNext feature may bypass source/output path safety review.
```

---

## 9.3 Deprecation and removal policy

Create or update:

```text
docs/deprecation-policy.md
docs/removal-policy.md
docs/schema-deprecation-policy.md
```

Deprecation policy must define:

```text
what can be deprecated
how operators are notified
how reports mention deprecated behavior
how long backward compatibility is maintained
migration requirements
rollback requirements
how old cases remain readable
```

Removal policy must define:

```text
removal criteria
required migration evidence
required backup evidence
required rollback evidence
operator warning requirements
release note requirements
test fixture requirements
```

Rules:

```text
Removing a schema column/table requires migration fixture and rollback evidence.
Removing support for a previously supported format requires release note and known limitation update.
Old cases must fail closed, not silently misread.
Deprecated features must not silently change forensic meaning.
```

---

## 9.4 Long-term schema compatibility

Create or update:

```text
docs/schema-compatibility.md
docs/case-db-version-policy.md
docs/migration-lifecycle.md
```

Schema compatibility must cover:

```text
case DB version detection
minimum supported schema version
upgrade path
backup before migration
rollback path
read-only open for old cases
fail-closed behavior for unsupported schema
migration audit event
migration report
```

Required tests:

```text
open current schema
open previous schema
migrate previous schema to current
backup created before migration
rollback works
unsupported future schema fails closed
missing required table fails closed
required hashes/source paths survive migration
audit events survive migration
derived artifacts survive migration
report flags survive migration
```

---

## 9.5 External review readiness

Create or update:

```text
docs/external-review-package.md
docs/independent-review-checklist.md
docs/reviewer-notes-template.md
```

External review package should include:

```text
architecture summary
source of truth summary
evidence safety policy
ULW/Stop hook completion contract
inventory/query contract
media validation state machine
derived artifact contract
audit event schema
report-defensibility policy
known limitations
unsupported features
accuracy corpus summary
reproducibility summary
performance summary
security/privacy/supply-chain summaries
release blocker policy
```

Independent reviewer checklist:

```text
Can PASS be falsely recorded?
Can Stop hook stale state recur?
Can source evidence be modified?
Can candidate appear as confirmed without validation?
Can ffprobe confirmation be confused with playback confirmation?
Can derived artifact be created without audit?
Can report hide failure/unsupported/skipped/partial?
Can large inventory load into memory?
Can unsupported feature appear supported?
Can disallowed legal wording appear?
Can migration corrupt old case?
Can Windows paths corrupt evidence references?
```

---

## 9.6 Training and operator certification materials

Create or update:

```text
docs/operator-training.md
docs/operator-checklist.md
docs/training-scenarios.md
docs/operator-competency-check.md
```

Training must cover:

```text
case creation
source registration
inventory triage
candidate-unvalidated meaning
validation state meaning
ffprobe-video-stream-confirmed vs playback-confirmed
proxy playback
direct original playback exception
frame capture as derived artifact
clip export as derived artifact
audit trail review
bulk preview
report package generation
failed/skipped/partial/unsupported handling
known limitations
privacy/redaction practices
support bundle creation
```

Training scenarios:

```text
small case review
large case search/filter
corrupted media candidate
duplicate candidate
unsupported format
failed validation
proxy generation failure
frame capture
clip export
report package with known limitations
case reopen and state persistence
```

---

## 9.7 Long-term regression schedule

Create or update:

```text
docs/regression-schedule.md
docs/release-test-matrix.md
```

Regression schedule:

```text
per commit:
unit tests
contract tests
lint where available

per PR:
targeted integration tests
report wording lint
source/output path safety tests

nightly:
larger corpus tests
reproducibility sample
large inventory synthetic test

per release candidate:
full corpus validation
full reproducibility validation
full performance tiers
installer validation
Windows path matrix
operator workflow pilot

post-hotfix:
P0/P1 reproduction test
regression test for fixed issue
minimal release package validation
```

---

## 9.8 Phase 7 Gate

Phase 7 is PASS only if:

```text
post-GA monitoring policy exists
field feedback policy exists
vNext feature gate exists
deprecation/removal policy exists
schema compatibility policy exists
external review package exists
operator training material exists
regression schedule exists
no unsupported feature is represented as supported
no telemetry/privacy behavior is implied without policy
no P0/P1 issue remains unregistered
```

If these are missing, final status is:

```text
PARTIALLY READY
```

not GA-long-term-ready.

---

# 10. Phase 8 — Final Verification / Cleanup / GO-NO-GO

Run relevant commands.

Minimum:

```bash
cargo test --test cli_inventory -- --nocapture
cargo test
npm test
dotnet test
```

If available:

```bash
npm run test:e2e
npm run build
dotnet build
dotnet publish
```

Manual QA is acceptable only where automation is not feasible.  
Manual QA must include exact steps, observed results, and evidence paths.

Required manual QA if not automated:

```text
fresh install
launch app
open/create case
register source
run inventory
search/filter/sort
open candidate
run validation
generate proxy
play proxy
frame capture
clip export
bulk preview
add to report
generate report package
view audit event
close case
reopen case
verify state persistence
uninstall
verify case data preservation
verify no lingering process/file lock
```

Archive release evidence.

Create/update:

```text
release/evidence/
release/evidence/test-results/
release/evidence/manual-qa/
release/evidence/checksums/
release/evidence/reports/
release/evidence/security/
release/evidence/performance/
release/evidence/operator-pilot/
release/evidence/go-no-go/
```

Archive must include:

```text
test command outputs
manual QA evidence
performance results
accuracy report
reproducibility report
security/privacy/supply-chain review
installer validation
Windows validation
operator pilot report
release manifest
checksums
GO/NO-GO decision
cleanup receipt
```

---

# 11. Cleanup Receipt

If only short-lived commands were used:

```json
{
  "status": "not-applicable",
  "reason": "implementation and verification used short-lived build/test/package/documentation subprocesses only; no server, browser, worker, tmux session, container, bound port, app instance, or persistent runtime was left running"
}
```

If any app, browser, local server, queue worker, watcher, ffmpeg/ffprobe, engine subprocess, installer, or background process was started:

```json
{
  "status": "done",
  "receiptPath": "<path>",
  "checkedAt": "<timestamp>",
  "noRemainingProcesses": true,
  "noOpenBrowsers": true,
  "noWorkers": true
}
```

Do not claim cleanup:not-applicable if any persistent process was started.

---

# 12. Global Release Blockers

No release may bypass these blockers.

```text
Technical Review
Security Review
Privacy Review
Supply-chain Review
Accuracy Validation
Reproducibility Validation
Performance Validation
Migration Validation
Operator Review
Report-defensibility Review
Legal wording Review
Installer/Package Validation
Windows Workstation Validation
Known Limitations Review
Release Notes Review
Support/Triage Policy
Hotfix Policy
Incident Response Plan
Corpus Governance
Feature Intake Governance
Post-GA Monitoring
External Review Readiness
Regression Schedule
```

Blocking rules:

```text
Any unchecked blocker means NO-GO.
Any unresolved P0/P1 issue means NO-GO.
Any missing migration fixture for a schema change means NO-GO.
Any unsupported feature advertised as supported means NO-GO.
Any disallowed legal wording means NO-GO.
Any silent incomplete package behavior means NO-GO.
Any evidence-corrupting behavior means NO-GO.
Any source evidence path write means NO-GO.
Any PASS evidence without structured proof and cleanup contract means NO-GO.
Any resume/compaction path that bypasses status --json canonical paths means NO-GO.
Any Stop hook stale active state recurrence means NO-GO.
Any 100k/1M GUI path that loads full inventory into memory means NO-GO.
Any carved candidate shown as confirmed media without validation evidence means NO-GO.
Any ffprobe-video-stream-confirmed shown as playback-confirmed without playback review means NO-GO.
Any frame capture or clip export without derived artifact and audit record means NO-GO.
Any report hiding failed/skipped/partial/unsupported state means NO-GO.
Any direct original playback without audit exception means NO-GO.
Any lingering ffmpeg/ffprobe/engine process after cleanup means NO-GO.
Any Windows path handling that corrupts or loses evidence paths means NO-GO.
Any missing package manifest or release checksums means NO-GO.
Any missing incident response plan means NO-GO.
Any missing support/triage policy means NO-GO.
Any unsupported feature represented as supported in roadmap/docs/reports means NO-GO.
Any telemetry/privacy behavior implied without policy means NO-GO.
```

---

# 13. Required Final Response Format

작업 완료 후 반드시 다음 형식으로 보고하라.

```md
# Implementation Report — FrameTrace Full Production / GA / Long-term Operations Readiness

## Summary

- 변경한 것
- 의도적으로 변경하지 않은 것
- 남은 제한사항

## Phase 0 — Inspection

- files inspected
- source of truth found
- Stop hook state path found
- GUI/data/query state found
- media/audit/report state found
- Windows/release/package state found
- support/incident/governance state found
- post-GA/vNext/external review state found
- gaps found

## Phase 1 — ULW-LOOP / Stop Hook / Evidence

### Changes

- PASS evidence validator
- cleanup receipt enforcement
- checkpoint validation
- status --json canonical path
- resume/compaction source-of-truth fix
- reconcile-hook-state command
- Stop hook diagnostics
- quality gate LIGHT/HEAVY behavior
- bootstrap fallback behavior

### Tests

- evidence tests
- checkpoint tests
- status/resume tests
- reconcile tests
- Stop hook tests
- quality gate tests
- bootstrap tests

### Gate Result

- PASS / FAIL
- blockers

## Phase 2 — GUI Inventory / SQLite Query / HTML Prototype

### Changes

- production inventory data contract
- list/search/facet/detail/bulk API contract
- keyset pagination
- SQLite/index/query-plan work
- bulk preview
- audit event contract
- HTML prototype
- localization
- generated review HTML security
- performance notes

### Tests

- inventory row contract
- default sort stability
- page_token
- search/facet
- bulk_preview
- virtualized 10k rows
- state preservation
- HTML escaping

### Gate Result

- PASS / FAIL
- blockers

## Phase 3 — Media Validation / Derived Artifacts / Audit / Report

### Changes

- media state machine
- validation records
- ffprobe/ffmpeg wrapper behavior
- derived artifact records
- queue/job records
- source/output path safety
- audit chain
- report-defensibility
- generated review integration

### Tests

- state transitions
- validation behavior
- derived artifact behavior
- path safety
- audit chain
- report wording
- generated HTML security

### Gate Result

- PASS / FAIL
- blockers

## Phase 4 — WinUI 3 / Windows Integration

### Changes

- engine boundary
- shell skeleton
- inventory integration
- media review integration
- report/audit workflow
- Windows path validation
- process/file lock cleanup

### Tests

- Rust tests
- .NET/WinUI tests
- E2E/manual QA tests
- Windows path safety tests

### Gate Result

- PASS / FAIL
- blockers

## Phase 5 — RC / Field Pilot / Operator Readiness

### Changes

- RC scope
- blocker register
- installer/package validation
- Windows validation matrix
- operator pilot
- accuracy evidence
- reproducibility evidence
- performance evidence
- security/privacy/supply-chain review
- operator docs
- known limitations
- unsupported feature review

### Gate Result

- PASS / FAIL
- blockers

## Phase 6 — GA / Post-release Operations / Governance

### Changes

- GA readiness review
- release artifacts
- manifest/checksums
- final install/upgrade/uninstall/rollback evidence
- support/triage policy
- hotfix/patch/versioning policy
- incident response plan
- security/privacy/supply-chain monitoring
- corpus/regression governance
- feature intake governance

### Gate Result

- PASS / FAIL
- blockers

## Phase 7 — Post-GA Operations / vNext / External Review

### Changes

- post-GA monitoring
- field feedback policy
- support metrics
- vNext feature gates
- feature risk register
- deprecation/removal policy
- schema compatibility policy
- external review package
- operator training material
- regression schedule

### Gate Result

- PASS / FAIL
- blockers

### Remaining Long-term Risks

- risk
- owner
- mitigation

## Files Changed

- file path
- reason

## Verification Evidence

Include exact commands and results.

Example:

```bash
cargo test --test cli_inventory -- --nocapture
cargo test
npm test
dotnet test
```

## Manual QA Evidence

Include exact steps and observed results for non-automated flows.

## Release Evidence Archive

- test results path
- manual QA path
- performance evidence path
- accuracy evidence path
- reproducibility evidence path
- security/privacy/supply-chain path
- manifest/checksum path
- GO/NO-GO decision path

## Cleanup Receipt

Include structured cleanup receipt.

Example:

```json
{
  "status": "not-applicable",
  "reason": "implementation and verification used short-lived build/test/package/documentation subprocesses only; no server, browser, worker, tmux session, container, bound port, app instance, or persistent runtime was left running"
}
```

If persistent processes were started, include actual cleanup receipt with:

```json
{
  "status": "done",
  "receiptPath": "<path>",
  "checkedAt": "<timestamp>",
  "noRemainingProcesses": true,
  "noOpenBrowsers": true,
  "noWorkers": true
}
```

## Final Recommendation

Choose one:

- GA GO
- GA NO-GO
- PARTIALLY READY
- GA READY BUT LONG-TERM OPS INCOMPLETE

Do not claim GA GO if any P0/P1 issue remains.
Do not claim GA READY BUT LONG-TERM OPS INCOMPLETE if release safety itself is incomplete.
Use GA READY BUT LONG-TERM OPS INCOMPLETE only when the product is release-safe but long-term governance docs are not fully complete.
```

중요: phase gate가 실패하면 다음 phase로 넘어가지 마라.  
중요: 테스트 실패를 숨기지 마라.  
중요: mock prototype을 production forensic validation처럼 표현하지 마라.  
중요: validation이 없는 carved file을 confirmed media로 표시하지 마라.  
중요: ffprobe 성공을 playback review 완료로 취급하지 마라.  
중요: derived artifact 없이 frame capture/clip export 파일만 생성하지 마라.  
중요: 보고서에서 실패, unsupported, skipped, partial을 숨기지 마라.  
중요: unsupported를 supported처럼 보이게 하지 마라.  
중요: 파일럿에서 나온 P0/P1은 반드시 NO-GO blocker로 올려라.  
중요: GA 단계에서는 기능을 늘리는 것이 아니라 배포 가능성과 운영 가능성을 증거로 고정하라.  
중요: Post-GA 단계에서는 새 기능을 넣는 것이 아니라 장기 운영, vNext gate, deprecation, external review readiness를 고정하라.
