/* FrameTrace Evidence Viewer — serverless single-file review UI.
 * Data arrives via window.__FRAMETRACE_DATA__ (inlined by frametrace make-review).
 * Reviewer state (layout, marks, selection) persists in localStorage per case. */

(async function () {
const DATA = await loadViewerData();
const manifest = DATA.manifest || {};
const scan = DATA.scan || {};
const carveLog = Array.isArray(DATA.carveLog) ? DATA.carveLog : [];
const filesystemLog = Array.isArray(DATA.filesystemLog) ? DATA.filesystemLog : [];
const validationLog = Array.isArray(DATA.validationLog) ? DATA.validationLog : [];
const anomalyLog = Array.isArray(DATA.anomalyLog) ? DATA.anomalyLog : [];
const anomalyFindings = anomalyLog.filter(item => item && item.kind && item.kind !== "none");
const anomaliesBySelector = new Map();
for (const item of anomalyFindings) {
  const key = String(item.selector || "");
  if (!key) continue;
  const list = anomaliesBySelector.get(key) || [];
  list.push(item);
  anomaliesBySelector.set(key, list);
}
const BS = String.fromCharCode(92);
const EXT_PREFIX = BS + BS + "?" + BS;
const EXT_UNC = EXT_PREFIX + "unc" + BS;
const LAYOUT_KEY = "ft.viewer." + (manifest.case_id || "case") + ".layout.v2";
const MARKS_KEY = "ft.viewer." + (manifest.case_id || "case") + ".marks";
const NOTES_KEY = "ft.viewer." + (manifest.case_id || "case") + ".notes";
const ANNOTATIONS_KEY = "ft.viewer." + (manifest.case_id || "case") + ".annotations.v1";
const RANGES_KEY = "ft.viewer." + (manifest.case_id || "case") + ".ranges";
const PROXIES_KEY = "ft.viewer." + (manifest.case_id || "case") + ".proxies";
const EXAMINER_KEY = "ft.viewer." + (manifest.case_id || "case") + ".examiner";
const MARK_STATUSES = ["reviewed", "important", "needs_verification"];
// Tag presets are the quick-apply taxonomy for the tag menu, the tag
// filter, the detail editor, and the popup player. The defaults cover
// any case type — accident, assault, fraud, missing-person, digital
// evidence — with the traffic set kept at the end for dashcam cases.
// Examiners can add/remove presets in the tag menu; the list is stored
// globally (not per case) because an investigator's tag vocabulary is
// personal, while applied tags stay per-case under TAGS_KEY.
const DEFAULT_TAG_PRESETS = ["핵심증거", "현장", "관련인", "동선·위치", "시간대확인", "조작의심", "출처불명", "참고자료", "보존요청", "제외", "사고", "과속", "신호위반", "차선변경", "보행자", "음주의심"];
const TAG_PRESETS_KEY = "ft.viewer.tag_presets";
const TAGS_KEY = "ft.viewer." + (manifest.case_id || "case") + ".tags";
const LOCALE_KEY = "ft.viewer." + (manifest.case_id || "case") + ".locale";

const I18N = {
  ko: {
    "unit.count": "건",
    "group.collapsed": "접힘",
    "grid.empty": "일치하는 증거가 없습니다.",
    "aria.select": "선택",
    "time.unknown": "시각 미상",
    "thumb.missing": "썸네일 없음",
    "thumb.damaged": "손상",
    "lang.switch": "언어 전환",
    "header.verified": "검증됨",
    "header.candidate": "후보",
    "header.failed": "실패",
    "header.anomaly": "이상 후보",
    "btn.theater": "시어터",
    "btn.fullscreen": "전체화면",
    "btn.pip": "PiP",
    "btn.clearDates": "기간 해제",
    "search.placeholder": "경로, ID, 파서, 해시 검색",
    "page.prev": "이전",
    "page.next": "다음",
    "filter.kind": "출처",
    "filter.kind.all": "전체 출처",
    "filter.kind.video": "원본 (논리 파일)",
    "filter.kind.carved": "카빙 후보",
    "filter.kind.filesystem": "파일시스템 복구",
    "filter.kind.candidate": "삭제 영상 후보 (복구 전)",
    "filter.status": "검증 상태",
    "filter.status.all": "전체 검증 상태",
    "filter.sort": "정렬",
    "filter.sort.id": "기본 (ID순)",
    "filter.sort.timeDesc": "시간 최신순",
    "filter.sort.timeAsc": "시간 오래된순",
    "filter.sort.name": "이름순",
    "filter.sort.sizeDesc": "크기순",
    "filter.group": "분류/그룹화",
    "filter.group.none": "그룹화 없음",
    "filter.group.day": "녹화 일자별",
    "filter.group.kind": "출처별",
    "filter.group.recType": "녹화 유형별 (주행/충격/주차)",
    "filter.group.status": "검증 상태별",
    "filter.group.mark": "판독 마크별",
    "filter.group.prefix": "이름 접두사별",
    "filter.group.channel": "채널별 (전/후방)",
    "filter.page.100": "100개씩",
    "filter.page.250": "250개씩",
    "filter.page.500": "500개씩",
    "filter.page.1000": "1000개씩",
    "filter.dateFrom": "기간 시작",
    "filter.dateTo": "기간 끝",
    "hist.title": "일자별 건수 — 클릭하면 그날로 필터",
    "split.v": "드래그: 목록/뷰어 폭 조절 / 더블클릭: 기본값",
    "split.h": "드래그: 영상 높이 조절 / 더블클릭: 기본값",
    "media.rate": "재생 속도 ([ / ])",
    "media.zoom": "확대/축소",
    "sc.next": "다음 / 이전 증거",
    "sc.toggle": "현재 증거 선택 토글",
    "sc.preview": "현재 증거 미리보기",
    "sc.mark": "현재 증거 판독 완료 / 중요 / 검증 대기 (다음으로 이동)",
    "sc.unmark": "현재 증거 마크 해제",
    "sc.selAll": "필터 결과 전체 선택",
    "sc.clear": "선택 해제",
    "sc.seek": "영상 10초 뒤로 / 앞으로",
    "sc.rate": "재생 속도 낮추기 / 높이기",
    "sc.range": "구간 IN / OUT 지정",
    "sc.fs": "영상 전체화면",
    "sc.theater": "시어터 모드",
    "sc.pip": "화면 속 화면",
    "sc.exit": "모드 종료",
    "sc.rangeSel": "범위 선택",
    "media.size": "크기",
    "media.fit": "맞춤",
    "panel.selected": "선택 증거",
    "menu.view": "보기 ▾",
    "menu.tag": "태그 ▾",
    "menu.export": "보내기·복사 ▾",
    "menu.filters": "정렬·표시 ▾",
    "menu.timeline": "타임라인",
    "timeline.title": "케이스 타임라인",
    "timeline.gen": "타임라인 생성/갱신",
    "btn.close": "닫기",
    "toolbar.skipBack": "« -10초",
    "toolbar.skipFwd": "+10초 »",
    "toolbar.popPlayer": "⧉ 영상 새 창",
    "toolbar.download": "⤓ 저장",
    "toolbar.capture": "프레임 캡처",
    "toolbar.in": "[ IN",
    "toolbar.out": "OUT ]",
    "toolbar.clip": "구간보내기",
    "toolbar.clipBurn": "법원용 보내기",
    "toolbar.dual": "듀얼 재생",
    "toast.noPair": "페어링된 채널 영상이 없습니다 — _F/_R 또는 전방/후방이 포함된 같은 이름 파일이 필요합니다.",
    "panel.telemetry": "주행 궤적",
    "btn.telemetry": "추출",
    "telemetry.summary": "{n}개 궤적점 (상대 개략도 — 지도 아님)",
    "telemetry.maxSpeed": "최고 {v}km/h",
    "telemetry.noPoints": "추출된 궤적점 없음",
    "toast.telemetryDone": "텔레메트리 {n}개 궤적점 추출",
    "toast.telemetryFail": "텔레메트리 추출 실패: {err}",
    "toast.telemetryOffline": "텔레메트리 추출은 워크스테이션 서버에서만 가능합니다.",
    "export.transcode": "미재생 포맷 프록시 큐",
    "toast.queueRunning": "트랜스코딩 큐 실행 중 — 파일 수에 따라 오래 걸릴 수 있습니다.",
    "toast.queueDone": "큐 완료: 후보 {t} · 변환 {p} · 기존 {c} · 실패 {f}",
    "toast.queueFail": "프록시 큐 실패: {err}",
    "toast.queueOffline": "프록시 큐는 워크스테이션 서버에서만 가능합니다.",
    "clip.exhibitPrompt": "화면에 새길 호증 표시를 입력하십시오 (예: 갑 제3호증). 비워두면 케이스ID·해시·타임코드만 새겨집니다.",
    "clip.exhibitDefault": "갑 제1호증",
    "toolbar.proxy": "프록시 재생",
    "panel.inspector": "포렌식 인스펙터",
    "panel.validation": "검증 로그",
    "panel.source": "원본",
    "panel.source.desc": "수정하지 않음",
    "panel.recovered": "복구물",
    "panel.recovered.desc": "검증 전까지 후보",
    "sel.selectAll": "전체 선택",
    "sel.clear": "해제",
    "sel.groupMark": "판독",
    "mark.reviewed": "판독 완료",
    "mark.important": "중요",
    "mark.verify": "검증 대기",
    "mark.clear": "해제",
    "sel.examiner": "검토자",
    "sel.apply": "케이스에 반영",
    "export.files": "선택 파일 다운로드",
    "export.selected": "선별 자료 묶기",
    "export.csv": "선별 CSV",
    "export.summary": "요약 리포트",
    "export.marks": "마크 내려받기",
    "export.list": "선택 목록 다운로드",
    "export.copyIds": "ID 복사",
    "export.copyPaths": "경로 복사",
    "filter.tag.all": "태그: 전체",
    "shortcuts.title": "단축키",
    "fs.exit": "✕ 닫기 — 워크스테이션으로",
    "chip.groupKind": "출처별 묶기",
    "grid.label": "증거 목록",
    "rectype.driving": "일반 (주행)",
    "rectype.event": "충격 (이벤트)",
    "rectype.parking": "주차",
    "rectype.unclassified": "미분류",
    "chan.front": "전방(F)",
    "chan.rear": "후방(R)",
    "chan.interior": "내부(I)",
    "chan.rear2": "후방2(B)",
    "misc.other": "기타",
    "status.verified": "검증됨",
    "status.confirmed": "ffprobe 확인",
    "status.failed": "검증 실패",
    "status.candidate": "미검증 후보",
    "status.duplicate": "중복 후보",
    "mark.noted": "메모",
    "mark.cleared": "마크 해제",
    "chip.all": "전체",
    "chip.recovery": "삭제·복구 항목",
    "chip.marked": "마크/태그된 항목",
    "chip.anomaly": "이상 징후 후보",
    "chip.warning": "경고 있음",
    "chip.failed": "검증 실패",
    "chip.candidate": "미검증 후보",
    "chip.duplicate": "중복 후보",
    "chip.markImportant": "중요 마크",
    "chip.markReviewed": "판독 완료",
    "chip.markNone": "판독 대기",
    "filter.label": "필터",
    "group.noTime": "시각 미상",
    "group.noMark": "마크 없음",
    "group.noChannel": "채널 미상",
    "tree.all": "전체",
    "tree.rectype": "녹화 유형",
    "tree.kind": "출처",
    "tree.date": "날짜",
    "kind.short.video": "원본",
    "kind.short.carved": "카빙",
    "kind.short.filesystem": "복구",
    "kind.short.candidate": "복구 전",
    "metric.indexed": "{n}편 색인",
    "sel.count": "{n}개 선택 · 마크 {m}",
    "triage.line": "· 판독 {done}/{total} 완료 (중요 {imp} · 검증 대기 {pend})",
    "warn.count": "경고 {n}",
    "warn.title": "경고",
    "dfl.badge": "합성의심 {band}",
    "dfl.title": "deepfake-lens 스크리닝 {score}점 — 검토 우선순위, 판정 아님",
    "dfl.fail": "스크리닝 실패",
    "dfl.failTitle": "deepfake-lens 스크리닝 실패: {error}",
    "dfl.dist": "합성의심 스크리닝 {n}/{total}건 — 높음 {high} · 주의 {medium} · 낮음 {low} · 판단어려움 {unknown} · 실패 {failed} · 미실시 {none}",
    "hist.item": "{day} {count}건",
    "hist.empty": "시각 정보가 있는 증거가 없습니다 — 파일명 패턴 또는 수정시각에서 추출합니다.",
    "hist.range": "녹화 기간: {from} ~ {to} · 시각 확인 {known}/{total}건 (파일명·수정시각 추출)",
    "hist.none": "녹화 시각을 추출한 증거가 없습니다 (파일명 패턴 또는 수정시각 필요)",
    "detail.recTime": "촬영 시각",
    "detail.est": "(추정)",
    "detail.channel": "채널",
    "detail.mark": "판독",
    "detail.unmarked": "미판독",
    "detail.len": "길이",
    "detail.size": "크기",
    "detail.anomaly": "이상 후보",
    "detail.dfl": "합성의심",
    "detail.dflScore": "{label} ({score}점 — 검토 우선순위)",
    "detail.dflFail": "스크리닝 실패: {error}",
    "detail.dflNone": "결과 없음",
    "detail.original": "원본",
    "detail.origPath": "원본 경로",
    "detail.path": "저장 경로",
    "detail.codec": "코덱",
    "detail.offset": "오프셋",
    "detail.unknown": "미상",
    "tag.input": "직접 입력",
    "tag.apply": "적용",
    "tag.presetSave": "프리셋 등록",
    "tag.presetSave.title": "입력한 태그를 프리셋 목록에도 등록 (태그 메뉴·필터에 표시)",
    "note.placeholder": "검토 메모 — 이 증거에 대한 소견 (마크/태그와 함께 케이스에 반영됨)",
    "validation.none": "검증 로그 없음",
    "toast.selectTarget": "대상 증거를 먼저 선택하세요.",
    "toast.selectFirst": "먼저 증거를 선택하세요.",
    "toast.storageFail": "브라우저 저장소에 저장할 수 없습니다: {err}",
    "toast.selectedAll": "필터 결과 {n}개를 선택했습니다.",
    "toast.markApplied": "{n}개 증거에 '{status}'를 적용했습니다.",
    "toast.tagApplied": "{n}개 증거에 '{tag}' 태그를 적용했습니다.",
    "toast.tagCleared": "{n}개 증거의 태그를 해제했습니다.",
    "toast.presetAdded": "'{tag}' 태그를 프리셋에 추가했습니다.",
    "toast.downloadStart": "{name} 다운로드를 시작했습니다.",
    "toast.copied": "{label}을(를) 클릭보드에 복사했습니다.",
    "copy.ids": "증거 ID",
    "copy.paths": "증거 경로",
    "toast.copyFail": "복사에 실패했습니다: {err}",
    "toast.fsExitFail": "전체화면 종료 실패: {err}",
    "toast.fsEnterFail": "전체화면 진입 실패: {err}",
    "toast.fsUnsupported": "이 브라우저는 전체화면을 지원하지 않습니다.",
    "toast.noVideo": "재생 중인 영상이 없습니다.",
    "toast.pipUnsupported": "이 브라우저는 화면 속 화면을 지원하지 않습니다.",
    "toast.pipFail": "화면 속 화면 실패: {err}",
    "toast.noRecord": "표시할 증거가 없습니다.",
    "toast.popupBlocked": "팝업이 차단되었습니다 — 브라우저 설정에서 팝업을 허용하십시오.",
    "toast.captureSaved": "프레임 저장: {path}",
    "toast.captureFail": "캡처 실패: {err}",
    "toast.captureOffline": "캡처 실패: 워크스테이션 서버에 연결할 수 없습니다.",
    "toast.pickEvidence": "재생 중인 증거를 먼저 선택하세요.",
    "toast.outBeforeIn": "OUT 지점이 IN보다 앞입니다 — 다시 지정하세요.",
    "toast.playPick": "재생할 증거를 먼저 선택하세요.",
    "toast.playOriginal": "원본 영상으로 재생합니다.",
    "toast.proxyBuilding": "프록시 생성 중… 원본 크기에 따라 수 분 걸릴 수 있습니다.",
    "toast.proxyPlaying": "프록시로 재생합니다 — 다시 누르면 원본으로 돌아갑니다.",
    "toast.proxyFail": "프록시 실패: {err}",
    "toast.proxyOffline": "프록시 실패: 워크스테이션 서버에 연결할 수 없습니다.",
    "toast.noRange": "먼저 IN/OUT 구간을 지정하세요 (i / o 키).",
    "toast.clipDone": "클립보내기 완료: {path}",
    "toast.clipFail": "보내기 실패: {err}",
    "toast.clipOffline": "보내기 실패: 워크스테이션 서버에 연결할 수 없습니다.",
    "toast.noChanges": "반영할 판독 변경이 없습니다.",
    "toast.applyDone": "판독 마크가 케이스에 반영되고 보고서가 갱신되었습니다.",
    "toast.applyFail": "반영 실패: {err}",
    "toast.applyOffline": "반영 실패: 서버에 연결할 수 없습니다.",
    "toast.noItems": "선택하거나 마크된 항목이 없습니다.",
    "toast.exportFail": "보내기 실패: {err}",
    "toast.exportDone": "선별 자료 {copied}개 복사 완료 (제외 {skipped}) — {dir}",
    "toast.offline": "워크스테이션 서버에 연결할 수 없습니다 — 서버 주소로 뷰어를 여십시오.",
    "toast.fileOnly": "파일 다운로드는 서버로 연 뷰어에서만 지원됩니다 — 워크스테이션에서 뷰어를 여십시오.",
    "toast.downloads": "{n}개 파일 다운로드 시작 — 브라우저가 다중 다운로드 허용을 물을 수 있습니다.",
    "toast.standalone": "파일로 직접 연 뷰어입니다 — 영상 재생·자료 묶기·마크 반영은 서버(127.0.0.1:8477)로 연 뷰어에서 가능하고, 여기서 단 마크/태그는 서버 뷰어와 별도로 저장됩니다.",
    "timeline.offline": "파일로 직접 연 뷰어에서는 타임라인을 불러올 수 없습니다 — 서버 뷰어를 사용하세요.",
    "tagmenu.delPreset": "프리셋에서 삭제",
    "tagmenu.clear": "태그 해제",
    "tagmenu.new": "새 태그 등록",
    "tagmenu.add": "추가",
    "filter.tag.named": "태그: {tag}",
    "report.title": "FrameTrace 판독 요약",
    "report.meta": "케이스: {case} · 생성: {when} · 항목 {n}개",
    "report.counts": "검증됨 {v} · 후보 {c} · 검증 실패 {f}",
    "report.note": "이 문서는 검토자가 선별한 항목의 요약입니다. 원본 증거 무결성은 케이스 감사 로그와 SHA-256 값으로 대조하십시오. '후보' 표시 항목은 검증 전이므로 증거로 주장하기 전 추가 검증이 필요합니다.",
    "report.th.name": "파일명",
    "report.th.kind": "출처",
    "report.th.status": "검증 상태",
    "report.th.mark": "마크",
    "report.th.tags": "태그",
    "report.th.size": "크기",
    "report.th.time": "녹화 시각",
    "report.th.warn": "경고/메모",
    "report.th.orig": "원본 경로",
    "report.memo": "소견:",
    "player.novid": "재생 가능한 파일이 없는 항목입니다 (복구 전 후보 등) — 다음으로 넘어가세요",
    "player.list": "증거 목록",
    "player.prev": "‹ 이전",
    "player.next": "다음 ›",
    "player.play": "재생/일시정지",
    "player.capTitle": "현재 프레임을 케이스에 저장",
    "player.hint": "←/→ ±10초 · ↑/↓ 배속 · space 재생 · k/j 이전/다음",
    "player.mark": "판독",
    "player.tags": "태그",
    "player.reviewed": "판독 완료(1)",
    "player.important": "중요(2)",
    "player.verify": "검증 대기(3)",
    "player.clear": "해제(0)",
    "player.dead": "메인 뷰어가 닫혔습니다 — 마크/태그/이동이 비활성화되었습니다.",
    "player.none": "표시할 증거가 없습니다",
    "player.last": "마지막 항목입니다",
    "player.first": "첫 항목입니다",
    "player.noVideo": "재생 중인 영상이 없습니다",
    "player.saved": "프레임 저장: {path}",
    "player.capFail": "캡처 실패: {err}",
    "player.capOffline": "캡처 실패: 서버 연결 불가",
    "player.warn": "경고 {n}",
    "player.meta": "{id} · {status} · {idx}/{total}건",
    "empty.index": "색인된 증거가 없습니다.",
    "empty.play": "직접 재생 가능한 파일 URL이 없습니다.",
    "range.label": "구간 {in} ~ {out}",
    "cand.vendor": "삭제 영상 후보",
    "cand.note": "복구 전 삭제 영상 후보 — recover-batch로 복구한 뒤 검증하십시오.",
    "player.cap": "프레임 캡처",
    "player.warnShort": "경고",
    "player.savedPfx": "프레임 저장:",
    "player.capFailPfx": "캡처 실패:",
    "warn.inspect": "조사 경고 {n}건",
    "timeline.building": "타임라인 생성 중…",
    "timeline.fail": "생성 실패: {err}",
    "timeline.failOffline": "생성 실패: 서버 연결 불가",
    "timeline.none": "타임라인이 아직 없습니다 — '타임라인 생성/갱신'을 누르세요.",
    "timeline.count": "{n}개 이벤트 · 후보급 (기록된 메타데이터 기반)",
    "timeline.ellipsis": "… 나머지 {n}건 생략 (전체: db/timeline.jsonl)",
    "timeline.unreadable": "타임라인을 읽을 수 없습니다.",
    "offline.openServer": "워크스테이션 서버(127.0.0.1:8477)로 뷰어를 열어야 사용할 수 있습니다.",
    "offline.applyTitle": "서버 경유 뷰어에서만 케이스에 직접 반영할 수 있습니다 — '마크 내려받기'로 파일을 저장하세요.",
    "offline.serverOnly": "워크스테이션 서버로 연 뷰어에서만 사용할 수 있습니다."
  },
  en: {
    "unit.count": "",
    "group.collapsed": "collapsed",
    "grid.empty": "No matching evidence.",
    "aria.select": "Select",
    "time.unknown": "Unknown time",
    "thumb.missing": "No thumbnail",
    "thumb.damaged": "Damaged",
    "lang.switch": "Switch language",
    "header.verified": "Verified",
    "header.candidate": "Candidate",
    "header.failed": "Failed",
    "header.anomaly": "Anomaly candidates",
    "btn.theater": "Theater",
    "btn.fullscreen": "Fullscreen",
    "btn.pip": "PiP",
    "btn.clearDates": "Clear dates",
    "search.placeholder": "Search path, ID, parser, hash",
    "page.prev": "Prev",
    "page.next": "Next",
    "filter.kind": "Source",
    "filter.kind.all": "All sources",
    "filter.kind.video": "Original (logical file)",
    "filter.kind.carved": "Carved candidate",
    "filter.kind.filesystem": "Filesystem recovery",
    "filter.kind.candidate": "Deleted video candidate (pre-recover)",
    "filter.status": "Validation status",
    "filter.status.all": "All validation statuses",
    "filter.sort": "Sort",
    "filter.sort.id": "Default (by ID)",
    "filter.sort.timeDesc": "Newest first",
    "filter.sort.timeAsc": "Oldest first",
    "filter.sort.name": "Name",
    "filter.sort.sizeDesc": "Largest first",
    "filter.group": "Group by",
    "filter.group.none": "No grouping",
    "filter.group.day": "By recording day",
    "filter.group.kind": "By source",
    "filter.group.recType": "By recording type",
    "filter.group.status": "By validation status",
    "filter.group.mark": "By review mark",
    "filter.group.prefix": "By name prefix",
    "filter.group.channel": "By channel",
    "filter.page.100": "100 / page",
    "filter.page.250": "250 / page",
    "filter.page.500": "500 / page",
    "filter.page.1000": "1000 / page",
    "filter.dateFrom": "Date from",
    "filter.dateTo": "Date to",
    "hist.title": "Counts by day — click to filter",
    "split.v": "Drag: adjust list/viewer width / double-click: reset",
    "split.h": "Drag: adjust video height / double-click: reset",
    "media.rate": "Playback speed ([ / ])",
    "media.zoom": "Zoom",
    "sc.next": "Next / previous evidence",
    "sc.toggle": "Toggle current evidence selection",
    "sc.preview": "Preview current evidence",
    "sc.mark": "Mark current: reviewed / important / needs verification (advances)",
    "sc.unmark": "Unmark current evidence",
    "sc.selAll": "Select all filtered results",
    "sc.clear": "Clear selection",
    "sc.seek": "Seek video 10s back / forward",
    "sc.rate": "Decrease / increase playback speed",
    "sc.range": "Set IN / OUT range",
    "sc.fs": "Video fullscreen",
    "sc.theater": "Theater mode",
    "sc.pip": "Picture-in-picture",
    "sc.exit": "Exit mode",
    "sc.rangeSel": "Range select",
    "media.size": "Size",
    "media.fit": "Fit",
    "panel.selected": "Selected evidence",
    "menu.view": "View ▾",
    "menu.tag": "Tags ▾",
    "menu.export": "Export & copy ▾",
    "menu.filters": "Sort & display ▾",
    "menu.timeline": "Timeline",
    "timeline.title": "Case timeline",
    "timeline.gen": "Generate/refresh timeline",
    "btn.close": "Close",
    "toolbar.skipBack": "« -10s",
    "toolbar.skipFwd": "+10s »",
    "toolbar.popPlayer": "⧉ Pop out player",
    "toolbar.download": "⤓ Save",
    "toolbar.capture": "Capture frame",
    "toolbar.in": "[ IN",
    "toolbar.out": "OUT ]",
    "toolbar.clip": "Export clip",
    "toolbar.clipBurn": "Court export",
    "toolbar.dual": "Dual sync",
    "toast.noPair": "No paired channel video — needs same-name files with _F/_R or front/rear in the name.",
    "panel.telemetry": "Drive track",
    "btn.telemetry": "Extract",
    "telemetry.summary": "{n} track points (relative sketch — no basemap)",
    "telemetry.maxSpeed": "max {v}km/h",
    "telemetry.noPoints": "No track points extracted",
    "toast.telemetryDone": "Telemetry: {n} track points",
    "toast.telemetryFail": "Telemetry extraction failed: {err}",
    "toast.telemetryOffline": "Telemetry extraction needs the workstation server.",
    "export.transcode": "Unplayable-format proxy queue",
    "toast.queueRunning": "Transcode queue running — may take a while for many files.",
    "toast.queueDone": "Queue done: {t} candidates · {p} proxied · {c} cached · {f} failed",
    "toast.queueFail": "Proxy queue failed: {err}",
    "toast.queueOffline": "The proxy queue needs the workstation server.",
    "clip.exhibitPrompt": "Exhibit label to burn into the video (e.g. Exhibit A). Leave empty to stamp only case id, hash, and timecode.",
    "clip.exhibitDefault": "Exhibit A",
    "toolbar.proxy": "Play proxy",
    "panel.inspector": "Forensic inspector",
    "panel.validation": "Validation log",
    "panel.source": "Source",
    "panel.source.desc": "Never modified",
    "panel.recovered": "Recovered",
    "panel.recovered.desc": "Candidate until verified",
    "sel.selectAll": "Select all",
    "sel.clear": "Clear",
    "sel.groupMark": "Mark",
    "mark.reviewed": "Reviewed",
    "mark.important": "Important",
    "mark.verify": "Needs verification",
    "mark.clear": "Unmark",
    "sel.examiner": "Reviewer",
    "sel.apply": "Apply to case",
    "export.files": "Download selected files",
    "export.selected": "Package selection",
    "export.csv": "Selection CSV",
    "export.summary": "Summary report",
    "export.marks": "Download marks",
    "export.list": "Download selection list",
    "export.copyIds": "Copy IDs",
    "export.copyPaths": "Copy paths",
    "filter.tag.all": "Tag: all",
    "shortcuts.title": "Shortcuts",
    "fs.exit": "✕ Close — back to workstation",
    "chip.groupKind": "Group by source",
    "grid.label": "Evidence list",
    "rectype.driving": "Driving",
    "rectype.event": "Event (impact)",
    "rectype.parking": "Parking",
    "rectype.unclassified": "Unclassified",
    "chan.front": "Front (F)",
    "chan.rear": "Rear (R)",
    "chan.interior": "Interior (I)",
    "chan.rear2": "Rear 2 (B)",
    "misc.other": "Other",
    "status.verified": "Verified",
    "status.confirmed": "ffprobe confirmed",
    "status.failed": "Validation failed",
    "status.candidate": "Unvalidated candidate",
    "status.duplicate": "Duplicate candidate",
    "mark.noted": "Note",
    "mark.cleared": "Unmark",
    "chip.all": "All",
    "chip.recovery": "Deleted/recovered items",
    "chip.marked": "Marked/tagged items",
    "chip.anomaly": "Anomaly candidates",
    "chip.warning": "With warnings",
    "chip.failed": "Validation failed",
    "chip.candidate": "Unvalidated candidates",
    "chip.duplicate": "Duplicate candidates",
    "chip.markImportant": "Marked important",
    "chip.markReviewed": "Reviewed",
    "chip.markNone": "Awaiting review",
    "filter.label": "Filters",
    "group.noTime": "Unknown time",
    "group.noMark": "No mark",
    "group.noChannel": "Unknown channel",
    "tree.all": "All",
    "tree.rectype": "Recording type",
    "tree.kind": "Source",
    "tree.date": "Date",
    "kind.short.video": "Source",
    "kind.short.carved": "Carved",
    "kind.short.filesystem": "Recovered",
    "kind.short.candidate": "Pre-recovery",
    "metric.indexed": "{n} indexed",
    "sel.count": "{n} selected · {m} marked",
    "triage.line": "· {done}/{total} reviewed (important {imp} · needs verification {pend})",
    "warn.count": "{n} warnings",
    "warn.title": "Warnings",
    "dfl.badge": "Deepfake suspicion {band}",
    "dfl.title": "deepfake-lens score {score} — review priority, not a verdict",
    "dfl.fail": "Screening failed",
    "dfl.failTitle": "deepfake-lens screening failed: {error}",
    "dfl.dist": "Deepfake screening {n}/{total} — high {high} · medium {medium} · low {low} · unknown {unknown} · failed {failed} · not screened {none}",
    "hist.item": "{day} {count} items",
    "hist.empty": "No evidence with time info — extracted from filename patterns or mtimes.",
    "hist.range": "Recording span: {from} ~ {to} · time known for {known}/{total} (from filename/mtime)",
    "hist.none": "No evidence has an extracted recording time (filename pattern or mtime required)",
    "detail.recTime": "Recorded at",
    "detail.est": "(est.)",
    "detail.channel": "Channel",
    "detail.mark": "Mark",
    "detail.unmarked": "Unreviewed",
    "detail.len": "Duration",
    "detail.size": "Size",
    "detail.anomaly": "Anomaly candidates",
    "detail.dfl": "Deepfake suspicion",
    "detail.dflScore": "{label} ({score} pts — review priority)",
    "detail.dflFail": "Screening failed: {error}",
    "detail.dflNone": "No result",
    "detail.original": "Original",
    "detail.origPath": "Original path",
    "detail.path": "Stored path",
    "detail.codec": "Codec",
    "detail.offset": "Offset",
    "detail.unknown": "Unknown",
    "tag.input": "Custom",
    "tag.apply": "Apply",
    "tag.presetSave": "Save as preset",
    "tag.presetSave.title": "Also register the typed tag in the preset list (shown in tag menu/filter)",
    "note.placeholder": "Review note — findings on this evidence (applied to the case with marks/tags)",
    "validation.none": "No validation log",
    "toast.selectTarget": "Select a target evidence first.",
    "toast.selectFirst": "Select evidence first.",
    "toast.storageFail": "Cannot write to browser storage: {err}",
    "toast.selectedAll": "Selected {n} filtered results.",
    "toast.markApplied": "Applied '{status}' to {n} evidence items.",
    "toast.tagApplied": "Applied tag '{tag}' to {n} evidence items.",
    "toast.tagCleared": "Cleared tags on {n} evidence items.",
    "toast.presetAdded": "Added tag '{tag}' to presets.",
    "toast.downloadStart": "Started downloading {name}.",
    "toast.copied": "Copied {label} to clipboard.",
    "copy.ids": "evidence IDs",
    "copy.paths": "evidence paths",
    "toast.copyFail": "Copy failed: {err}",
    "toast.fsExitFail": "Failed to exit fullscreen: {err}",
    "toast.fsEnterFail": "Failed to enter fullscreen: {err}",
    "toast.fsUnsupported": "This browser does not support fullscreen.",
    "toast.noVideo": "No video is playing.",
    "toast.pipUnsupported": "This browser does not support picture-in-picture.",
    "toast.pipFail": "Picture-in-picture failed: {err}",
    "toast.noRecord": "No evidence to show.",
    "toast.popupBlocked": "Popup blocked — allow popups in browser settings.",
    "toast.captureSaved": "Frame saved: {path}",
    "toast.captureFail": "Capture failed: {err}",
    "toast.captureOffline": "Capture failed: cannot reach the workstation server.",
    "toast.pickEvidence": "Select the playing evidence first.",
    "toast.outBeforeIn": "OUT precedes IN — set it again.",
    "toast.playPick": "Select evidence to play first.",
    "toast.playOriginal": "Playing the original video.",
    "toast.proxyBuilding": "Building proxy… may take minutes depending on source size.",
    "toast.proxyPlaying": "Playing proxy — press again to return to the original.",
    "toast.proxyFail": "Proxy failed: {err}",
    "toast.proxyOffline": "Proxy failed: cannot reach the workstation server.",
    "toast.noRange": "Set an IN/OUT range first (i / o keys).",
    "toast.clipDone": "Clip export done: {path}",
    "toast.clipFail": "Export failed: {err}",
    "toast.clipOffline": "Export failed: cannot reach the workstation server.",
    "toast.noChanges": "No review changes to apply.",
    "toast.applyDone": "Review marks applied to the case and the report refreshed.",
    "toast.applyFail": "Apply failed: {err}",
    "toast.applyOffline": "Apply failed: cannot reach the server.",
    "toast.noItems": "No selected or marked items.",
    "toast.exportFail": "Export failed: {err}",
    "toast.exportDone": "Copied {copied} selected items (skipped {skipped}) — {dir}",
    "toast.offline": "Cannot reach the workstation server — open the viewer via the server address.",
    "toast.fileOnly": "File download requires a server-hosted viewer — open it from the workstation.",
    "toast.downloads": "Started {n} file downloads — the browser may ask to allow multiple downloads.",
    "toast.standalone": "Viewer opened directly from file — playback, packaging and mark apply need the server viewer (127.0.0.1:8477); marks/tags made here are stored separately.",
    "timeline.offline": "Timeline is unavailable in a file-opened viewer — use the server viewer.",
    "tagmenu.delPreset": "Remove from presets",
    "tagmenu.clear": "Clear tags",
    "tagmenu.new": "New tag",
    "tagmenu.add": "Add",
    "filter.tag.named": "Tag: {tag}",
    "report.title": "FrameTrace review summary",
    "report.meta": "Case: {case} · Generated: {when} · {n} items",
    "report.counts": "Verified {v} · Candidates {c} · Validation failed {f}",
    "report.note": "This document summarizes items selected by the reviewer. Verify original evidence integrity against the case audit log and SHA-256 values. Items marked 'candidate' are unvalidated and require further verification before being asserted as evidence.",
    "report.th.name": "Filename",
    "report.th.kind": "Source",
    "report.th.status": "Validation status",
    "report.th.mark": "Mark",
    "report.th.tags": "Tags",
    "report.th.size": "Size",
    "report.th.time": "Recorded at",
    "report.th.warn": "Warnings/notes",
    "report.th.orig": "Original path",
    "report.memo": "Note:",
    "player.novid": "No playable file on this item (e.g. pre-recovery candidate) — skip to the next",
    "player.list": "Evidence list",
    "player.prev": "‹ Prev",
    "player.next": "Next ›",
    "player.play": "Play/Pause",
    "player.capTitle": "Save current frame to the case",
    "player.hint": "←/→ ±10s · ↑/↓ rate · space play · k/j prev/next",
    "player.mark": "Mark",
    "player.tags": "Tags",
    "player.reviewed": "Reviewed (1)",
    "player.important": "Important (2)",
    "player.verify": "Needs verification (3)",
    "player.clear": "Clear (0)",
    "player.dead": "Main viewer closed — marks/tags/navigation disabled.",
    "player.none": "No evidence to show",
    "player.last": "Last item",
    "player.first": "First item",
    "player.noVideo": "No video is playing",
    "player.saved": "Frame saved: {path}",
    "player.capFail": "Capture failed: {err}",
    "player.capOffline": "Capture failed: cannot reach the server",
    "player.warn": "{n} warnings",
    "player.meta": "{id} · {status} · {idx}/{total}",
    "empty.index": "No indexed evidence.",
    "empty.play": "No directly playable file URL.",
    "range.label": "Range {in} ~ {out}",
    "cand.vendor": "Deleted video candidate",
    "cand.note": "Pre-recovery deleted video candidate — recover with recover-batch, then validate.",
    "player.cap": "Capture frame",
    "player.warnShort": "warnings",
    "player.savedPfx": "Frame saved:",
    "player.capFailPfx": "Capture failed:",
    "warn.inspect": "Inspection warnings: {n}",
    "timeline.building": "Building timeline…",
    "timeline.fail": "Build failed: {err}",
    "timeline.failOffline": "Build failed: cannot reach the server",
    "timeline.none": "No timeline yet — press 'Build/refresh timeline'.",
    "timeline.count": "{n} events · candidate-grade (based on recorded metadata)",
    "timeline.ellipsis": "… {n} more omitted (full list: db/timeline.jsonl)",
    "timeline.unreadable": "Cannot read the timeline.",
    "offline.openServer": "Open the viewer via the workstation server (127.0.0.1:8477) to use this.",
    "offline.applyTitle": "Direct case apply is only available in the server-hosted viewer — use 'Download marks' to save a file.",
    "offline.serverOnly": "Only available when the viewer is opened via the workstation server."
  }
};

function t(key) {
  let locale = storageGet(LOCALE_KEY, "ko") || "ko";
  // `state` is a top-level const initialized after the record maps that
  // call t() run — touching it in its TDZ throws ReferenceError, so the
  // access must be guarded rather than assumed initialized.
  try {
    if (state && state.locale) locale = state.locale;
  } catch { /* state not initialized yet */ }
  return (I18N[locale] && I18N[locale][key]) || (I18N.ko[key]) || key;
}

// tf formats a locale string with {name} placeholders.
function tf(key, subs) {
  let s = t(key);
  if (subs) for (const [name, value] of Object.entries(subs)) {
    s = s.split("{" + name + "}").join(String(value));
  }
  return s;
}

function applyChromeI18n() {
  document.documentElement.lang = state.locale;
  document.querySelectorAll("[data-i18n]").forEach(el => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach(el => {
    el.setAttribute("placeholder", t(el.dataset.i18nPlaceholder));
  });
  document.querySelectorAll("[data-i18n-title]").forEach(el => {
    el.setAttribute("title", t(el.dataset.i18nTitle));
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach(el => {
    el.setAttribute("aria-label", t(el.dataset.i18nAriaLabel));
  });
  const langBtn = document.getElementById("btnLang");
  if (langBtn) langBtn.textContent = state.locale === "ko" ? "EN" : "KO";
}

const videos = Array.isArray(scan.videos) ? scan.videos : [];

const flsEntryByInode = new Map();
(DATA.flsEntries || []).forEach(entry => {
  if (entry.inode != null) flsEntryByInode.set(String(entry.inode), entry);
});

const TIME_PATTERNS = [
  { re: /(20\d{2})[_.\-]?(0[1-9]|1[0-2])[_.\-]?(0[1-9]|[12]\d|3[01])[ T_\-]+([01]\d|2[0-3])[:_.\-]?([0-5]\d)[:_.\-]?([0-5]\d)/, hasTime: true },
  { re: /(20\d{2})(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])[ T_\-]+([01]\d|2[0-3])([0-5]\d)([0-5]\d)/, hasTime: true },
  { re: /(20\d{2})[_.\-]?(0[1-9]|1[0-2])[_.\-]?(0[1-9]|[12]\d|3[01])(?!\d)/, hasTime: false }
];

function parseTimeFromName(name) {
  const text = String(name || "");
  for (const pattern of TIME_PATTERNS) {
    const match = text.match(pattern.re);
    if (!match) continue;
    const [year, month, day] = [match[1], match[2], match[3]];
    const hh = match[4] ?? "00";
    const mm = match[5] ?? "00";
    const ss = match[6] ?? "00";
    const ts = new Date(+year, +month - 1, +day, +hh, +mm, +ss).getTime() / 1000;
    if (Number.isFinite(ts)) {
      return { ts, date: `${year}-${month}-${day}`, source: "name" };
    }
  }
  return null;
}

function recordingTimeFor(record) {
  for (const candidate of [record.originalPath, record.name, record.path]) {
    const parsed = parseTimeFromName(candidate);
    if (parsed) return parsed;
  }
  if (record.kind === "video" && record.modifiedUnix) {
    const day = new Date(record.modifiedUnix * 1000).toLocaleDateString("sv-SE");
    return { ts: record.modifiedUnix, date: day, source: "mtime" };
  }
  return null;
}

function recTypeFor(record) {
  const text = `${record.originalPath || ""} ${record.originalPath ? "" : record.name || ""}`.toLowerCase();
  if (/(event|충격|사고|impact)/.test(text)) return "event";
  if (/(parking|주차)/.test(text)) return "parking";
  if (/(driving|주행|상시|continuous|normal)/.test(text)) return "driving";
  return "";
}

function recTypeLabel(recType) {
  return ({ driving: t("rectype.driving"), event: t("rectype.event"), parking: t("rectype.parking"), unclassified: t("rectype.unclassified") })[recType] || t("rectype.unclassified");
}

function channelCodeOf(record) {
  const name = `${record.originalPath || ""} ${record.name || ""}`;
  let match = name.match(/[_\-. ]([FRIB])(?:[_.\- ]|[a-z0-9]*$)/i);
  if (match) return match[1].toUpperCase();
  if (/front/i.test(name)) return "F";
  if (/rear/i.test(name)) return "R";
  if (/interior|inside/i.test(name)) return "I";
  return null;
}

function channelLabel(code) {
  return ({ F: t("chan.front"), R: t("chan.rear"), I: t("chan.interior"), B: t("chan.rear2") })[code] || code;
}

function channelFor(record) {
  const code = channelCodeOf(record);
  return code ? channelLabel(code) : null;
}

// Channel pairing for dual sync playback: two records mate when their
// filenames differ only by a channel token (_F/_R/_I/_B or front/rear/
// interior) inside the same folder. The channel token is stripped to form
// a shared key — `20240101_1200_F.mp4` and `20240101_1200_R.mp4` pair.
function pairKeyOf(record) {
  const p = String(record.originalPath || record.name || "").replace(/\\/g, "/");
  const dir = p.split("/").slice(0, -1).join("/").toLowerCase();
  let base = (p.split("/").pop() || "").replace(/\.[^.]*$/, "").toLowerCase();
  base = base
    .replace(/front|rear|interior|inside/g, "")
    .replace(/[_\-. ][frib](?=$|[_\-. ])/g, "")
    .replace(/^[ _\-.]+|[ _\-.]+$/g, "");
  return base ? `${dir}|${base}` : "";
}

function matesOf(record) {
  const code = channelCodeOf(record);
  const key = pairKeyOf(record);
  if (!code || !key) return [];
  return records.filter(r =>
    r.id !== record.id &&
    pairKeyOf(r) === key &&
    channelCodeOf(r) && channelCodeOf(r) !== code &&
    mediaSrcFor(r)
  );
}

// Dual-channel panes: any pane can drive — user events are mirrored to the
// others. A suppress flag stops echo loops, and a drift poll re-locks
// panes that wander (>0.3s) without a user event (decoder jitter,
// background tab throttling).
function wireDualSync(videos) {
  const list = Array.from(videos);
  if (list.length < 2) return;
  let suppress = false;
  const release = () => setTimeout(() => { suppress = false; }, 250);
  list.forEach(src => {
    src.addEventListener("play", () => {
      if (suppress) return;
      suppress = true;
      list.forEach(v => { if (v !== src) { v.currentTime = src.currentTime; v.play().catch(() => {}); } });
      release();
    });
    src.addEventListener("pause", () => {
      if (suppress) return;
      suppress = true;
      list.forEach(v => { if (v !== src) v.pause(); });
      release();
    });
    src.addEventListener("seeked", () => {
      if (suppress) return;
      suppress = true;
      list.forEach(v => { if (v !== src) v.currentTime = src.currentTime; });
      release();
    });
    src.addEventListener("ratechange", () => {
      if (suppress) return;
      suppress = true;
      list.forEach(v => { if (v !== src) v.playbackRate = src.playbackRate; });
      release();
    });
  });
  const drift = setInterval(() => {
    if (!list[0].isConnected) { clearInterval(drift); return; }
    const driver = list.find(v => !v.paused) || list[0];
    list.forEach(v => {
      if (v !== driver && Math.abs(v.currentTime - driver.currentTime) > 0.3) {
        v.currentTime = driver.currentTime;
      }
    });
  }, 500);
}

function prefixFor(record) {
  const name = String(record.originalPath || record.name || "");
  const match = name.match(/[A-Za-z가-힣_\-]+/);
  return match ? match[0].replace(/[_\-]+$/, "") || t("misc.other") : t("misc.other");
}

function originalNameFor(record) {
  const path = originalPathFor(record);
  if (path) {
    const base = path.replace(/\\/g, "/").split("/").pop();
    if (base && base.includes(".")) return base;
  }
  return record.name || "";
}

function originalPathFor(record) {
  if (record.inode && flsEntryByInode.has(String(record.inode))) {
    return flsEntryByInode.get(String(record.inode)).path || "";
  }
  // Recovered outputs are named inode_<num>.bin; map that back to the
  // original path recorded by inspect-image.
  const match = String(record.name || "").match(/inode_(\d+)/);
  if (match && flsEntryByInode.has(match[1])) {
    return flsEntryByInode.get(match[1]).path || "";
  }
  return "";
}

function fmtUnix(value) {
  if (!Number.isFinite(value)) return "-";
  return new Date(value * 1000).toLocaleString();
}

function normalizePath(value) {
  let text = String(value || "");
  if (text.slice(0, 4).toLowerCase() === EXT_PREFIX) text = text.slice(4);
  return text.split(BS).join("/").toLowerCase();
}

function fileUrl(path) {
  if (!path) return "";
  let value = String(path);
  if (value.startsWith("file:")) return value;
  if (value.slice(0, 8).toLowerCase() === EXT_UNC) value = BS + BS + value.slice(8);
  else if (value.slice(0, 4).toLowerCase() === EXT_PREFIX) value = value.slice(4);
  const normalized = value.split(BS).join("/");
  // encodeURI leaves #, ?, and & unescaped, which truncates file URLs whose
  // evidence paths contain them; encode per segment instead, keeping the
  // drive-letter colon literal.
  const encodeSegment = segment => (/^[A-Za-z]:$/.test(segment) ? segment : encodeURIComponent(segment));
  const encoded = normalized.split("/").map(encodeSegment).join("/");
  if (normalized.length > 2 && normalized[1] === ":" && normalized[2] === "/") return "file:///" + encoded;
  if (normalized.startsWith("//")) return "file:" + encoded;
  if (normalized.startsWith("/")) return "file://" + encoded;
  return encoded;
}

function escapeHtml(value) {
  return String(value ?? "").replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&#39;");
}

// When the viewer is served by the local examiner workstation (http://127.0.0.1),
// file:// video sources are blocked by the browser; route playback through the
// server's Range-enabled /media endpoint instead. Opening the page directly
// from disk keeps the original file:// URL.
function mediaSrcFor(record) {
  if (location.protocol === "http:" || location.protocol === "https:") {
    // An examiner-requested review proxy wins over the original for playback.
    const proxy = state.proxies?.[record.id];
    if (proxy) return "/media?path=" + encodeURIComponent(proxy);
    if (record.path) return "/media?path=" + encodeURIComponent(record.path);
    return "";
  }
  if (!record.fileUrl) return "";
  return record.fileUrl;
}

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function highlightEscape(value, query) {
  const safe = escapeHtml(value);
  if (!query) return safe;
  try {
    return safe.replaceAll(new RegExp("(" + escapeRegExp(escapeHtml(query)) + ")", "gi"), "<mark>$1</mark>");
  } catch (error) {
    return safe;
  }
}

function fmtBytes(value) {
  if (!Number.isFinite(value)) return "-";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let current = value;
  let unit = 0;
  while (current >= 1024 && unit < units.length - 1) { current /= 1024; unit += 1; }
  return `${current.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function fmtDuration(value) {
  if (!Number.isFinite(value)) return "-";
  const seconds = Math.round(value);
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

function statusLabel(status) {
  if (status === "ffprobe-video-stream-confirmed") return t("status.verified");
  if (status === "ffprobe-confirmed") return t("status.confirmed");
  if (status === "validation-failed") return t("status.failed");
  if (status === "candidate-unvalidated") return t("status.candidate");
  if (status === "duplicate-candidate") return t("status.duplicate");
  return status;
}

function statusClass(status) {
  if (status === "ffprobe-video-stream-confirmed" || status === "ffprobe-confirmed") return "ok";
  if (status === "validation-failed") return "failed";
  return "candidate";
}

function toast(message) {
  const host = document.getElementById("toastHost");
  const item = document.createElement("div");
  item.className = "toast";
  item.textContent = message;
  host.appendChild(item);
  setTimeout(() => item.remove(), 3200);
}

function storageGet(key, fallback) {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
  } catch (error) {
    return fallback;
  }
}

function storageSet(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch (error) {
    toast(tf("toast.storageFail", { err: error.message }));
    return false;
  }
}

const validationsByPath = new Map(validationLog.map(item => [normalizePath(item.target_path), item]));
const recoveredFilesystemLog = filesystemLog.filter(item => item.event === "recover-inode" && item.output_path);
// Case-level warnings recorded on inspection events (e.g. a truncated
// fls listing means the deleted-file list is incomplete) — the reviewer
// must see them, not just the CLI.
const inspectionWarnings = filesystemLog
  .filter(item => item.event === "inspect-image-filesystem" && Array.isArray(item.warnings) && item.warnings.length)
  .flatMap(item => item.warnings);

const records = [
  ...videos.map(video => {
    const validation = validationsByPath.get(normalizePath(video.source_path));
    return {
      id: video.id,
      kind: "video",
      name: video.relative_path || video.id,
      path: video.source_path,
      fileUrl: video.file_url,
      parser: video.source_profile?.parser || "-",
      vendor: video.source_profile?.vendor || "-",
      status: validation?.validation_status || (video.ffprobe_ok ? "ffprobe-confirmed" : "candidate-unvalidated"),
      sha256: validation?.target_sha256 || video.sha256 || video.hash_status || "-",
      duration: validation?.duration_seconds ?? video.duration_seconds,
      codec: validation?.video_codec || video.video_codec || "-",
      size: video.size_bytes,
      note: validation?.validation_note || video.source_profile?.recommended_action || "-",
      indexStatus: video.index_status || "active",
      modifiedUnix: video.modified_unix,
      inode: video.inode,
      validation
    };
  }),
  ...carveLog.map(item => {
    const validation = validationsByPath.get(normalizePath(item.output_path));
    return {
      id: item.id || item.output_path,
      kind: "carved",
      name: item.output_path ? item.output_path.split(/[\\\/]/).pop() : item.id,
      path: item.output_path,
      fileUrl: fileUrl(item.output_path),
      parser: item.signature || "carve",
      vendor: "Recovered candidate",
      status: validation?.validation_status || item.validation_status || "candidate-unvalidated",
      sha256: validation?.target_sha256 || item.sha256 || "-",
      duration: validation?.duration_seconds,
      codec: validation?.video_codec || item.extension || "-",
      size: item.size_bytes,
      note: validation?.validation_note || item.validation_note || "-",
      offset: item.offset,
      indexStatus: "active",
      validation
    };
  }),
  ...(DATA.flsEntries || [])
    .filter(entry => entry.deleted && entry.video_candidate)
    .map(entry => {
      const inode = entry.inode != null ? String(entry.inode) : "";
      return {
        id: `fls:${inode || entry.raw_line || "unknown"}`,
        kind: "candidate",
        name: entry.path || entry.raw_line || `inode ${inode}`,
        path: "",
        fileUrl: "",
        parser: "fls listing",
        vendor: t("cand.vendor"),
        status: "candidate-unvalidated",
        sha256: "-",
        duration: null,
        codec: "-",
        size: null,
        note: t("cand.note"),
        indexStatus: "recovery-pending",
        modifiedUnix: null,
        inode: entry.inode,
        originalPath: entry.path || "",
        validation: null
      };
    }),
  ...recoveredFilesystemLog.map(item => {
    const original = item.inode && flsEntryByInode.get(String(item.inode));
    const validation = validationsByPath.get(normalizePath(item.output_path));
    return {
      id: `inode:${item.partition_offset ?? 0}:${item.inode || item.output_path}`,
      kind: "filesystem",
      name: item.output_path ? item.output_path.split(/[\\\/]/).pop() : item.inode,
      path: item.output_path,
      fileUrl: fileUrl(item.output_path),
      parser: "tsk/icat",
      vendor: "Filesystem recovery",
      status: validation?.validation_status || item.validation_status || "candidate-unvalidated",
      sha256: validation?.target_sha256 || item.sha256 || "-",
      duration: validation?.duration_seconds,
      codec: validation?.video_codec || "-",
      size: item.size_bytes,
      note: validation?.validation_note || "Recovered inode output; validate before final reporting.",
      offset: item.partition_offset,
      inode: item.inode,
      originalPath: original?.path || "",
      warnings: Array.isArray(item.warnings) ? item.warnings : [],
      indexStatus: "active",
      validation
    };
  })
];

records.forEach(record => {
  record.originalPath = record.originalPath || originalPathFor(record);
  const rec = recordingTimeFor(record);
  record.recTime = rec ? rec.ts : null;
  record.recDay = rec ? rec.date : null;
  record.recSource = rec ? rec.source : null;
  record.channel = channelFor(record);
  record.prefix = prefixFor(record);
  record.recType = recTypeFor(record);
    record.originalName = originalNameFor(record);
  record.thumb = DATA.thumbs?.[record.id] || null;
  // Artifact filenames sanitize record ids (inode:<off>:<ino> →
  // inode_<off>_<ino>) — mirror the same mapping for the lookup.
  record.dfl = DATA.deepfake?.[String(record.id).replace(/[:\\\/]/g, "_")] || null;
  record.telemetry = DATA.telemetry?.[String(record.id).replace(/[:\\\/]/g, "_")] || null;
  const fromId = anomaliesBySelector.get(record.id) || [];
  const fromValidation = Array.isArray(record.validation?.anomaly_flags)
    ? record.validation.anomaly_flags.map(kind => ({ kind, selector: record.id, detail: "validation anomaly_flags" }))
    : [];
  const merged = [...fromId];
  for (const item of fromValidation) {
    if (!merged.some(existing => existing.kind === item.kind)) merged.push(item);
  }
  record.anomalies = merged;
  record.hasAnomaly = record.anomalies.length > 0;
});
// Carve-log and inode records duplicate indexed files under different ids;
// reuse the indexed video's thumbnail through the shared file name.
const thumbByName = new Map(records.filter(record => record.thumb).map(record => [record.name, record.thumb]));
records.forEach(record => {
  if (!record.thumb && record.name) record.thumb = thumbByName.get(record.name) || null;
});

const state = {
  activeId: records[0]?.id || null,
  selectedIds: new Set(),
  lastCheckedKey: null,
  drafts: storageGet(ANNOTATIONS_KEY, {}),
  deletedIds: [],
  examiners: {},
  marks: storageGet(MARKS_KEY, {}),
  notes: storageGet(NOTES_KEY, {}),
  ranges: storageGet(RANGES_KEY, {}),
  proxies: storageGet(PROXIES_KEY, {}),
  examiner: storageGet(EXAMINER_KEY, ""),
  tags: storageGet(TAGS_KEY, {}),
  tagPresets: storageGet(TAG_PRESETS_KEY, null),
  locale: storageGet(LOCALE_KEY, "ko") === "en" ? "en" : "ko",
  layout: Object.assign({ videoMode: "fit", videoZoom: 100, theater: false, dual: false, playerH: 0, colSplit: 0, rate: 1 }, storageGet(LAYOUT_KEY, {})),
  currentPage: 1,
  pageSize: 100,
  query: "",
  kind: "",
  status: "",
  chip: "",
  sortBy: "id",
  groupBy: "none",
  dateFrom: "",
  dateTo: "",
  recType: "",
  collapsedGroups: new Set()
};

function touchAnnotation(id) {
  state.examiners[id] = (state.examiner || "").trim() || state.examiners[id] || null;
  const draft = {
    mark: state.marks[id] || null,
    note: state.notes[id] || "",
    tags: state.tags[id] || [],
    examiner: state.examiners[id]
  };
  state.drafts[id] = draft;
  state.deletedIds = [...new Set([...state.deletedIds, id])];
  storageSet(ANNOTATIONS_KEY, state.drafts);
  markReviewDirty();
}

function hydrateAnnotations(annotations) {
  state.marks = {}; state.notes = {}; state.tags = {}; state.examiners = {};
  for (const mark of annotations.marks || []) {
    if (mark.status !== "noted") state.marks[mark.id] = { status: mark.status, marked_unix: mark.marked_unix };
    state.notes[mark.id] = mark.note || "";
    state.examiners[mark.id] = mark.examiner || null;
  }
  for (const entry of annotations.tags || []) state.tags[entry.id] = entry.tags;
  state.deletedIds = [];
  for (const [id, draft] of Object.entries(state.drafts)) {
    if (draft.mark) state.marks[id] = draft.mark;
    else delete state.marks[id];
    state.notes[id] = draft.note;
    state.tags[id] = draft.tags;
    state.examiners[id] = draft.examiner;
    state.deletedIds.push(id);
  }
}

hydrateAnnotations(DATA.annotations || { marks: [], tags: [] });

// null = never customized → seed from defaults; an empty array is a
// deliberate "no presets" choice and must not reseed.
if (!Array.isArray(state.tagPresets)) state.tagPresets = [...DEFAULT_TAG_PRESETS];

// Same-origin hosts (e.g. the examiner workstation iframe) read live case
// stats through this getter instead of re-parsing the embedded data.
window.__frametraceSummary = () => {
  const byKind = {};
  const byStatus = {};
  let warned = 0;
  records.forEach(r => {
    byKind[r.kind || "video"] = (byKind[r.kind || "video"] || 0) + 1;
    byStatus[r.status] = (byStatus[r.status] || 0) + 1;
    if ((r.warnings || []).length) warned += 1;
  });
  return {
    total: records.length,
    byKind,
    byStatus,
    warned,
    marked: Object.keys(state.marks).length,
    noted: records.filter(r => (state.notes[r.id] || "").trim()).length
  };
};

const els = {
  caseLine: document.getElementById("caseLine"),
  resultCount: document.getElementById("resultCount"),
  triageStatus: document.getElementById("triageStatus"),
  metricVideos: document.getElementById("metricVideos"),
  metricCarved: document.getElementById("metricCarved"),
  metricVerified: document.getElementById("metricVerified"),
  metricFailed: document.getElementById("metricFailed"),
  metricAnomaly: document.getElementById("metricAnomaly"),
  query: document.getElementById("query"),
  kind: document.getElementById("kind"),
  status: document.getElementById("status"),
  pageSize: document.getElementById("pageSize"),
  presetChips: document.getElementById("presetChips"),
  recordGrid: document.getElementById("recordGrid"),
  prevPage: document.getElementById("prevPage"),
  nextPage: document.getElementById("nextPage"),
  pageStatus: document.getElementById("pageStatus"),
  mediaTitle: document.getElementById("mediaTitle"),
  mediaStatus: document.getElementById("mediaStatus"),
  mediaStage: document.getElementById("mediaStage"),
  detailBadges: document.getElementById("detailBadges"),
  summaryList: document.getElementById("summaryList"),
  metaList: document.getElementById("metaList"),
  validationList: document.getElementById("validationList"),
  selectionBar: document.getElementById("selectionBar"),
  selectionCount: document.getElementById("selectionCount"),
  videoMode: document.getElementById("videoMode"),
  videoZoom: document.getElementById("videoZoom"),
  playRate: document.getElementById("playRate"),
  facetTree: document.getElementById("facetTree"),
  sortBy: document.getElementById("sortBy"),
  groupBy: document.getElementById("groupBy"),
  tagFilter: document.getElementById("tagFilter"),
  dateFrom: document.getElementById("dateFrom"),
  dateTo: document.getElementById("dateTo"),
  dayHistogram: document.getElementById("dayHistogram"),
  periodLabel: document.getElementById("periodLabel"),
  dflSummary: document.getElementById("dflSummary"),
  caseWarnings: document.getElementById("caseWarnings")
};

const PRESET_CHIPS = [
  ["", "chip.all"],
  ["kind:recovery", "chip.recovery"],
  ["marked:any", "chip.marked"],
  ["anomaly", "chip.anomaly"],
  ["warning", "chip.warning"],
  ["status:validation-failed", "chip.failed"],
  ["status:candidate-unvalidated", "chip.candidate"],
  ["status:duplicate-candidate", "chip.duplicate"],
  ["mark:important", "chip.markImportant"],
  ["mark:reviewed", "chip.markReviewed"],
  ["mark:none", "chip.markNone"],
];

function filteredRecords() {
  const list = records.filter(record => {
    if (state.kind && record.kind !== state.kind) return false;
    if (state.recType && (record.recType || "unclassified") !== state.recType) return false;
    if (state.status && record.status !== state.status) return false;
    if (state.chip === "anomaly" && !record.hasAnomaly) return false;
    if (state.chip === "warning" && !(record.warnings || []).length) return false;
    if (state.chip === "kind:recovery" && record.kind !== "candidate" && record.kind !== "filesystem") return false;
    if (state.chip === "marked:any" && !state.marks[record.id] && !(state.tags[record.id] || []).length && !(state.notes[record.id] || "").trim()) return false;
    if (state.dateFrom || state.dateTo) {
      if (!record.recDay) return false;
      if (state.dateFrom && record.recDay < state.dateFrom) return false;
      if (state.dateTo && record.recDay > state.dateTo) return false;
    }
    if (state.chip.startsWith("tag:")) {
      const wanted = state.chip.slice(4);
      const tags = state.tags[record.id] || [];
      if (!tags.includes(wanted)) return false;
    }
    if (state.chip.startsWith("mark:")) {
      const wanted = state.chip.slice(5);
      const mark = state.marks[record.id];
      if (wanted === "none") { if (mark) return false; }
      else if (!mark || mark.status !== wanted) return false;
    }
    if (!state.query) return true;
    const haystack = [record.id, record.name, record.path, record.originalPath, record.parser, record.vendor, record.sha256, record.note, record.status];
    return haystack.some(value => String(value ?? "").toLowerCase().includes(state.query));
  });
  const sorted = [...list];
  switch (state.sortBy) {
    case "time-desc":
      sorted.sort((a, b) => (b.recTime ?? -Infinity) - (a.recTime ?? -Infinity) || a.id.localeCompare(b.id));
      break;
    case "time-asc":
      sorted.sort((a, b) => (a.recTime ?? Infinity) - (b.recTime ?? Infinity) || a.id.localeCompare(b.id));
      break;
    case "name":
      sorted.sort((a, b) => a.name.localeCompare(b.name, "ko") || a.id.localeCompare(b.id));
      break;
    case "size-desc":
      sorted.sort((a, b) => (b.size || 0) - (a.size || 0) || a.id.localeCompare(b.id));
      break;
    default:
      sorted.sort((a, b) => a.id.localeCompare(b.id));
  }
  return sorted;
}

function groupKeyFor(record) {
  switch (state.groupBy) {
    case "day": return record.recDay || t("group.noTime");
    case "kind": return ({ video: t("filter.kind.video"), carved: t("filter.kind.carved"), filesystem: t("filter.kind.filesystem"), candidate: t("filter.kind.candidate") })[record.kind] || record.kind;
    case "recType": return recTypeLabel(record.recType);
    case "status": return statusLabel(record.status);
    case "mark": return markOf(record) ? markLabel(markOf(record).status) : t("group.noMark");
    case "prefix": return record.prefix;
    case "channel": return record.channel || t("group.noChannel");
    default: return "";
  }
}

function selectedRecord() { return records.find(record => record.id === state.activeId); }
function markOf(record) { return state.marks[record.id] || null; }

function saveLayout() {
  const { videoMode, videoZoom, theater, dual, playerH, colSplit, rate } = state.layout;
  storageSet(LAYOUT_KEY, { videoMode, videoZoom, theater, dual, playerH, colSplit, rate });
}

let layoutSaveTimer = null;
function saveLayoutSoon() {
  clearTimeout(layoutSaveTimer);
  layoutSaveTimer = setTimeout(saveLayout, 250);
}

function applyLayout() {
  document.body.classList.toggle("theater", !!state.layout.theater);
  const player = document.querySelector(".player");
  const h = Number(state.layout.playerH) || 0;
  player.style.height = h >= 160 ? h + "px" : "";
  applyColumnSplit();
  applyVideoScale();
  els.videoMode.value = state.layout.videoMode;
  els.videoZoom.value = String(state.layout.videoZoom);
  els.playRate.value = String(state.layout.rate || 1);
  const rateVideo = els.mediaStage.querySelector("video");
  if (rateVideo) rateVideo.playbackRate = state.layout.rate || 1;
  document.getElementById("btnDual")?.classList.toggle("on", !!state.layout.dual);
}

// Column split between the browse pane and the media pane. The default lives
// in CSS (55%/45%); a dragged value is stored as the left pane percentage.
function applyColumnSplit() {
  const main = document.querySelector(".main");
  const pct = Number(state.layout.colSplit) || 0;
  if (pct >= 28 && pct <= 72) {
    main.style.gridTemplateColumns = `minmax(0, ${pct}%) 10px minmax(0, 1fr)`;
  } else {
    main.style.gridTemplateColumns = "";
  }
}

function setupColumnSplitter() {
  const splitter = document.getElementById("vsplitter");
  const main = document.querySelector(".main");
  let dragging = false;
  splitter.addEventListener("pointerdown", event => {
    dragging = true;
    splitter.classList.add("dragging");
    splitter.setPointerCapture(event.pointerId);
    event.preventDefault();
  });
  splitter.addEventListener("pointermove", event => {
    if (!dragging) return;
    const rect = main.getBoundingClientRect();
    const pct = ((event.clientX - rect.left) / rect.width) * 100;
    state.layout.colSplit = Math.max(28, Math.min(72, pct));
    applyColumnSplit();
  });
  const end = () => {
    if (!dragging) return;
    dragging = false;
    splitter.classList.remove("dragging");
    saveLayout();
  };
  splitter.addEventListener("pointerup", end);
  splitter.addEventListener("dblclick", () => {
    state.layout.colSplit = 0;
    saveLayout();
    applyLayout();
  });
}

function setupHeightSplitter() {
  const splitter = document.getElementById("hsplitter");
  const player = document.querySelector(".player");
  let startY = 0;
  let startH = 0;
  let dragging = false;
  splitter.addEventListener("pointerdown", event => {
    dragging = true;
    startY = event.clientY;
    startH = player.offsetHeight;
    splitter.classList.add("dragging");
    splitter.setPointerCapture(event.pointerId);
    event.preventDefault();
  });
  splitter.addEventListener("pointermove", event => {
    if (!dragging) return;
    state.layout.playerH = Math.max(160, Math.min(window.innerHeight - 260, startH + event.clientY - startY));
    applyLayout();
  });
  const end = () => {
    if (!dragging) return;
    dragging = false;
    splitter.classList.remove("dragging");
    saveLayout();
  };
  splitter.addEventListener("pointerup", end);
  splitter.addEventListener("dblclick", () => {
    state.layout.playerH = 0;
    saveLayout();
    applyLayout();
  });
}

function applyVideoScale() {
  const mode = state.layout.videoMode;
  const zoom = Math.max(30, Math.min(300, Number(state.layout.videoZoom) || 100));
  els.mediaStage.classList.toggle("mode-fit", mode === "fit");
  els.mediaStage.classList.toggle("mode-zoom", mode !== "fit");
  const video = els.mediaStage.querySelector("video");
  if (video && mode !== "fit") {
    const native = video.videoWidth || 1280;
    const width = Math.round(native * zoom / 100);
    video.style.width = Math.min(width, 8000) + "px";
    video.style.height = "auto";
  } else if (video) {
    video.style.width = "";
    video.style.height = "";
  }
}

function renderGrid(filtered) {
  if (!filtered.some(record => record.id === state.activeId)) {
    state.activeId = filtered[0]?.id || null;
  }
  const pageCount = Math.max(1, Math.ceil(filtered.length / state.pageSize));
  state.currentPage = Math.min(Math.max(1, state.currentPage), pageCount);
  const start = (state.currentPage - 1) * state.pageSize;
  const pageRows = filtered.slice(start, start + state.pageSize);

  els.resultCount.textContent = `${filtered.length}`;
  const rangeEnd = Math.min(filtered.length, start + pageRows.length);
  const rangeStart = filtered.length ? start + 1 : 0;
  els.pageStatus.textContent = `${rangeStart}–${rangeEnd} / ${filtered.length} · ${state.currentPage}/${pageCount}`;
  let reviewed = 0, important = 0, pending = 0;
  records.forEach(record => {
    const status = state.marks[record.id]?.status;
    if (status === "reviewed") reviewed += 1;
    else if (status === "important") important += 1;
    else if (status === "needs_verification") pending += 1;
  });
  if (els.triageStatus) {
    els.triageStatus.textContent = tf("triage.line", { done: reviewed, total: records.length, imp: important, pend: pending });
  }
  if (els.dflSummary) {
    const dist = { high: 0, medium: 0, low: 0, unknown: 0, failed: 0 };
    let screened = 0;
    records.forEach(record => {
      const dfl = record.dfl;
      if (!dfl) return;
      if (dfl.error) { dist.failed += 1; screened += 1; return; }
      const band = String(dfl.band || "unknown");
      if (band in dist) dist[band] += 1; else dist.unknown += 1;
      screened += 1;
    });
    els.dflSummary.textContent = screened
      ? tf("dfl.dist", { n: screened, total: records.length, high: dist.high, medium: dist.medium, low: dist.low, unknown: dist.unknown, failed: dist.failed, none: records.length - screened })
      : "";
  }
  els.prevPage.disabled = state.currentPage <= 1;
  els.nextPage.disabled = state.currentPage >= pageCount;

  const groupCounts = new Map();
  if (state.groupBy !== "none") {
    filtered.forEach(record => {
      const key = groupKeyFor(record);
      groupCounts.set(key, (groupCounts.get(key) || 0) + 1);
    });
  }

  const cardsHtml = [];
  let lastGroup = null;
  pageRows.forEach(record => {
    if (state.groupBy !== "none") {
      const key = groupKeyFor(record);
      if (key !== lastGroup) {
        lastGroup = key;
        const collapsed = state.collapsedGroups.has(key);
        const kindAttr = state.groupBy === "kind" ? ` data-gkind="${escapeHtml(record.kind || "")}"` : "";
        cardsHtml.push(`<div class="group-header"${kindAttr} data-group="${escapeHtml(key)}" tabindex="0" role="button"><span>${escapeHtml(key)}</span><span class="muted">${groupCounts.get(key) || 0}${t("unit.count")}${collapsed ? ` · ${t("group.collapsed")}` : ""}</span></div>`);
        if (collapsed) return;
      }
      if (state.collapsedGroups.has(key)) return;
    }
    cardsHtml.push(renderCard(record));
  });
  els.recordGrid.innerHTML = cardsHtml.join("") || `<div class="fallback">${t("grid.empty")}</div>`;
}

function renderCard(record) {
  const mark = markOf(record);
  const markChip = mark ? `<span class="mark-chip ${escapeHtml(mark.status)}">${escapeHtml(markLabel(mark.status))}</span>` : "";
  const staleTag = record.indexStatus === "stale" ? '<span class="muted">(stale)</span>' : "";
  const thumb = record.thumb
    ? `<img src="${escapeHtml(record.thumb)}" loading="lazy" decoding="async" fetchpriority="low">`
    : `<div class="ph">${record.status === "validation-failed" ? t("thumb.damaged") : t("thumb.missing")}</div>`;
  const recTypeTag = record.recType ? `<span class="rec-type">${escapeHtml(recTypeLabel(record.recType))}</span>` : "";
  const channel = record.channel ? `<span class="channel-badge">${escapeHtml(record.channel)}</span>` : "";
  const displayName = record.originalName && record.originalName !== record.name
    ? record.originalName
    : record.name;
  const recTime = record.recTime
    ? new Date(record.recTime * 1000).toLocaleString("ko-KR", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" })
    : "";
  const tagsHtml = tagListFor(record).length
    ? `<div class="tag-row">${tagListFor(record).map(tag => `<span class="tag-chip">${escapeHtml(tag)}</span>`).join("")}</div>`
    : "";
  const anomalyChip = record.hasAnomaly
    ? `<span class="badge anomaly" title="${escapeHtml(record.anomalies.map(item => item.kind).join(", "))}">${escapeHtml(t("header.anomaly"))}</span>`
    : "";
  const kindChip = `<span class="kind-badge kind-${escapeHtml(record.kind || "video")}" title="${t("tree.kind")}">${escapeHtml(KIND_SHORT[record.kind] || record.kind || "?")}</span>`;
  const warnChip = (record.warnings || []).length
    ? `<span class="badge warn" title="${escapeHtml(record.warnings.join("\n"))}">${tf("warn.count", { n: record.warnings.length })}</span>`
    : "";
  const dflChip = record.dfl && record.dfl.band
    ? `<span class="badge dfl dfl-${escapeHtml(record.dfl.band)}" title="${escapeHtml(tf("dfl.title", { score: record.dfl.score ?? "" }))}">${escapeHtml(tf("dfl.badge", { band: record.dfl.band_label || record.dfl.band }))}</span>`
    : (record.dfl && record.dfl.error
      ? `<span class="badge dfl dfl-low" title="${escapeHtml(tf("dfl.failTitle", { error: record.dfl.error }))}">${escapeHtml(t("dfl.fail"))}</span>`
      : "");
  return `<div class="card ${record.id === state.activeId ? "active" : ""}" data-id="${escapeHtml(record.id)}" tabindex="${record.id === state.activeId ? 0 : -1}" role="option" aria-selected="${state.selectedIds.has(record.id)}">
    <div class="thumb">${thumb}<input type="checkbox" aria-label="${escapeHtml(t("aria.select"))}" ${state.selectedIds.has(record.id) ? "checked" : ""} data-check="${escapeHtml(record.id)}">${recTypeTag}${kindChip}<span class="dur">${fmtDuration(record.duration)}</span></div>
    <div class="meta">
      <div class="name-row"><span class="name" title="${escapeHtml(displayName)}">${highlightEscape(displayName, state.query)}</span>${channel ? `<span class="channel-badge">${escapeHtml(record.channel)}</span>` : ""}</div>
      <div class="time-row"><span class="time-text">${recTime ? escapeHtml(recTime) : t("time.unknown")}</span><span class="badge ${statusClass(record.status)}" title="${escapeHtml(record.status)}">${escapeHtml(statusLabel(record.status))}</span>${anomalyChip}${warnChip}${dflChip}</div>
      ${tagsHtml}
      <div class="sub-row"><span class="sub">${fmtBytes(record.size)}${markChip}${staleTag}</span></div>
    </div>
  </div>`;
}

function renderHistogram(filtered) {
  const days = new Map();
  filtered.forEach(record => {
    if (record.recDay) days.set(record.recDay, (days.get(record.recDay) || 0) + 1);
  });
  const top = [...days.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).slice(-16);
  const max = Math.max(1, ...top.map(entry => entry[1]));
  els.dayHistogram.innerHTML = top.map(([day, count]) => `
    <button type="button" class="${state.dateFrom === day && state.dateTo === day ? "selected" : ""}" data-day="${escapeHtml(day)}" title="${escapeHtml(tf("hist.item", { day, count }))}" aria-label="${escapeHtml(tf("hist.item", { day, count }))}">
      <span class="bar" style="height:${Math.round((count * 40) / max)}px"></span>
      <span class="lbl">${escapeHtml(day.slice(5))}</span>
    </button>`).join("")
    || `<span class="muted">${t("hist.empty")}</span>`;
  els.dayHistogram.querySelectorAll("button[data-day]").forEach(button => {
    button.addEventListener("click", () => {
      state.dateFrom = button.dataset.day;
      state.dateTo = button.dataset.day;
      els.dateFrom.value = state.dateFrom;
      els.dateTo.value = state.dateTo;
      state.currentPage = 1;
      render();
    });
  });
  const known = records.filter(record => record.recDay).map(record => record.recDay).sort();
  els.periodLabel.textContent = known.length
    ? tf("hist.range", { from: known[0], to: known[known.length - 1], known: known.length, total: records.length })
    : t("hist.none");
}

function tagListFor(record) { return state.tags[record.id] || []; }

function saveTagPresets() { storageSet(TAG_PRESETS_KEY, state.tagPresets); }
// renderDetails/openPlayerWindow call tagPresets() for the preset list —
// the array itself lives on state (initialized with DEFAULT_TAG_PRESETS).
function tagPresets() { return state.tagPresets || DEFAULT_TAG_PRESETS; }
function addTagPreset(tag) {
  const name = String(tag || "").trim();
  if (!name || state.tagPresets.includes(name)) return;
  state.tagPresets.push(name);
  saveTagPresets();
}
function removeTagPreset(tag) {
  const idx = state.tagPresets.indexOf(tag);
  if (idx < 0) return;
  state.tagPresets.splice(idx, 1);
  saveTagPresets();
}

function addTag(id, tag) {
  const list = state.tags[id] || [];
  if (!list.includes(tag)) list.push(tag);
  state.tags[id] = list;
  touchAnnotation(id);
  storageSet(TAGS_KEY, state.tags);
}

function removeTag(id, tag) {
  const list = state.tags[id] || [];
  const idx = list.indexOf(tag);
  if (idx >= 0) list.splice(idx, 1);
  state.tags[id] = list;
  touchAnnotation(id);
  storageSet(TAGS_KEY, state.tags);
}

function toggleTag(id, tag) {
  const list = state.tags[id] || [];
  if (list.includes(tag)) removeTag(id, tag);
  else addTag(id, tag);
}

function markLabel(status) {
  if (status === "reviewed") return t("mark.reviewed");
  if (status === "important") return t("mark.important");
  if (status === "needs_verification") return t("mark.verify");
  if (status === "noted") return t("mark.noted");
  return status;
}

function indexOfFiltered(id) {
  return filteredRecords().findIndex(record => record.id === id);
}

function selectRange(fromId, toId, checked) {
  const filtered = filteredRecords();
  const from = filtered.findIndex(record => record.id === fromId);
  const to = filtered.findIndex(record => record.id === toId);
  if (from < 0 || to < 0) return;
  const [low, high] = from <= to ? [from, to] : [to, from];
  for (let index = low; index <= high; index += 1) {
    if (checked) state.selectedIds.add(filtered[index].id);
    else state.selectedIds.delete(filtered[index].id);
  }
}

function selectAllFiltered() {
  filteredRecords().forEach(record => state.selectedIds.add(record.id));
  render();
  toast(tf("toast.selectedAll", { n: state.selectedIds.size }));
}

function clearSelection() {
  state.selectedIds.clear();
  render();
}

function targetIds() {
  if (state.selectedIds.size) return [...state.selectedIds];
  return state.activeId ? [state.activeId] : [];
}

function applyMark(status) {
  const ids = targetIds();
  if (!ids.length) { toast(t("toast.selectTarget")); return; }
  const stamped = Math.floor(Date.now() / 1000);
  ids.forEach(id => {
    if (status === null) { delete state.marks[id]; state.notes[id] = ""; }
    else state.marks[id] = { status, marked_unix: stamped };
    // touchAnnotation must run after the mutation — it snapshots the
    // resulting state into the draft that hydration replays on reload.
    touchAnnotation(id);
  });
  storageSet(MARKS_KEY, state.marks);
  render();
  toast(tf("toast.markApplied", { n: ids.length, status: status === null ? t("mark.cleared") : markLabel(status) }));
}

let mediaRenderedFor = null;

function drawTelemetry(report) {
  const canvas = document.getElementById("telemetryCanvas");
  const note = document.getElementById("telemetryNote");
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const pts = (report.points || []).filter(p => Number.isFinite(p.lat) && Number.isFinite(p.lon));
  if (pts.length < 2) {
    note.textContent = report.note || t("telemetry.noPoints");
    return;
  }
  // Relative track sketch — no basemap: the viewer stays fully offline.
  const lats = pts.map(p => p.lat), lons = pts.map(p => p.lon);
  const [minLat, maxLat] = [Math.min(...lats), Math.max(...lats)];
  const [minLon, maxLon] = [Math.min(...lons), Math.max(...lons)];
  const pad = 8;
  const s = Math.min((canvas.width - 2 * pad) / Math.max(maxLon - minLon, 1e-9),
                     (canvas.height - 2 * pad) / Math.max(maxLat - minLat, 1e-9));
  const X = p => pad + (p.lon - minLon) * s;
  const Y = p => canvas.height - pad - (p.lat - minLat) * s;
  const stride = Math.max(1, Math.floor(pts.length / 4000));
  ctx.strokeStyle = "#0f7c71";
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  for (let i = 0; i < pts.length; i += stride) {
    i === 0 ? ctx.moveTo(X(pts[i]), Y(pts[i])) : ctx.lineTo(X(pts[i]), Y(pts[i]));
  }
  ctx.stroke();
  ctx.fillStyle = "#2ea043";
  ctx.beginPath(); ctx.arc(X(pts[0]), Y(pts[0]), 3.5, 0, 7); ctx.fill();
  ctx.fillStyle = "#c0392b";
  const last = pts[pts.length - 1];
  ctx.beginPath(); ctx.arc(X(last), Y(last), 3.5, 0, 7); ctx.fill();
  const speed = Number.isFinite(report.max_speed_kmh) ? ` · ${tf("telemetry.maxSpeed", { v: report.max_speed_kmh.toFixed(0) })}` : "";
  note.textContent = tf("telemetry.summary", { n: pts.length }) + speed;
}

function renderDetails() {
  const record = selectedRecord();
  const teleTitle = document.getElementById("telemetryTitle");
  const telePane = document.getElementById("telemetryPane");
  if (!record) {
    els.mediaStage.innerHTML = `<div class="fallback">${t("empty.index")}</div>`;
    els.mediaTitle.textContent = "-";
    els.mediaStatus.textContent = "-";
    els.summaryList.innerHTML = "";
    els.metaList.innerHTML = "";
    els.validationList.innerHTML = "";
    els.detailBadges.innerHTML = "";
    if (teleTitle) teleTitle.hidden = true;
    if (telePane) telePane.hidden = true;
    mediaRenderedFor = null;
    return;
  }
  els.mediaTitle.textContent = record.originalName || record.name || record.id;
  els.mediaStatus.textContent = record.status;
  els.mediaStatus.className = `badge ${statusClass(record.status)}`;
  const dualMates = state.layout.dual ? matesOf(record) : [];
  const mediaKey = `${record.id}:${record.fileUrl}:${state.proxies[record.id] || ""}:dual:${state.layout.dual ? dualMates.map(m => m.id).join(",") : "off"}`;
  if (mediaRenderedFor !== mediaKey) {
    const mediaSrc = mediaSrcFor(record);
    if (mediaSrc && dualMates.length) {
      els.mediaStage.innerHTML = `<div class="dual-stage">${[record, ...dualMates].map(r =>
        `<div class="dual-pane"><video controls preload="metadata" src="${escapeHtml(mediaSrcFor(r))}"></video>` +
        `<span class="dual-label">${escapeHtml(`${channelLabel(channelCodeOf(r))} · ${r.originalName || r.name || r.id}`)}</span></div>`
      ).join("")}</div>`;
      wireDualSync(els.mediaStage.querySelectorAll("video"));
    } else {
      els.mediaStage.innerHTML = mediaSrc
        ? `<video controls preload="metadata" src="${escapeHtml(mediaSrc)}"></video>`
        : `<div class="fallback">${t("empty.play")}</div>`;
    }
    els.mediaStage.querySelectorAll("video").forEach(v => {
      v.playbackRate = state.layout.rate || 1;
      v.addEventListener("loadedmetadata", applyVideoScale);
    });
    mediaRenderedFor = mediaKey;
  }
  applyVideoScale();
  updateRangeLabel();
  const mark = markOf(record);
  const recordTags = tagListFor(record);
  if (teleTitle && telePane) {
    teleTitle.hidden = !mediaSrcFor(record);
    const tele = record.telemetry;
    if (tele && Array.isArray(tele.points) && tele.points.length) {
      telePane.hidden = false;
      drawTelemetry(tele);
    } else {
      telePane.hidden = true;
    }
  }
  document.getElementById("detailBadges").innerHTML = [
    `<span class="badge ${statusClass(record.status)}">${escapeHtml(statusLabel(record.status))}</span>`,
    (record.warnings || []).length ? `<span class="badge warn" title="${escapeHtml(record.warnings.join("\n"))}">${tf("warn.count", { n: record.warnings.length })}</span>` : "",
    record.hasAnomaly ? `<span class="badge anomaly">${escapeHtml(t("header.anomaly"))}</span>` : "",
    record.dfl && record.dfl.band ? `<span class="badge dfl dfl-${escapeHtml(record.dfl.band)}">${escapeHtml(tf("dfl.badge", { band: record.dfl.band_label || record.dfl.band }))}</span>` : "",
    mark ? `<span class="mark-chip ${escapeHtml(mark.status)}">${escapeHtml(markLabel(mark.status))}</span>` : "",
    ...recordTags.map(tag => `<span class="tag-chip">${escapeHtml(tag)}</span>`),
    record.indexStatus === "stale" ? '<span class="muted">stale</span>' : ""
  ].join(" ");
  els.summaryList.innerHTML = [
    [t("detail.recTime"), record.recTime ? fmtUnix(record.recTime) + (record.recSource === "name" ? "" : " " + t("detail.est")) : t("detail.unknown")],
    [t("detail.channel"), record.channel || "-"],
    [t("detail.mark"), mark ? markLabel(mark.status) : t("detail.unmarked")],
    [t("detail.len"), fmtDuration(record.duration)],
    [t("detail.size"), fmtBytes(record.size)],
    [t("detail.anomaly"), record.hasAnomaly ? record.anomalies.map(item => item.kind).join(", ") : "-"],
    [t("detail.dfl"), record.dfl ? (record.dfl.band
        ? tf("detail.dflScore", { label: record.dfl.band_label || record.dfl.band, score: record.dfl.score ?? "?" }) + (record.dfl.signal_titles?.length ? ": " + record.dfl.signal_titles.slice(0, 3).join(", ") : "")
        : (record.dfl.error ? tf("detail.dflFail", { error: record.dfl.error }) : t("detail.dflNone"))) : "-"]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${escapeHtml(v)}</dd>`).join("");
  els.metaList.innerHTML = [
    [t("detail.original"), `<code>${escapeHtml(record.originalName || record.name)}</code>`],
    [t("detail.origPath"), record.originalPath ? `<code>${escapeHtml(record.originalPath)}</code>` : "-"],
    [t("detail.path"), `<code>${escapeHtml(record.path)}</code>`],
    [t("detail.codec"), escapeHtml(record.codec)],
    ["SHA-256", `<code>${escapeHtml(record.sha256)}</code>`],
    [t("detail.offset"), record.offset != null ? String(record.offset) : "-"]
  ].map(([k, v]) => `<dt>${escapeHtml(k)}</dt><dd>${v}</dd>`).join("");
  // tag editor for the selected evidence
  const tagEditorHtml = `<div class="tag-editor">
    ${tagPresets().map(preset => `<button type="button" class="tag-btn ${recordTags.includes(preset) ? "on" : ""}" data-preset-tag="${escapeHtml(preset)}">${escapeHtml(preset)}</button>`).join("")}
    <input type="text" class="tag-input" id="customTagInput" placeholder="${t("tag.input")}" style="width:80px;height:24px;font-size:11px;">
    <button type="button" class="mini" id="btnAddCustomTag">${t("tag.apply")}</button>
    <button type="button" class="mini" id="btnSaveCustomTag" title="${t("tag.presetSave.title")}">${t("tag.presetSave")}</button>
  </div>`;
  // replace existing tag editor if any
  const oldEditor = document.querySelector(".tag-editor-wrap");
  if (oldEditor) oldEditor.remove();
  const metaEl = els.metaList;
  metaEl.insertAdjacentHTML("afterend", `<div class="tag-editor-wrap">${tagEditorHtml}
    <textarea id="evidenceNote" class="note-input" rows="2" placeholder="${t("note.placeholder")}">${escapeHtml(state.notes[record.id] || "")}</textarea>
  </div>`);
  document.getElementById("evidenceNote").addEventListener("input", e => {
    const value = e.target.value;
    if (value.trim()) state.notes[record.id] = value;
    else state.notes[record.id] = "";
    touchAnnotation(record.id);
    storageSet(NOTES_KEY, state.notes);
  });
  document.querySelectorAll(".tag-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      toggleTag(record.id, btn.dataset.presetTag);
      render();
    });
  });
  const customInput = document.getElementById("customTagInput");
  const customBtn = document.getElementById("btnAddCustomTag");
  if (customInput && customBtn) {
    const addCustom = () => {
      const tag = customInput.value.trim();
      if (!tag) return;
      addTag(record.id, tag);
      customInput.value = "";
      render();
      document.getElementById("customTagInput")?.focus();
    };
    customBtn.addEventListener("click", addCustom);
    customInput.addEventListener("keydown", e => { if (e.key === "Enter") addCustom(); });
    const savePresetBtn = document.getElementById("btnSaveCustomTag");
    if (savePresetBtn) {
      savePresetBtn.addEventListener("click", () => {
        const tag = customInput.value.trim();
        if (!tag) return;
        addTagPreset(tag);
        customInput.value = "";
        render();
        toast(tf("toast.presetAdded", { tag }));
      });
    }
  }
  const related = validationLog.filter(item => normalizePath(item.target_path) === normalizePath(record.path) || item.selector === record.id);
  const anomalyRelated = (record.anomalies || []).map(item => `<div class="validation-item">
    <strong>${escapeHtml(item.kind || "anomaly")}</strong>
    <div class="muted">${escapeHtml(item.detail || "")}</div>
    <code>candidate-finding</code>
  </div>`);
  const warningItems = (record.warnings || []).map(warning => `<div class="validation-item warn">
    <strong>${t("warn.title")}</strong>
    <div class="muted">${escapeHtml(warning)}</div>
    <code>recovery-warning</code>
  </div>`);
  els.validationList.innerHTML = [
    ...warningItems,
    ...related.map(item => `<div class="validation-item">
    <strong>${escapeHtml(item.validation_status || "-")}</strong>
    <div class="muted">${escapeHtml(item.validation_note || item.ffprobe_error || "-")}</div>
    <code>${escapeHtml(item.target_sha256 || "-")}</code>
  </div>`),
    ...anomalyRelated
  ].join("") || `<div class="validation-item">${t("validation.none")}</div>`;
}

function renderTree() {
  const typeCounts = new Map();
  const kindCounts = new Map();
  const dayCounts = new Map();
  records.forEach(record => {
    const typeKey = record.recType || "unclassified";
    typeCounts.set(typeKey, (typeCounts.get(typeKey) || 0) + 1);
    kindCounts.set(record.kind, (kindCounts.get(record.kind) || 0) + 1);
    if (record.recDay) dayCounts.set(record.recDay, (dayCounts.get(record.recDay) || 0) + 1);
  });
  const kindLabels = { video: t("filter.kind.video"), carved: t("filter.kind.carved"), filesystem: t("filter.kind.filesystem"), candidate: t("filter.kind.candidate") };
  const items = [];
  const addItems = (title, entries, activeKey, onPick) => {
    items.push({ header: title });
    items.push({ label: t("tree.all"), count: null, key: "", pick: () => onPick(""), active: activeKey === "" });
    [...entries.entries()].sort((a, b) => (a[0] < b[0] ? -1 : 1)).forEach(([key, count]) => {
      items.push({ label: key, count, key, pick: () => onPick(key), active: key === activeKey });
    });
  };
  addItems(t("tree.rectype"), new Map([...typeCounts.entries()].map(([k, c]) => [recTypeLabel(k), c])),
    state.recType ? recTypeLabel(state.recType) : "", label => {
      const match = [...typeCounts.entries()].find(([k]) => recTypeLabel(k) === label);
      state.recType = match ? match[0] : "";
      state.currentPage = 1;
      render();
    });
  addItems(t("tree.kind"), new Map([...kindCounts.entries()].map(([k, c]) => [kindLabels[k] || k, c])),
    kindLabels[state.kind] || "", label => {
      state.kind = Object.entries(kindLabels).find(([, kindLabel]) => kindLabel === label)?.[0] || "";
      els.kind.value = state.kind;
      state.currentPage = 1;
      render();
    });
  addItems(t("tree.date"), dayCounts, state.dateFrom && state.dateFrom === state.dateTo ? state.dateFrom : "", day => {
    state.dateFrom = day;
    state.dateTo = day;
    els.dateFrom.value = day;
    els.dateTo.value = day;
    state.currentPage = 1;
    render();
  });
  els.facetTree.innerHTML = items.map(item => item.header !== undefined
    ? `<div class="tree-sec">${escapeHtml(item.header)}</div>`
    : `<div class="tree-item${item.active ? " active" : ""}" data-pick="${escapeHtml(item.label)}" tabindex="0" role="button"><span>${escapeHtml(item.label)}</span>${item.count != null ? `<span class="muted">${item.count}</span>` : ""}</div>`
  ).join("");
  const pickers = items.filter(item => item.pick);
  const nodes = els.facetTree.querySelectorAll(".tree-item");
  let idx = 0;
  nodes.forEach(node => {
    const entry = pickers[idx];
    if (!entry) return;
    idx += 1;
    node.addEventListener("click", () => entry.pick(node.dataset.pick));
    node.addEventListener("keydown", e => {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); entry.pick(node.dataset.pick); }
    });
  });
}

function renderMetrics() {
  els.caseLine.textContent = `${manifest.case_id || "case"} · ${manifest.title || "Untitled"} · ${scan.source_path || "-"}`;
  els.metricVideos.textContent = tf("metric.indexed", { n: videos.length });
  els.metricCarved.textContent = carveLog.length + recoveredFilesystemLog.length;
  els.metricVerified.textContent = records.filter(record => record.status === "ffprobe-video-stream-confirmed" || record.status === "ffprobe-confirmed").length;
  els.metricFailed.textContent = records.filter(record => record.status === "validation-failed").length;
  if (els.metricAnomaly) {
    els.metricAnomaly.textContent = String(records.filter(record => record.hasAnomaly).length);
  }
  const markCount = Object.keys(state.marks).length;
  els.selectionCount.textContent = tf("sel.count", { n: state.selectedIds.size, m: markCount });
  if (els.caseWarnings) {
    if (inspectionWarnings.length) {
      els.caseWarnings.hidden = false;
      els.caseWarnings.innerHTML = `<b>${tf("warn.inspect", { n: inspectionWarnings.length })}</b>${inspectionWarnings.map(escapeHtml).join(" · ")}`;
    } else {
      els.caseWarnings.hidden = true;
    }
  }
}

function renderChips() {
  // Status/mark chips stay inline; tag filters live in the 정렬·표시
  // panel's tag select so the chip row stays a single group.
  const parts = [`<span class="chip-label">${t("filter.label")}</span>`];
  for (const [value, label] of PRESET_CHIPS) {
    if (value.startsWith("tag:")) continue;
    parts.push(`<button type="button" class="chip ${state.chip === value ? "active" : ""}" data-chip="${escapeHtml(value)}">${escapeHtml(t(label))}</button>`);
  }
  els.presetChips.innerHTML = parts.join("");
  els.presetChips.querySelectorAll(".chip").forEach(chip => {
    chip.addEventListener("click", () => {
      const value = chip.dataset.chip;
      state.chip = value;
      if (value.startsWith("status:")) {
        state.status = value.slice(7);
        els.status.value = state.status;
      } else {
        state.status = "";
        els.status.value = "";
      }
      state.currentPage = 1;
      render();
    });
  });
  // Tag filter options follow the editable preset list plus any tags
  // already applied in this case that aren't presets (custom tags).
  const appliedTags = new Set();
  Object.values(state.tags).forEach(list => (list || []).forEach(tag => appliedTags.add(tag)));
  const filterTags = [...state.tagPresets, ...[...appliedTags].filter(tag => !state.tagPresets.includes(tag))];
  els.tagFilter.innerHTML = `<option value="">${t("filter.tag.all")}</option>`
    + filterTags.map(tag => `<option value="tag:${escapeHtml(tag)}">${escapeHtml(tf("filter.tag.named", { tag }))}</option>`).join("");
  els.tagFilter.value = state.chip.startsWith("tag:") ? state.chip : "";
}

function render() {
  applyChromeI18n();
  // Filter+sort once per render — grid and histogram share the result.
  const filtered = filteredRecords();
  renderMetrics();
  renderChips();
  renderHistogram(filtered);
  renderTree();
  renderGrid(filtered);
  renderDetails();
}

function setupGridDelegation() {
  if (els.recordGrid._delegated) return;
  els.recordGrid._delegated = true;
  let shiftRange = false;
  els.recordGrid.addEventListener("click", event => {
    const header = event.target.closest(".group-header");
    if (header) {
      const key = header.dataset.group;
      if (state.collapsedGroups.has(key)) state.collapsedGroups.delete(key);
      else state.collapsedGroups.add(key);
      renderGrid(filteredRecords());
      return;
    }
    if (event.target.closest("input[type='checkbox']")) return;
    const card = event.target.closest(".card");
    if (!card) return;
    state.activeId = card.dataset.id;
    render();
  });
  els.recordGrid.addEventListener("keydown", event => {
    if (event.target.closest("input")) return;
    const header = event.target.closest(".group-header");
    if (header && (event.key === "Enter" || event.key === " ")) {
      event.preventDefault();
      event.stopPropagation();
      const key = header.dataset.group;
      if (state.collapsedGroups.has(key)) state.collapsedGroups.delete(key);
      else state.collapsedGroups.add(key);
      renderGrid(filteredRecords());
      return;
    }
    const card = event.target.closest(".card");
    if (!card) return;
    if (event.key === "Enter") {
      event.stopPropagation();
      state.activeId = card.dataset.id;
      render();
      return;
    }
    if (event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      if (state.selectedIds.has(card.dataset.id)) state.selectedIds.delete(card.dataset.id);
      else state.selectedIds.add(card.dataset.id);
      render();
      return;
    }
    const step = { ArrowDown: 1, ArrowRight: 1, ArrowUp: -1, ArrowLeft: -1 }[event.key];
    if (step) {
      event.preventDefault();
      event.stopPropagation();
      state.activeId = card.dataset.id;
      moveActive(step);
      els.recordGrid.querySelector(`.card[data-id="${CSS.escape(state.activeId)}"]`)?.focus();
    }
  });
  els.recordGrid.addEventListener("click", event => {
    const box = event.target.closest("input[type='checkbox']");
    if (!box) return;
    event.stopPropagation();
    shiftRange = event.shiftKey && !!state.lastCheckedKey;
  }, true);
  els.recordGrid.addEventListener("change", event => {
    const box = event.target.closest("input[type='checkbox']");
    if (!box) return;
    const id = box.dataset.check;
    if (shiftRange) {
      selectRange(state.lastCheckedKey, id, box.checked);
    } else if (box.checked) {
      state.selectedIds.add(id);
    } else {
      state.selectedIds.delete(id);
    }
    state.lastCheckedKey = id;
    shiftRange = false;
    render();
  });
}

function applyTag(tag) {
  const ids = targetIds();
  if (!ids.length) { toast(t("toast.selectTarget")); return; }
  ids.forEach(id => {
    const list = state.tags[id] || [];
    if (!list.includes(tag)) list.push(tag);
    state.tags[id] = list;
    touchAnnotation(id);
  });
  storageSet(TAGS_KEY, state.tags);
  render();
  toast(tf("toast.tagApplied", { n: ids.length, tag }));
}

function clearTags() {
  const ids = targetIds();
  if (!ids.length) { toast(t("toast.selectTarget")); return; }
  ids.forEach(id => { delete state.tags[id]; touchAnnotation(id); });
  storageSet(TAGS_KEY, state.tags);
  render();
  toast(tf("toast.tagCleared", { n: ids.length }));
}

function downloadJSON(filename, payload) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 5000);
  toast(tf("toast.downloadStart", { name: filename }));
}

function downloadText(filename, text, mime) {
  const blob = new Blob([text], { type: mime || "text/plain;charset=utf-8" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(link.href), 5000);
  toast(tf("toast.downloadStart", { name: filename }));
}

function csvCell(value) {
  const text = String(value ?? "");
  return /[",\n\r]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function isoTime(unix) {
  return unix ? new Date(unix * 1000).toISOString() : "";
}

const KIND_LABELS = {
  video: t("filter.kind.video"),
  carved: t("filter.kind.carved"),
  filesystem: t("filter.kind.filesystem"),
  candidate: t("filter.kind.candidate")
};
const KIND_SHORT = {
  video: t("kind.short.video"),
  carved: t("kind.short.carved"),
  filesystem: t("kind.short.filesystem"),
  candidate: t("kind.short.candidate")
};

function exportItem(record) {
  return {
    id: record.id,
    name: record.name || "",
    path: record.path || "",
    kind: record.kind || "",
    status: record.status || "",
    mark: state.marks[record.id]?.status || "",
    tags: state.tags[record.id] || [],
    warnings: record.warnings || [],
    sha256: record.sha256 && record.sha256 !== "-" ? record.sha256 : "",
    rec_time: isoTime(record.recTime),
    original_path: record.originalPath || "",
    note: record.note && record.note !== "-" ? record.note : ""
  };
}

function selectedRecords() {
  return records.filter(record => state.selectedIds.has(record.id));
}

function copyText(text, label) {
  const done = () => toast(tf("toast.copied", { label }));
  const fallback = () => {
    const area = document.createElement("textarea");
    area.value = text;
    document.body.appendChild(area);
    area.select();
    try {
      document.execCommand("copy");
      done();
    } catch (error) {
      toast(tf("toast.copyFail", { err: error.message }));
    }
    area.remove();
  };
  if (navigator.clipboard?.writeText) {
    navigator.clipboard.writeText(text).then(done, fallback);
  } else {
    fallback();
  }
}

function toggleTheater() {
  state.layout.theater = !state.layout.theater;
  saveLayout();
  applyLayout();
}

function toggleFullscreen() {
  const stage = document.getElementById("mediaStage");
  if (document.fullscreenElement) {
    document.exitFullscreen().catch(error => toast(tf("toast.fsExitFail", { err: error.message })));
    return;
  }
  if (stage.requestFullscreen) {
    stage.requestFullscreen().catch(error => toast(tf("toast.fsEnterFail", { err: error.message })));
  } else {
    toast(t("toast.fsUnsupported"));
  }
}

async function togglePip() {
  const video = els.mediaStage.querySelector("video");
  if (!video) { toast(t("toast.noVideo")); return; }
  try {
    if (document.pictureInPictureElement) {
      await document.exitPictureInPicture();
    } else if (video.requestPictureInPicture) {
      await video.requestPictureInPicture();
    } else {
      toast(t("toast.pipUnsupported"));
    }
  } catch (error) {
    toast(tf("toast.pipFail", { err: error.message }));
  }
}

// --- media transport: skip / playback rate / popup player ---

const RATES = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 4];

function currentVideo() {
  return els.mediaStage.querySelector("video");
}

function skipVideo(delta) {
  const video = currentVideo();
  if (!video) return;
  video.currentTime = Math.max(0, Math.min(video.duration || 1e9, video.currentTime + delta));
}

function setRate(rate) {
  state.layout.rate = rate;
  saveLayout();
  els.playRate.value = String(rate);
  const video = currentVideo();
  if (video) video.playbackRate = rate;
}

function rateStep(delta) {
  const index = RATES.indexOf(Number(state.layout.rate) || 1);
  const next = RATES[Math.max(0, Math.min(RATES.length - 1, (index < 0 ? 3 : index) + delta))];
  setRate(next);
}

// Descriptor handed to the popup player so it can render the record the
// main window considers active — the popup never owns review state.
function playerDescriptor() {
  const record = selectedRecord();
  if (!record) return null;
  return {
    id: record.id,
    name: record.originalName || record.name || record.id,
    status: record.status,
    statusLabel: statusLabel(record.status),
    src: mediaSrcFor(record),
    mark: state.marks[record.id]?.status || "",
    tags: tagListFor(record),
    warnings: record.warnings || [],
    index: indexOfFiltered(record.id) + 1,
    total: filteredRecords().length,
  };
}

// Same-origin popup windows drive review through this bridge — marks and
// tags flow through the same state/localStorage path as the main viewer,
// so popup actions are identical to clicking the buttons there.
window.__ftBridge = {
  descriptor: () => playerDescriptor(),
  focus(id) {
    if (state.activeId !== id && records.some(record => record.id === id)) {
      state.activeId = id;
      render();
    }
    return playerDescriptor();
  },
  navigate(step) {
    const before = state.activeId;
    moveActive(step);
    return { moved: state.activeId !== before, descriptor: playerDescriptor() };
  },
  // markActive applies the mark and advances — the popup treats the
  // returned descriptor as the next clip to play.
  mark(status) {
    markActive(status);
    return playerDescriptor();
  },
  toggleTag(tag) {
    if (state.activeId) {
      toggleTag(state.activeId, tag);
      render();
    }
    return playerDescriptor();
  },
  // Filtered list for the popup's quick-jump sidebar — mirrors the main
  // grid order so popup position always matches what the examiner sees.
  list() {
    return filteredRecords().map((r, i) => ({
      id: r.id,
      name: r.originalName || r.name || r.id,
      index: i + 1,
      mark: state.marks[r.id]?.status || "",
      markLabel: state.marks[r.id]?.status ? markLabel(state.marks[r.id].status) : "",
      hasVideo: !!mediaSrcFor(r),
    }));
  },
};

// BroadcastChannel mirror of __ftBridge: a popup bound to this case keeps
// working even when window.opener was severed (browser popup policies,
// parent navigation), because the channel is name-addressed rather than
// object-referenced. Requests carry a correlation seq; replies echo it.
const playerChannel = ("BroadcastChannel" in window)
  ? new BroadcastChannel(`ft-player:${manifest.case_id || "case"}`)
  : null;
if (playerChannel) {
  playerChannel.addEventListener("message", (ev) => {
    const m = ev.data || {};
    const reply = (payload) => playerChannel.postMessage({ seq: m.seq, payload });
    if (m.type === "descriptor") reply(playerDescriptor());
    else if (m.type === "focus") reply(window.__ftBridge.focus(m.arg));
    else if (m.type === "navigate") reply(window.__ftBridge.navigate(m.arg));
    else if (m.type === "mark") reply(window.__ftBridge.mark(m.arg));
    else if (m.type === "toggleTag") reply(window.__ftBridge.toggleTag(m.arg));
    else if (m.type === "list") reply(window.__ftBridge.list());
  });
}

// A minimal dedicated player window — the examiner can park the clip on
// a second monitor and keep triaging: marks, tags, and next/previous
// navigation all route back through __ftBridge.
function openPlayerWindow() {
  const record = selectedRecord();
  if (!record) { toast(t("toast.noRecord")); return; }
  const win = window.open("", "frametrace-player", "popup=yes,width=1100,height=880");
  if (!win) { toast(t("toast.popupBlocked")); return; }
  const e = escapeHtml;
  const rate = state.layout.rate || 1;
  const rateOptions = RATES.map(r => `<option value="${r}"${r === rate ? " selected" : ""}>${r}×</option>`).join("");
  const tagButtons = tagPresets().map(tag => `<button class="tg" data-tag="${e(tag)}">${e(tag)}</button>`).join("");
  win.document.open();
  win.document.write(`<!doctype html><html lang="${state.locale}"><head><meta charset="utf-8">
<title>${e(record.name || record.id)} — FrameTrace Player</title>
<style>
body{margin:0;background:#101815;color:#dfe8e4;font-family:ui-sans-serif,system-ui,"Segoe UI",sans-serif;display:flex;flex-direction:column;height:100vh}
header{padding:8px 14px;font-size:13px;display:flex;gap:10px;align-items:center;border-bottom:1px solid #24332e;flex-wrap:wrap}
header .id{color:#8fa79e;font-family:monospace;font-size:11px}
#wrap{flex:1;min-height:0;display:flex}
#left{flex:1;min-width:0;display:flex;flex-direction:column}
#stage{flex:1;min-height:0;display:grid;place-items:center;background:#000;position:relative}
video{width:100%;height:100%;object-fit:contain}
#novid{position:absolute;color:#8fa79e;font-size:13px;background:rgba(0,0,0,.6);padding:8px 14px;border-radius:6px}
#list{width:230px;flex:0 0 auto;overflow-y:auto;border-left:1px solid #24332e;font-size:12px}
.li{padding:6px 10px;cursor:pointer;border-bottom:1px solid #1c2825;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.li:hover{background:#1b2b26}
.li:focus-visible{outline:2px solid #3fa08e;outline-offset:-2px}
.li.on{background:#3fa08e;color:#08130f;font-weight:700}
.li.novid{color:#5a6b64}
.li .idx{color:#8fa79e;margin-right:4px;font-family:monospace;font-size:11px}
.li.on .idx{color:#0a1c18}
.li .mk{float:right;color:#d9b45b}
.li.on .mk{color:#08130f}
.bar{display:flex;gap:8px;align-items:center;padding:6px 14px;border-top:1px solid #24332e;font-size:13px;flex-wrap:wrap}
button,select{font:inherit;height:28px;border:1px solid #3a4f48;border-radius:5px;background:#1b2b26;color:#dfe8e4;padding:0 10px;cursor:pointer;font-size:12px}
button:hover,select:hover{border-color:#3fa08e}
button.on{background:#3fa08e;border-color:#3fa08e;color:#08130f}
.muted{color:#8fa79e;font-size:12px}
#note{color:#d9b45b;font-size:12px}
</style></head><body>
<header><strong id="title"></strong><span class="id" id="meta"></span><span id="note"></span></header>
<div id="wrap"><div id="left">
<div id="stage"><video id="vv" controls autoplay></video><div id="novid" hidden>${t("player.novid")}</div></div>
<div class="bar">
<button id="prev">${t("player.prev")}</button>
<button id="b10">« -10s</button><button id="b1">-1s</button>
<button id="f1">+1s</button><button id="f10">+10s »</button>
<select id="rate">${rateOptions}</select>
<button id="play">${t("player.play")}</button>
<button id="next">${t("player.next")}</button>
<button id="cap" title="${t("player.capTitle")}">${t("player.cap")}</button>
<span class="muted">${t("player.hint")}</span>
</div>
<div class="bar">
<span class="muted">${t("player.mark")}</span>
<button class="mk" data-m="reviewed">${t("player.reviewed")}</button>
<button class="mk" data-m="important">${t("player.important")}</button>
<button class="mk" data-m="needs_verification">${t("player.verify")}</button>
<button class="mk" data-m="">${t("player.clear")}</button>
<span class="muted">${t("player.tags")}</span>${tagButtons}
</div>
</div><div id="list" role="listbox" aria-label="${t("player.list")}"></div></div>
<script>
var v=document.getElementById("vv");
var RATES=[${RATES.join(",")}];
var rate=document.getElementById("rate");
var cur=null;
v.playbackRate=${rate};
var seqN=0,pend={},chan=("BroadcastChannel" in window)?new BroadcastChannel(${JSON.stringify(`ft-player:${manifest.case_id || "case"}`)}):null;
if(chan){chan.onmessage=function(ev){var m=ev.data||{},f=m.seq&&pend[m.seq];if(f){delete pend[m.seq];f(m.payload);}};}
function call(op,arg,cb){
var b=(window.opener&&!window.opener.closed&&window.opener.__ftBridge)||null;
if(b){try{cb(b[op](arg));}catch(e){cb(undefined);}return;}
if(!chan){cb(undefined);return;}
var s=++seqN;pend[s]=cb;
chan.postMessage({type:op,seq:s,arg:arg});
setTimeout(function(){if(pend[s]){delete pend[s];cb(undefined);}},3000);
}
function esc(s){return String(s==null?"":s).replace(/[&<>"']/g,function(c){return{"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]});}
function note(t){document.getElementById("note").textContent=t||"";}
function dead(){note(${JSON.stringify(t("player.dead"))});}
function applyDesc(d,autoplay){
if(!d){document.getElementById("title").textContent=${JSON.stringify(t("player.none"))};return;}
cur=d;
document.getElementById("title").textContent=d.name;
document.getElementById("meta").textContent=d.id+" · "+d.statusLabel+" · "+(d.index||"-")+"/"+d.total+(d.mark?" · "+${JSON.stringify(t("player.mark"))}+":"+d.mark:"")+(d.warnings.length?" · "+${JSON.stringify(t("player.warnShort"))}+" "+d.warnings.length:"");
document.title=d.name+" — FrameTrace Player";
if(d.src){document.getElementById("novid").hidden=true;if(v.getAttribute("src")!==d.src){v.setAttribute("src",d.src);v.load();}v.playbackRate=Number(rate.value);if(autoplay!==false)v.play().catch(function(){});}
else{document.getElementById("novid").hidden=false;v.removeAttribute("src");v.load();}
document.querySelectorAll(".mk").forEach(function(b){b.classList.toggle("on",b.dataset.m===d.mark);});
document.querySelectorAll(".tg").forEach(function(b){b.classList.toggle("on",d.tags.indexOf(b.dataset.tag)>=0);});
renderList();
}
var listEl=document.getElementById("list");
function renderList(){
call("list",null,function(items){
if(!items){listEl.innerHTML="";return;}
listEl.innerHTML=items.map(function(it){
return '<div class="li'+(it.id===(cur&&cur.id)?" on":"")+(it.hasVideo?"":" novid")+'" data-id="'+esc(it.id)+'" tabindex="0" role="option" aria-selected="'+(it.id===(cur&&cur.id))+'"><span class="idx">'+it.index+'</span>'+esc(it.name)+(it.markLabel?'<span class="mk">'+esc(it.markLabel)+'</span>':'')+'</div>';
}).join("");
var on=listEl.querySelector(".li.on");if(on)on.scrollIntoView({block:"nearest"});
});
}
listEl.addEventListener("click",function(e){var it=e.target.closest(".li");if(!it)return;call("focus",it.dataset.id,function(d){if(d===undefined){dead();return;}applyDesc(d,true);});});
listEl.addEventListener("keydown",function(e){if(e.key!=="Enter"&&e.key!==" ")return;var it=e.target.closest(".li");if(!it)return;e.preventDefault();call("focus",it.dataset.id,function(d){if(d===undefined){dead();return;}applyDesc(d,true);});});
function nav(step,autoplay){call("navigate",step,function(r){if(!r){dead();return;}applyDesc(r.descriptor,autoplay);if(!r.moved)note(step>0?${JSON.stringify(t("player.last"))}:${JSON.stringify(t("player.first"))});else note("");});}
function doMark(m){if(!cur)return;call("focus",cur.id,function(f){if(f===undefined){dead();return;}call("mark",m||null,function(d){if(d===undefined){dead();return;}applyDesc(d,true);});});}
function doTag(t){if(!cur)return;call("focus",cur.id,function(f){if(f===undefined){dead();return;}call("toggleTag",t,function(d){if(d===undefined){dead();return;}applyDesc(d,false);});});}
function skip(d){v.currentTime=Math.max(0,Math.min(v.duration||1e9,v.currentTime+d));}
function stepRate(d){var i=RATES.indexOf(Number(rate.value));var n=RATES[Math.max(0,Math.min(RATES.length-1,(i<0?3:i)+d))];rate.value=String(n);v.playbackRate=n;}
document.getElementById("b10").onclick=function(){skip(-10)};
document.getElementById("b1").onclick=function(){skip(-1)};
document.getElementById("f1").onclick=function(){skip(1)};
document.getElementById("f10").onclick=function(){skip(10)};
document.getElementById("prev").onclick=function(){nav(-1,true)};
document.getElementById("next").onclick=function(){nav(1,true)};
rate.onchange=function(){v.playbackRate=Number(rate.value)};
document.getElementById("play").onclick=function(){v.paused?v.play():v.pause()};
document.querySelectorAll(".mk").forEach(function(b){b.onclick=function(){doMark(b.dataset.m)}});
document.querySelectorAll(".tg").forEach(function(b){b.onclick=function(){doTag(b.dataset.tag)}});
document.getElementById("cap").onclick=function(){
if(!v.videoWidth){note(${JSON.stringify(t("player.noVideo"))});return;}
var c=document.createElement("canvas");c.width=v.videoWidth;c.height=v.videoHeight;
c.getContext("2d").drawImage(v,0,0);
var du=c.toDataURL("image/jpeg",0.92);
fetch("/api/capture-frame",{method:"POST",headers:{"Content-Type":"text/plain"},body:JSON.stringify({id:cur?cur.id:"unknown",image:du.slice(du.indexOf(",")+1),time:v.currentTime.toFixed(2)})}).then(function(r){return r.json()}).then(function(d){note(d.ok?${JSON.stringify(t("player.savedPfx"))}+" "+d.path:${JSON.stringify(t("player.capFailPfx"))}+" "+(d.error||""))}).catch(function(){note(${JSON.stringify(t("player.capOffline"))})});
};
v.addEventListener("ended",function(){nav(1,true);});
document.addEventListener("keydown",function(e){
if(e.target&&e.target.tagName==="SELECT")return;
if(e.target&&e.target.closest&&e.target.closest("#list")){
if(e.key==="ArrowDown"||e.key==="ArrowUp"){e.preventDefault();var items=listEl.querySelectorAll(".li");var idx=Array.prototype.indexOf.call(items,e.target);var n=items[idx+(e.key==="ArrowDown"?1:-1)];if(n)n.focus();}
return;}
if(e.key==="ArrowLeft")skip(-10);else if(e.key==="ArrowRight")skip(10);
else if(e.key==="ArrowUp"){e.preventDefault();stepRate(1);}
else if(e.key==="ArrowDown"){e.preventDefault();stepRate(-1);}
else if(e.key===" "){e.preventDefault();v.paused?v.play():v.pause();}
else if(e.key==="j")nav(1,true);else if(e.key==="k")nav(-1,true);
else if(e.key==="1")doMark("reviewed");else if(e.key==="2")doMark("important");
else if(e.key==="3")doMark("needs_verification");else if(e.key==="0")doMark("");});
call("descriptor",null,function(d){if(d===undefined){dead();}else{applyDesc(d,true);}});
<\/script></body></html>`);
  win.document.close();
  win.focus();
}

function moveActive(step) {
  const filtered = filteredRecords();
  if (!filtered.length) return;
  const index = filtered.findIndex(record => record.id === state.activeId);
  const next = Math.max(0, Math.min(filtered.length - 1, (index < 0 ? 0 : index + step)));
  const nextId = filtered[next].id;
  const nextPage = Math.floor(next / state.pageSize) + 1;
  const pageChanged = nextPage !== state.currentPage;
  state.activeId = nextId;
  if (pageChanged) {
    state.currentPage = nextPage;
    render();
    const card = els.recordGrid.querySelector(`.card[data-id="${CSS.escape(state.activeId)}"]`);
    card?.scrollIntoView({ block: "nearest" });
    return;
  }
  // Same page: avoid rebuilding up to 1000 cards on every j/k.
  els.recordGrid.querySelectorAll(".card.active").forEach(el => { el.classList.remove("active"); el.tabIndex = -1; });
  const card = els.recordGrid.querySelector(`.card[data-id="${CSS.escape(state.activeId)}"]`);
  if (card) { card.classList.add("active"); card.tabIndex = 0; }
  card?.scrollIntoView({ block: "nearest" });
  renderDetails();
}

function toggleActiveSelection() {
  if (!state.activeId) return;
  if (state.selectedIds.has(state.activeId)) state.selectedIds.delete(state.activeId);
  else state.selectedIds.add(state.activeId);
  render();
}

let shortcutsLastFocus = null;
function toggleShortcuts(open) {
  const modal = document.getElementById("shortcutsModal");
  if (open && modal.hidden) shortcutsLastFocus = document.activeElement;
  modal.hidden = !open;
  if (open) document.getElementById("btnShortcutsClose")?.focus();
  else if (shortcutsLastFocus) { shortcutsLastFocus.focus(); shortcutsLastFocus = null; }
}
document.getElementById("shortcutsModal").addEventListener("keydown", e => {
  if (e.key !== "Tab") return;
  const focusables = [...e.currentTarget.querySelectorAll("button, input, select, a[href], [tabindex]")]
    .filter(el => !el.disabled);
  if (!focusables.length) return;
  const first = focusables[0];
  const last = focusables[focusables.length - 1];
  if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
  else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
});

// Single-key triage: mark the active record and advance so an examiner
// can clear a review queue without touching the mouse.
function markActive(status) {
  const before = filteredRecords();
  const index = before.findIndex(record => record.id === state.activeId);
  if (index < 0) return;
  if (status === null) { delete state.marks[state.activeId]; state.notes[state.activeId] = ""; }
  else state.marks[state.activeId] = { status, marked_unix: Math.floor(Date.now() / 1000) };
  touchAnnotation(state.activeId);
  storageSet(MARKS_KEY, state.marks);
  const after = filteredRecords();
  const next = before.slice(index + 1).find(record => after.some(item => item.id === record.id));
  state.activeId = next?.id || after[Math.min(index, after.length - 1)]?.id || null;
  state.currentPage = Math.max(1, Math.floor(after.findIndex(record => record.id === state.activeId) / state.pageSize) + 1);
  render();
}

document.addEventListener("keydown", event => {
  const tag = document.activeElement?.tagName;
  if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") {
    if (event.key === "Escape") document.activeElement.blur();
    return;
  }
  if (tag === "BUTTON" && (event.key === " " || event.key === "Enter")) {
    return; // let the focused button receive its normal activation keys
  }
  if (event.ctrlKey && (event.key === "a" || event.key === "A")) { event.preventDefault(); selectAllFiltered(); return; }
  if (event.ctrlKey && (event.key === "d" || event.key === "D")) { event.preventDefault(); clearSelection(); return; }
  switch (event.key) {
    case "j": moveActive(1); break;
    case "k": moveActive(-1); break;
    case " ": event.preventDefault(); toggleActiveSelection(); break;
    case "Enter":
      els.mediaStage.querySelector("video")?.play().catch(() => {});
      break;
    case "1": markActive("reviewed"); break;
    case "2": markActive("important"); break;
    case "3": markActive("needs_verification"); break;
    case "0": markActive(null); break;
    case "ArrowLeft": skipVideo(-10); break;
    case "ArrowRight": skipVideo(10); break;
    case "[": rateStep(-1); break;
    case "]": rateStep(1); break;
    case "i": setRangePoint("in"); break;
    case "o": setRangePoint("out"); break;
    case "f": toggleFullscreen(); break;
    case "t": toggleTheater(); break;
    case "p": togglePip(); break;
    case "?": toggleShortcuts(true); break;
    case "Escape": toggleShortcuts(false); if (state.layout.theater) toggleTheater(); break;
    default: break;
  }
});

els.query.addEventListener("input", () => {
  clearTimeout(els.query._debounce);
  els.query._debounce = setTimeout(() => {
    state.query = els.query.value.trim().toLowerCase();
    state.currentPage = 1;
    render();
  }, 150);
});
els.kind.addEventListener("change", () => { state.kind = els.kind.value; state.currentPage = 1; render(); });
els.status.addEventListener("change", () => { state.status = els.status.value; state.currentPage = 1; render(); });
els.pageSize.addEventListener("change", () => {
  state.pageSize = Number(els.pageSize.value) || 100;
  state.currentPage = 1;
  render();
});
els.prevPage.addEventListener("click", () => { state.currentPage -= 1; render(); });
els.nextPage.addEventListener("click", () => { state.currentPage += 1; render(); });
els.sortBy.addEventListener("change", () => { state.sortBy = els.sortBy.value; state.currentPage = 1; render(); });
els.groupBy.addEventListener("change", () => { state.groupBy = els.groupBy.value; state.currentPage = 1; render(); });
els.tagFilter.addEventListener("change", () => {
  state.chip = els.tagFilter.value;
  state.status = "";
  els.status.value = "";
  state.currentPage = 1;
  render();
});
els.dateFrom.addEventListener("change", () => { state.dateFrom = els.dateFrom.value; state.currentPage = 1; render(); });
els.dateTo.addEventListener("change", () => { state.dateTo = els.dateTo.value; state.currentPage = 1; render(); });
document.getElementById("btnClearDates").addEventListener("click", () => {
  state.dateFrom = "";
  state.dateTo = "";
  els.dateFrom.value = "";
  els.dateTo.value = "";
  state.currentPage = 1;
  render();
});
els.videoMode.addEventListener("change", () => { state.layout.videoMode = els.videoMode.value; saveLayout(); applyVideoScale(); });
els.videoZoom.addEventListener("input", () => {
  state.layout.videoMode = els.videoMode.value === "fit" ? "fit" : els.videoMode.value;
  if (els.videoMode.value !== "fit") state.layout.videoMode = els.videoMode.value;
  state.layout.videoZoom = Number(els.videoZoom.value);
  if (els.videoMode.value === "fit") { els.videoMode.value = "fit"; }
  saveLayoutSoon();
  applyVideoScale();
});
document.getElementById("btnTheater").addEventListener("click", toggleTheater);
document.getElementById("btnFullscreen").addEventListener("click", toggleFullscreen);
document.getElementById("btnPip").addEventListener("click", togglePip);
document.getElementById("btnSkipBack").addEventListener("click", () => skipVideo(-10));
document.getElementById("btnSkipFwd").addEventListener("click", () => skipVideo(10));
document.getElementById("playRate").addEventListener("change", () => setRate(Number(els.playRate.value) || 1));
document.getElementById("btnPopPlayer").addEventListener("click", openPlayerWindow);
document.getElementById("btnDual").addEventListener("click", () => {
  state.layout.dual = !state.layout.dual;
  saveLayout();
  applyLayout();
  if (state.layout.dual) {
    const record = selectedRecord();
    if (!record || !matesOf(record).length) toast(t("toast.noPair"));
  }
  mediaRenderedFor = null;
  renderDetails();
});
document.getElementById("btnTelemetry").addEventListener("click", async (ev) => {
  const record = selectedRecord();
  if (!record) return;
  const btn = ev.currentTarget;
  btn.disabled = true;
  try {
    const res = await fetch("/api/telemetry", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "" })
    });
    const data = await res.json();
    if (data.ok && data.report) {
      record.telemetry = data.report;
      if (DATA.telemetry) DATA.telemetry[String(record.id).replace(/[:\\\/]/g, "_")] = data.report;
      renderDetails();
      toast(tf("toast.telemetryDone", { n: data.report.point_count || 0 }));
    } else {
      toast(tf("toast.telemetryFail", { err: data.error || "" }));
    }
  } catch {
    toast(t("toast.telemetryOffline"));
  } finally {
    btn.disabled = false;
  }
});

// Grab the decoded frame straight off the <video> element — no ffmpeg round
// trip — and store it as a hashed case artifact (artifacts/captures/).
async function captureCurrentFrame(videoEl, recordId) {
  if (!videoEl || !videoEl.videoWidth) { toast(t("toast.noVideo")); return; }
  const canvas = document.createElement("canvas");
  canvas.width = videoEl.videoWidth;
  canvas.height = videoEl.videoHeight;
  canvas.getContext("2d").drawImage(videoEl, 0, 0);
  const dataUrl = canvas.toDataURL("image/jpeg", 0.92);
  const b64 = dataUrl.slice(dataUrl.indexOf(",") + 1);
  try {
    const res = await fetch("/api/capture-frame", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: recordId, image: b64, time: videoEl.currentTime.toFixed(2) })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.captureSaved", { path: data.path }) : tf("toast.captureFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.captureOffline"));
  }
}
document.getElementById("btnCaptureFrame").addEventListener("click", () => {
  const record = selectedRecord();
  captureCurrentFrame(currentVideo(), record ? record.id : "unknown");
});

// --- 구간(인/아웃) 마킹 + 클립보내기: 기록별로 localStorage에 유지됨 ---
function rangeOf(record) { return record ? state.ranges[record.id] : null; }
function updateRangeLabel() {
  const el = document.getElementById("rangeLabel");
  const record = selectedRecord();
  const range = rangeOf(record);
  const point = value => Number.isFinite(value) ? `${value.toFixed(1)}s` : "—";
  el.textContent = range && (Number.isFinite(range.in) || Number.isFinite(range.out))
    ? tf("range.label", { in: point(range.in), out: point(range.out) })
    : "";
  const proxyBtn = document.getElementById("btnProxy");
  if (proxyBtn) proxyBtn.classList.toggle("on", !!(record && state.proxies[record.id]));
}
function setRangePoint(which) {
  const video = currentVideo();
  const record = selectedRecord();
  if (!video || !record) { toast(t("toast.pickEvidence")); return; }
  const t = video.currentTime;
  const range = state.ranges[record.id] || { in: null, out: null };
  if (which === "in") range.in = t; else range.out = t;
  if (Number.isFinite(range.in) && Number.isFinite(range.out) && range.out <= range.in) {
    toast(t("toast.outBeforeIn"));
    if (which === "in") range.in = null; else range.out = null;
  }
  if (range.in == null && range.out == null) delete state.ranges[record.id];
  else state.ranges[record.id] = range;
  storageSet(RANGES_KEY, state.ranges);
  updateRangeLabel();
}
document.getElementById("btnSetIn").addEventListener("click", () => setRangePoint("in"));
document.getElementById("btnSetOut").addEventListener("click", () => setRangePoint("out"));
// Review proxy toggle: heavy originals (4K/HEVC) stutter on exam machines,
// so the viewer can lazily ask the server for a low-bitrate proxy.
document.getElementById("btnProxy").addEventListener("click", async () => {
  const record = selectedRecord();
  if (!record) { toast(t("toast.playPick")); return; }
  const btn = document.getElementById("btnProxy");
  if (state.proxies[record.id]) {
    delete state.proxies[record.id];
    storageSet(PROXIES_KEY, state.proxies);
    mediaRenderedFor = null;
    render();
    toast(t("toast.playOriginal"));
    return;
  }
  btn.disabled = true;
  toast(t("toast.proxyBuilding"));
  try {
    const res = await fetch("/api/proxy", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "" })
    });
    const data = await res.json();
    if (data.ok) {
      state.proxies[record.id] = data.path;
      storageSet(PROXIES_KEY, state.proxies);
      mediaRenderedFor = null;
      render();
      toast(t("toast.proxyPlaying"));
    } else {
      toast(tf("toast.proxyFail", { err: data.error || "" }));
    }
  } catch {
    toast(t("toast.proxyOffline"));
  } finally {
    btn.disabled = false;
  }
});
document.getElementById("btnExportClip").addEventListener("click", async () => {
  const record = selectedRecord();
  const range = rangeOf(record);
  if (!record || !range || !Number.isFinite(range.in) || !Number.isFinite(range.out) || range.out <= range.in) {
    toast(t("toast.noRange"));
    return;
  }
  try {
    const res = await fetch("/api/export-clip", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "", start: range.in.toFixed(3), duration: (range.out - range.in).toFixed(3) })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.clipDone", { path: data.path }) : tf("toast.clipFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.clipOffline"));
  }
});
document.getElementById("btnExportClipBurn").addEventListener("click", async () => {
  const record = selectedRecord();
  const range = rangeOf(record);
  if (!record || !range || !Number.isFinite(range.in) || !Number.isFinite(range.out) || range.out <= range.in) {
    toast(t("toast.noRange"));
    return;
  }
  const exhibit = window.prompt(t("clip.exhibitPrompt"), t("clip.exhibitDefault")) ?? "";
  try {
    const res = await fetch("/api/export-clip", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ id: record.id, path: record.path || "", start: range.in.toFixed(3), duration: (range.out - range.in).toFixed(3), burn_in: "true", exhibit })
    });
    const data = await res.json();
    toast(data.ok ? tf("toast.clipDone", { path: data.path }) : tf("toast.clipFail", { err: data.error || "" }));
  } catch {
    toast(t("toast.clipOffline"));
  }
});
document.getElementById("btnShortcuts").addEventListener("click", () => toggleShortcuts(true));
document.getElementById("btnShortcutsClose").addEventListener("click", () => toggleShortcuts(false));
document.getElementById("btnSelectFiltered").addEventListener("click", selectAllFiltered);
document.getElementById("btnClearSelection").addEventListener("click", clearSelection);
document.getElementById("btnMarkReviewed").addEventListener("click", () => applyMark("reviewed"));
document.getElementById("btnMarkImportant").addEventListener("click", () => applyMark("important"));
document.getElementById("btnMarkVerify").addEventListener("click", () => applyMark("needs_verification"));
document.getElementById("btnMarkClear").addEventListener("click", () => applyMark(null));
document.getElementById("btnCopyIds").addEventListener("click", () => {
  const ids = targetIds();
  if (ids.length) copyText(ids.join("\n"), t("copy.ids"));
});
document.getElementById("btnCopyPaths").addEventListener("click", () => {
  const ids = new Set(targetIds());
  const paths = records.filter(record => ids.has(record.id)).map(record => record.path);
  if (paths.length) copyText(paths.join("\n"), t("copy.paths"));
  else toast(t("toast.selectTarget"));
});
// Tag menu: preset buttons are rebuilt from the editable preset list.
// Each row applies the tag to the current selection; the trailing ×
// removes the preset (applied tags on records are untouched). The
// input registers a new preset — and, when a selection exists, applies
// it right away.
const tagMenuList = document.getElementById("tagMenuList");
function rebuildTagMenu() {
  tagMenuList.innerHTML = state.tagPresets.map(tag =>
    `<div class="tag-menu-row"><button type="button" data-keep data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}</button><button type="button" class="tag-del" data-keep data-del-preset="${escapeHtml(tag)}" title="${t("tagmenu.delPreset")}">×</button></div>`
  ).join("")
    + `<button type="button" data-tag-clear>${t("tagmenu.clear")}</button><hr>`
    + `<div class="tag-add"><input type="text" id="tagPresetInput" placeholder="${t("tagmenu.new")}" maxlength="24">`
    + `<button type="button" data-keep data-tag-add>${t("tagmenu.add")}</button></div>`;
}
function addTagPresetFromMenu() {
  const input = document.getElementById("tagPresetInput");
  const name = (input?.value || "").trim();
  if (!name) return;
  const isNew = !state.tagPresets.includes(name);
  addTagPreset(name);
  rebuildTagMenu();
  document.getElementById("tagPresetInput")?.focus();
  if (targetIds().length) applyTag(name);
  else if (isNew) toast(tf("toast.presetAdded", { tag: name }));
}
tagMenuList.addEventListener("click", e => {
  const del = e.target.closest("[data-del-preset]");
  if (del) { removeTagPreset(del.dataset.delPreset); rebuildTagMenu(); return; }
  if (e.target.closest("[data-tag-add]")) { addTagPresetFromMenu(); return; }
  if (e.target.closest("[data-tag-clear]")) { clearTags(); return; }
  const apply = e.target.closest("[data-tag]");
  if (apply) applyTag(apply.dataset.tag);
});
tagMenuList.addEventListener("keydown", e => {
  if (e.key === "Enter" && e.target.id === "tagPresetInput") {
    e.preventDefault();
    addTagPresetFromMenu();
  }
});

// Selection-bar menus: toggle on the anchor button, close on outside
// click / Escape / a menu item without data-keep (tag items stay open so
// several tags can be applied to one selection).
const MENU_BUTTONS = { tagMenuList: "btnTagMenu", exportMenuList: "btnExportMenu", viewMenuList: "btnViewMenu" };
function setMenuExpanded(listId, open) {
  const btn = document.getElementById(MENU_BUTTONS[listId]);
  if (btn) btn.setAttribute("aria-expanded", open ? "true" : "false");
}
function closeAllMenus() {
  document.querySelectorAll(".menu-list").forEach(l => { l.hidden = true; setMenuExpanded(l.id, false); });
}
function toggleMenu(listId) {
  const list = document.getElementById(listId);
  const willOpen = list.hidden;
  closeAllMenus();
  list.hidden = !willOpen;
  setMenuExpanded(listId, !willOpen);
}
document.getElementById("btnTagMenu").addEventListener("click", e => {
  e.stopPropagation();
  rebuildTagMenu();
  toggleMenu("tagMenuList");
});
document.getElementById("btnExportMenu").addEventListener("click", e => {
  e.stopPropagation();
  toggleMenu("exportMenuList");
});
document.getElementById("btnViewMenu").addEventListener("click", e => {
  e.stopPropagation();
  toggleMenu("viewMenuList");
});
document.getElementById("btnMoreFilters").addEventListener("click", e => {
  const extra = document.getElementById("filtersExtra");
  extra.hidden = !extra.hidden;
  e.currentTarget.setAttribute("aria-expanded", extra.hidden ? "false" : "true");
});
const btnGroupKind = document.getElementById("btnGroupKind");
btnGroupKind.addEventListener("click", () => {
  state.groupBy = state.groupBy === "kind" ? "none" : "kind";
  els.groupBy.value = state.groupBy;
  state.currentPage = 1;
  render();
});
const syncGroupKindChip = () => btnGroupKind.classList.toggle("active", state.groupBy === "kind");
els.groupBy.addEventListener("change", syncGroupKindChip);
document.querySelectorAll(".menu-list").forEach(list => {
  list.addEventListener("click", e => {
    // Keep the document-level closer from seeing in-menu clicks; an
    // action item without data-keep closes its own menu explicitly.
    e.stopPropagation();
    if (e.target.closest("button") && !e.target.closest("button").hasAttribute("data-keep")) {
      list.hidden = true;
      setMenuExpanded(list.id, false);
    }
  });
});
document.addEventListener("click", closeAllMenus);
document.addEventListener("keydown", e => {
  if (e.key === "Escape") closeAllMenus();
});
document.getElementById("btnDownloadSelection").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  downloadJSON(`frametrace-selection-${manifest.case_id || "case"}-${Date.now()}.json`, {
    schema_version: 1,
    case_id: manifest.case_id || null,
    created_unix: Math.floor(Date.now() / 1000),
    items: selected.map(record => ({
      selector: record.id,
      kind: record.kind,
      action: record.kind === "video" ? "export" : record.kind === "candidate" ? "recover" : "validate",
      format: "mp4",
      notes: record.name
    }))
  });
});
function marksPayload() {
  // Marks and free-text notes travel together: a record with only a note
  // is exported with status "noted" so the case DB keeps the memo.
  const ids = new Set(Object.keys(state.marks));
  Object.keys(state.notes).forEach(id => { if ((state.notes[id] || "").trim()) ids.add(id); });
  const marks = [...ids].map(id => {
    const mark = state.marks[id];
    const note = (state.notes[id] || "").trim();
    return {
      id,
      status: mark ? mark.status : "noted",
      marked_unix: mark ? mark.marked_unix : Math.floor(Date.now() / 1000),
      note,
      examiner: state.examiners[id] ?? ((state.examiner || "").trim() || null)
    };
  });
  const tagEntries = Object.entries(state.tags).map(([id, tags]) => ({ id, tags }));
  return {
    schema_version: 2,
    protocol: "patch-v1",
    deleted_ids: state.deletedIds.filter(id => !ids.has(id)),
    case_id: manifest.case_id || null,
    examiner: (state.examiner || "").trim() || null,
    exported_unix: Math.floor(Date.now() / 1000),
    marks,
    tags: tagEntries
  };
}

document.getElementById("btnDownloadMarks").addEventListener("click", () => {
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) { toast(t("toast.noChanges")); return; }
  downloadJSON(`frametrace-marks-${manifest.case_id || "case"}.json`, payload);
});

// --- Review-change protection -------------------------------------------
// touchAnnotation() funnels every mark/note/tag mutation, and deletedIds
// already tracks records changed since the last server import. pendingReviewSync
// drives both the beforeunload guard and the debounced server auto-save, so
// examiner work can't silently die in localStorage.
let pendingReviewSync = state.deletedIds.length > 0;
let autosaveTimer = null;
const viewerIsServed = location.protocol === "http:" || location.protocol === "https:";

function markReviewDirty() {
  pendingReviewSync = true;
  if (!viewerIsServed) return; // file:// viewing — unload warning only
  clearTimeout(autosaveTimer);
  autosaveTimer = setTimeout(autoApplyMarks, 2500);
}

function clearReviewDirty() {
  pendingReviewSync = false;
  state.deletedIds = [];
  state.drafts = {};
  storageSet(ANNOTATIONS_KEY, state.drafts);
}

async function pushMarksToCase(payload) {
  const res = await fetch("/api/import-marks", {
    method: "POST",
    headers: { "Content-Type": "text/plain" },
    body: JSON.stringify({ marks_json: JSON.stringify(payload) })
  });
  return res.json();
}

async function autoApplyMarks() {
  if (!pendingReviewSync || !viewerIsServed) return;
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) {
    clearReviewDirty();
    return;
  }
  try {
    const data = await pushMarksToCase(payload);
    if (data.ok) clearReviewDirty();
  } catch { /* stay dirty — the next change retries */ }
}

window.addEventListener("beforeunload", (event) => {
  if (!pendingReviewSync && !state.deletedIds.length) return;
  event.preventDefault();
  event.returnValue = "";
});

// Server-served viewers can push marks straight into the case DB instead of
// the download → file-pick → import dance.
document.getElementById("btnApplyMarks").addEventListener("click", async () => {
  const payload = marksPayload();
  if (!payload.marks.length && !payload.tags.length && !payload.deleted_ids.length) { toast(t("toast.noChanges")); return; }
  try {
    const data = await pushMarksToCase(payload);
    if (data.ok) {
      clearReviewDirty();
      toast(t("toast.applyDone"));
    } else {
      toast(tf("toast.applyFail", { err: data.error || "?" }));
    }
  } catch (err) {
    toast(t("toast.applyOffline"));
  }
});

// --- 선별 결과 내보내기: 사람이 받는 형태 (CSV / 요약 리포트 / 자료 묶음) ---

document.getElementById("btnDownloadCsv").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  const header = ["id", "kind", "name", "status", "mark", "examiner_note", "tags", "sha256", "size_bytes",
    "recorded_time", "channel", "rec_type", "inode", "original_path", "source_path",
    "warnings", "note", "anomalies"];
  const rows = selected.map(record => [
    record.id,
    KIND_LABELS[record.kind] || record.kind,
    record.name,
    record.status,
    state.marks[record.id]?.status || ((state.notes[record.id] || "").trim() ? "noted" : ""),
    (state.notes[record.id] || "").trim(),
    (state.tags[record.id] || []).join("; "),
    record.sha256,
    record.size ?? "",
    isoTime(record.recTime),
    record.channel || "",
    record.recType || "",
    record.inode ?? "",
    record.originalPath || "",
    record.path || "",
    (record.warnings || []).join("; "),
    record.note && record.note !== "-" ? record.note : "",
    (record.anomalies || []).map(item => item.kind).join("; ")
  ].map(csvCell).join(","));
  downloadText(
    `frametrace-selection-${manifest.case_id || "case"}.csv`,
    String.fromCharCode(0xFEFF) + header.join(",") + "\n" + rows.join("\n") + "\n",
    "text/csv;charset=utf-8"
  );
});

// A self-contained, printable handoff report: everything a requester
// needs to understand what was selected and why, without the viewer.
document.getElementById("btnSummary").addEventListener("click", () => {
  let items = selectedRecords();
  if (!items.length) {
    items = records.filter(record => state.marks[record.id] || (state.tags[record.id] || []).length || (state.notes[record.id] || "").trim());
  }
  if (!items.length) { toast(t("toast.noItems")); return; }
  const e = escapeHtml;
  const fmtSize = value => Number.isFinite(value) ? `${(value / 1048576).toFixed(1)} MB` : "-";
  const rowHtml = items.map(record => {
    const mark = state.marks[record.id]?.status;
    const memo = (state.notes[record.id] || "").trim();
    const warn = [...(record.warnings || []), ...(record.anomalies || []).map(item => item.kind)].join("; ");
    return `<tr><td>${e(record.id)}</td><td>${e(record.name)}</td><td>${e(KIND_LABELS[record.kind] || record.kind)}</td>` +
      `<td>${e(record.status)}</td><td>${e(mark ? markLabel(mark) : (memo ? t("mark.noted") : ""))}</td>` +
      `<td>${e((state.tags[record.id] || []).join(", "))}</td>` +
      `<td class="mono">${e(record.sha256 && record.sha256 !== "-" ? record.sha256 : "")}</td>` +
      `<td>${fmtSize(record.size)}</td><td>${e(isoTime(record.recTime))}</td>` +
      `<td>${e(warn)}${record.note && record.note !== "-" ? `<br>${e(record.note)}` : ""}${memo ? `<br><b>${t("report.memo")}</b> ${e(memo)}` : ""}</td>` +
      `<td class="mono">${e(record.originalPath || record.path || "")}</td></tr>`;
  }).join("\n");
  const counts = { verified: 0, candidate: 0, failed: 0 };
  items.forEach(record => {
    if (record.status === "validation-failed") counts.failed += 1;
    else if (record.status === "ffprobe-video-stream-confirmed") counts.verified += 1;
    else counts.candidate += 1;
  });
  const html = `<!doctype html><html lang="${state.locale}"><head><meta charset="utf-8">
<title>${t("report.title")} — ${e(manifest.case_id || "case")}</title>
<style>
body{font-family:ui-sans-serif,system-ui,"Segoe UI",sans-serif;margin:32px;color:#1f2724}
h1{font-size:20px} .meta{color:#68736f;font-size:13px;margin-bottom:4px}
.note{background:#fdf6e8;border:1px solid #e3cf9e;border-radius:6px;padding:10px 12px;font-size:13px;margin:14px 0}
table{border-collapse:collapse;width:100%;font-size:12px}
th,td{border:1px solid #d8dedb;padding:6px 8px;text-align:left;vertical-align:top}
th{background:#f2f5f4} .mono{font-family:Consolas,monospace;font-size:11px;word-break:break-all}
</style></head><body>
<h1>${t("report.title")}</h1>
<div class="meta">${tf("report.meta", { case: manifest.case_id || "-", when: new Date().toLocaleString(), n: items.length })}</div>
<div class="meta">${tf("report.counts", { v: counts.verified, c: counts.candidate, f: counts.failed })}</div>
<div class="note">${t("report.note")}</div>
<table><thead><tr><th>ID</th><th>${t("report.th.name")}</th><th>${t("report.th.kind")}</th><th>${t("report.th.status")}</th><th>${t("report.th.mark")}</th><th>${t("report.th.tags")}</th>
<th>SHA-256</th><th>${t("report.th.size")}</th><th>${t("report.th.time")}</th><th>${t("report.th.warn")}</th><th>${t("report.th.orig")}</th></tr></thead>
<tbody>
${rowHtml}
</tbody></table>
</body></html>`;
  downloadText(`frametrace-summary-${manifest.case_id || "case"}.html`, html, "text/html;charset=utf-8");
});

// 서버 측 자료 묶음 — 선택 파일을 exports/selection-*/ 에 해시 매니페스트와 함께 복사.
document.getElementById("btnExportSelected").addEventListener("click", async () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  const btn = document.getElementById("btnExportSelected");
  btn.disabled = true;
  try {
    const res = await fetch("/api/export-selected", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ items: selected.map(exportItem) })
    }).then(reply => reply.json());
    if (!res.ok) {
      toast(tf("toast.exportFail", { err: res.error || "" }));
      return;
    }
    toast(tf("toast.exportDone", { copied: res.copied, skipped: res.skipped, dir: res.export_dir }));
    fetch("/api/open-folder", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: res.export_dir })
    }).catch(() => {});
  } catch (err) {
    toast(t("toast.offline"));
  } finally {
    btn.disabled = false;
  }
});

// --- 원본 파일 다운로드: /media?path&download=1이 Content-Disposition으로
// 저장을 유도. file:// 스탠드얼론에서는 서버가 없으므로 지원하지 않음 ---
function downloadHref(record) {
  if (location.protocol !== "http:" && location.protocol !== "https:") return "";
  if (!record.path) return "";
  return "/media?path=" + encodeURIComponent(record.path) + "&download=1";
}
function triggerDownload(record) {
  const href = downloadHref(record);
  if (!href) return false;
  const a = document.createElement("a");
  a.href = href;
  a.download = record.originalName || record.name || record.id;
  document.body.appendChild(a);
  a.click();
  a.remove();
  return true;
}
document.getElementById("btnDownloadFile").addEventListener("click", () => {
  const record = selectedRecord();
  if (!record) { toast(t("toast.selectFirst")); return; }
  if (!triggerDownload(record)) {
    toast(t("toast.fileOnly"));
  }
});
document.getElementById("btnDownloadFiles").addEventListener("click", () => {
  const selected = selectedRecords();
  if (!selected.length) { toast(t("toast.selectFirst")); return; }
  let started = 0;
  selected.forEach((record, index) => {
    if (!downloadHref(record)) return;
    started += 1;
    setTimeout(() => triggerDownload(record), index * 450);
  });
  if (!started) {
    toast(t("toast.fileOnly"));
  } else {
    toast(tf("toast.downloads", { n: started }));
  }
});
document.getElementById("btnTranscodeQueue").addEventListener("click", async (ev) => {
  const btn = ev.currentTarget;
  const ids = selectedRecords().map(r => r.id).join(",");
  btn.disabled = true;
  toast(t("toast.queueRunning"));
  try {
    const res = await fetch("/api/transcode-queue", {
      method: "POST",
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ ids })
    });
    const data = await res.json();
    if (data.ok && data.summary) {
      toast(tf("toast.queueDone", { t: data.summary.total ?? 0, p: data.summary.proxied ?? 0, c: data.summary.skipped_existing ?? 0, f: data.summary.failed ?? 0 }));
    } else {
      toast(tf("toast.queueFail", { err: data.error || "" }));
    }
  } catch {
    toast(t("toast.queueOffline"));
  } finally {
    btn.disabled = false;
  }
});

// --- 케이스 타임라인 패널: db/timeline.jsonl을 읽어 시간순 이벤트를 표시 ---
async function loadTimeline(regenerate) {
  const list = document.getElementById("timelineList");
  const meta = document.getElementById("timelineMeta");
  if (location.protocol === "file:") {
    list.innerHTML = `<div class="timeline-row"><span class="desc">${t("timeline.offline")}</span></div>`;
    return;
  }
  if (regenerate) {
    meta.textContent = t("timeline.building");
    try {
      const res = await fetch("/api/advanced", {
        method: "POST", headers: { "Content-Type": "text/plain" },
        body: JSON.stringify({ tool: "timeline" })
      });
      const data = await res.json();
      if (!data.ok) { meta.textContent = tf("timeline.fail", { err: data.error || "" }); return; }
    } catch { meta.textContent = t("timeline.failOffline"); return; }
  }
  try {
    const res = await fetch("/case/db/timeline.jsonl");
    if (!res.ok) {
      meta.textContent = t("timeline.none");
      list.innerHTML = "";
      return;
    }
    const text = await res.text();
    const events = text.split("\n").filter(l => l.trim()).map(l => { try { return JSON.parse(l); } catch { return null; } }).filter(Boolean);
    const byPath = new Map(records.map(r => [r.path, r.id]));
    meta.textContent = tf("timeline.count", { n: events.length });
    const cap = 400;
    list.innerHTML = events.slice(0, cap).map(ev => {
      const recId = byPath.get(ev.path);
      return `<div class="timeline-row" data-id="${escapeHtml(recId || "")}">
        <span class="ts">${escapeHtml(isoTime(ev.ts_unix))}</span>
        <span class="src">${escapeHtml(ev.source || "")}</span>
        <span class="desc" title="${escapeHtml(ev.path || "")} ${escapeHtml(ev.detail || "")}">${escapeHtml(ev.kind || "")} — ${escapeHtml((ev.path || "").split(/[\\/]/).pop() || ev.path || "")}</span>
      </div>`;
    }).join("") + (events.length > cap ? `<div class="timeline-row"><span class="desc muted">${tf("timeline.ellipsis", { n: events.length - cap })}</span></div>` : "");
    list.querySelectorAll(".timeline-row[data-id]").forEach(row => {
      row.addEventListener("click", () => {
        if (!row.dataset.id) return;
        state.selectedIds.clear();
        state.selectedIds.add(row.dataset.id);
        state.activeId = row.dataset.id;
        render();
      });
    });
  } catch {
    meta.textContent = t("timeline.unreadable");
  }
}
document.getElementById("btnTimeline").addEventListener("click", () => {
  const panel = document.getElementById("timelinePanel");
  panel.hidden = !panel.hidden;
  if (!panel.hidden) loadTimeline(false);
});
document.getElementById("btnTimelineClose").addEventListener("click", () => {
  document.getElementById("timelinePanel").hidden = true;
});
document.getElementById("btnTimelineGen").addEventListener("click", () => loadTimeline(true));

// The viewer also runs standalone via file:// — server-backed features need
// the workstation server, and marks/tags live in a different localStorage
// origin than the served viewer, so make those limits visible up front.
if (location.protocol === "file:") {
  const btn = document.getElementById("btnExportSelected");
  btn.disabled = true;
  btn.title = t("offline.openServer");
  const apply = document.getElementById("btnApplyMarks");
  apply.disabled = true;
  apply.title = t("offline.applyTitle");
  const cap = document.getElementById("btnCaptureFrame");
  cap.disabled = true;
  cap.title = t("offline.serverOnly");
  const clip = document.getElementById("btnExportClip");
  clip.disabled = true;
  clip.title = t("offline.serverOnly");
  toast(t("toast.standalone"));
}

state.pageSize = Number(els.pageSize.value) || 100;
const examinerInput = document.getElementById("examinerName");
if (examinerInput) {
  examinerInput.value = state.examiner || "";
  examinerInput.addEventListener("input", () => {
    state.examiner = examinerInput.value;
    storageSet(EXAMINER_KEY, state.examiner);
  });
}
// Embedded mode (inside the examiner workstation iframe): drop the
// viewer's own chrome — case title, stat badges, locale toggle, view
// menu — because the host already shows them. Grid/media keep full
// functionality. The host flips it off over postMessage when the
// iframe goes fullscreen, so the full UI returns there.
const setEmbed = on => document.body.classList.toggle("embed", on);
setEmbed(new URLSearchParams(location.search).has("embed") || window.self !== window.top);
const exitFsBtn = document.getElementById("btnExitFs");
window.addEventListener("message", event => {
  if (!event.data || event.data.type !== "ft-embed") return;
  setEmbed(!!event.data.embed);
  // Framed + embed off = the host fullscreened us: show a way back.
  if (exitFsBtn) exitFsBtn.hidden = event.data.embed || window.self === window.top;
});
if (exitFsBtn) {
  exitFsBtn.addEventListener("click", () => {
    try { window.parent.postMessage({ type: "ft-exit-fullscreen" }, "*"); } catch (e) {}
  });
}
setupHeightSplitter();
setupColumnSplitter();
setupGridDelegation();
document.getElementById("btnLang")?.addEventListener("click", () => {
  state.locale = state.locale === "ko" ? "en" : "ko";
  storageSet(LOCALE_KEY, state.locale);
  render();
});
applyLayout();
render();
})();

/* Data loading order:
 * 1. window.__FRAMETRACE_DATA__ already set — legacy single-file bundle
 *    (older make-review output) — use it directly.
 * 2. Served by the frametrace workstation — page the case index through
 *    /api/records + /api/records-meta so the HTML never carries a
 *    multi-hundred-MB inline payload for large cases.
 * 3. Opened as a plain file (or a server without the API) — the
 *    generated sibling data-bundle.js, which a plain <script> tag can
 *    load even under file:// where fetch() is blocked. */
async function loadViewerData() {
  if (window.__FRAMETRACE_DATA__ && window.__FRAMETRACE_DATA__.manifest) {
    return window.__FRAMETRACE_DATA__;
  }
  const note = document.createElement("div");
  note.id = "ft-load-note";
  note.style.cssText = "position:fixed;inset:0;display:grid;place-items:center;background:rgba(14,21,19,.92);color:#8fa79e;font:14px/1.5 system-ui,sans-serif;z-index:9999";
  note.textContent = "증거 목록을 불러오는 중…";
  document.body.appendChild(note);
  const done = () => note.remove();
  try {
    const probe = await fetch("/api/records?offset=0&limit=500");
    if (probe.ok) {
      const first = await probe.json();
      if (first && first.ok) {
        const metaResp = await fetch("/api/records-meta");
        const meta = metaResp.ok ? await metaResp.json() : {};
        const videos = Array.isArray(first.videos) ? first.videos.slice() : [];
        const total = first.total || videos.length;
        while (videos.length < total) {
          note.textContent = `증거 목록을 불러오는 중… ${videos.length}/${total}`;
          const r = await (await fetch(`/api/records?offset=${videos.length}&limit=500`)).json();
          if (!r || !r.ok || !Array.isArray(r.videos) || !r.videos.length) break;
          videos.push(...r.videos);
        }
        const scan = (meta && meta.scan) || {};
        scan.videos = videos;
        done();
        return {
          manifest: meta.manifest || {},
          scan,
          carveLog: meta.carveLog,
          filesystemLog: meta.filesystemLog,
          validationLog: meta.validationLog,
          anomalyLog: meta.anomalyLog,
          flsEntries: meta.flsEntries,
          thumbs: meta.thumbs,
          annotations: meta.annotations,
          deepfake: meta.deepfake,
          telemetry: meta.telemetry,
        };
      }
    }
  } catch { /* API unavailable — fall through to the bundle */ }
  await new Promise((resolve) => {
    const s = document.createElement("script");
    s.src = "data-bundle.js";
    s.onload = resolve;
    s.onerror = resolve;
    document.head.appendChild(s);
  });
  done();
  return window.__FRAMETRACE_DATA__ || {};
}
