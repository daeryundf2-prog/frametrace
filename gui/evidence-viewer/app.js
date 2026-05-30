const records = [
  {
    id: "vid_000001",
    type: "video",
    name: "20260519_120000_F.mp4",
    path: "E:/BLACKVUE/Record/20260519_120000_F.mp4",
    source: "SD001 BlackVue microSD",
    sourceKind: "mounted-volume",
    vendor: "BlackVue",
    parser: "blackvue_channel_suffix",
    status: "important",
    reviewed: false,
    report: true,
    hashStatus: "complete",
    hash: "a964a0f1d7b5...e19c",
    fullHash: "a964a0f1d7b51d4cb912a0e27f365bf87d7512e409d7f7c76d16cf34b8f3e19c",
    size: "184.2 MB",
    timestamp: "2026-05-19 12:00:00",
    acquiredAt: "2026-05-19 13:12:04 KST",
    readMode: "source mounted read-only",
    offset: "logical file",
    codec: "H.264 / MP4",
    duration: 86,
    channel: "front",
    validation: "ffprobe-video-stream-confirmed",
    note: "Impact moment visible at 00:00:27.400",
    events: [27.4, 28.1, 31.8],
    scene: "road",
  },
  {
    id: "vid_000002",
    type: "video",
    name: "20260519_120000_R.mp4",
    path: "E:/BLACKVUE/Record/20260519_120000_R.mp4",
    source: "SD001 BlackVue microSD",
    sourceKind: "mounted-volume",
    vendor: "BlackVue",
    parser: "blackvue_channel_suffix",
    status: "reviewed",
    reviewed: true,
    report: false,
    hashStatus: "complete",
    hash: "81fc8a3a03bd...93af",
    fullHash: "81fc8a3a03bd816b6b08303a0b9d6910710f69ee127cf82de347e88f97bd93af",
    size: "171.6 MB",
    timestamp: "2026-05-19 12:00:00",
    acquiredAt: "2026-05-19 13:12:04 KST",
    readMode: "source mounted read-only",
    offset: "logical file",
    codec: "H.264 / MP4",
    duration: 86,
    channel: "rear",
    validation: "ffprobe-video-stream-confirmed",
    note: "Rear channel synchronized with front clip",
    events: [28.0],
    scene: "rear",
  },
  {
    id: "vid_000117",
    type: "video",
    name: "ch02_20260519_115830.dav",
    path: "D:/EXPORT/Dahua/ch02_20260519_115830.dav",
    source: "HDD001 NVR export",
    sourceKind: "folder",
    vendor: "Dahua",
    parser: "dahua_dav_signature",
    status: "needs_verification",
    reviewed: false,
    report: false,
    hashStatus: "skipped",
    hash: "pending selected hash",
    fullHash: "pending selected hash",
    size: "422.0 MB",
    timestamp: "2026-05-19 11:58:30",
    acquiredAt: "2026-05-19 13:48:22 KST",
    readMode: "copied NVR export",
    offset: "logical file",
    codec: "Dahua DAV / H.264 candidate",
    duration: 242,
    channel: "cctv-02",
    validation: "candidate needs playback verification",
    note: "Native DAV export, preserve original and export derived MP4",
    events: [66.2, 141.4, 177.5],
    scene: "parking",
  },
  {
    id: "img_000041",
    type: "photo",
    name: "frame_capture_20260519_120027.jpg",
    path: "C:/Cases/FT-2026-0519/artifacts/thumbnails/frame_capture_20260519_120027.jpg",
    source: "Derived from vid_000001",
    sourceKind: "derived",
    vendor: "Derived",
    parser: "frame_capture",
    status: "reviewed",
    reviewed: true,
    report: true,
    hashStatus: "complete",
    hash: "d5eb2fb75cc4...13a1",
    fullHash: "d5eb2fb75cc4de304d9db693a759315ee815bb420bdf6f71ebf0c232b44e13a1",
    size: "412 KB",
    timestamp: "2026-05-19 12:00:27",
    acquiredAt: "2026-05-19 14:10:31 KST",
    readMode: "derived artifact",
    offset: "parent vid_000001 @ 00:00:27.400",
    codec: "JPEG still",
    duration: 1,
    channel: "front",
    validation: "derived artifact",
    note: "Report still frame, original clip linked",
    events: [],
    scene: "still",
  },
  {
    id: "carve_000009",
    type: "candidate",
    name: "carved_000009_offset_771751936.mp4",
    path: "C:/Cases/FT-2026-0519/artifacts/carved/carved_000009_offset_771751936.mp4",
    source: "E01 exported raw image",
    sourceKind: "raw-image",
    vendor: "Generic",
    parser: "mp4_ftyp_carver",
    status: "candidate",
    reviewed: false,
    report: false,
    hashStatus: "complete",
    hash: "55fd0fd9b3cf...7b41",
    fullHash: "55fd0fd9b3cfa48fba1f134d1f35dc6f5d5aef4e50f018bf3e11a4ff9c1c7b41",
    size: "27.4 MB",
    timestamp: "unknown",
    acquiredAt: "2026-05-19 15:06:11 KST",
    readMode: "E01 exported raw image",
    offset: "771751936 bytes",
    codec: "MP4 ftyp/moov candidate",
    duration: 18,
    channel: "unknown",
    validation: "candidate-unvalidated",
    note: "Contiguous MP4 signature; verify moov/mdat and playback",
    events: [1.2, 14.5],
    scene: "damaged",
  },
  {
    id: "img_000088",
    type: "photo",
    name: "parking_lot_plate_crop.jpg",
    path: "C:/Cases/FT-2026-0519/artifacts/derived/parking_lot_plate_crop.jpg",
    source: "Derived from vid_000117",
    sourceKind: "derived",
    vendor: "Derived",
    parser: "photo_review",
    status: "important",
    reviewed: false,
    report: true,
    hashStatus: "complete",
    hash: "1940ce411cf9...f801",
    fullHash: "1940ce411cf9383f8ee7a917d68fa1d7e45c18db4562ef2dcf556e771aa7f801",
    size: "620 KB",
    timestamp: "2026-05-19 12:00:48",
    acquiredAt: "2026-05-19 14:22:05 KST",
    readMode: "derived artifact",
    offset: "parent vid_000117 @ 00:02:18.500",
    codec: "JPEG still",
    duration: 1,
    channel: "cctv-02",
    validation: "derived artifact",
    note: "Contrast-adjusted copy, original frame retained",
    events: [],
    scene: "plate",
  },
];

const translations = {
  ko: {
    "app.title": "FrameTrace 증거 뷰어",
    "app.subtitle": "증거 뷰어",
    "case.summaryAria": "사건 요약",
    "case.id": "사건 FT-2026-0519",
    "case.description": "의뢰인 CCTV/블랙박스 입수",
    "case.prototype": "프로토타입 세션",
    "lang.switch": "언어 전환",
    "sync.title": "동기화 채널 보기 전환",
    "package.title": "사건 패키징",
    "sources.aria": "증거 소스",
    "browser.aria": "증거 브라우저",
    "filters.aria": "증거 필터",
    "table.aria": "색인된 증거 파일",
    "viewer.aria": "증거 뷰어",
    "viewer.canvas": "선택 증거 미리보기",
    "inspector.aria": "포렌식 인스펙터",
    "panel.evidence": "증거",
    "panel.reviewQueue": "검토 큐",
    "panel.fileState": "파일 상태",
    "panel.metadata": "메타데이터",
    "panel.outputQueue": "출력 큐",
    "panel.sessionActivity": "세션 활동",
    "search.label": "검색",
    "search.placeholder": "경로, 해시, 파서, 메모",
    "table.status": "상태",
    "table.time": "시각",
    "table.file": "파일",
    "table.type": "유형",
    "table.preview": "미리보기",
    "table.size": "크기",
    "table.previewFor": "{name} 미리보기",
    "overlay.metadata": "메타",
    "overlay.levels": "레벨",
    "overlay.compare": "비교",
    "transport.prevFile": "이전 파일",
    "transport.prevFrame": "이전 프레임",
    "transport.playPause": "재생/일시정지",
    "transport.nextFrame": "다음 프레임",
    "transport.nextFile": "다음 파일",
    "transport.speed": "재생 속도",
    "transport.zoom": "확대",
    "action.reviewed": "검토완료",
    "action.important": "중요",
    "action.report": "보고서",
    "action.frame": "프레임",
    "action.queueVerify": "검증 대기",
    "stat.files": "파일",
    "stat.videos": "영상",
    "stat.photos": "사진",
    "stat.carved": "복구 후보",
    "stat.important": "중요",
    "stat.verify": "검증 필요",
    "stat.report": "보고서",
    "stat.reviewed": "검토완료",
    "filter.all": "전체",
    "filter.video": "영상",
    "filter.photo": "사진",
    "filter.candidate": "복구",
    "filter.needs_verification": "검증",
    "filter.important": "중요",
    "filter.report": "보고서",
    "queue.unreviewed": "미검토",
    "queue.important": "중요",
    "queue.report": "보고서 포함",
    "queue.needs_verification": "검증 필요",
    "queue.triage": "1차 검토",
    "queue.active": "활성 검토",
    "source.all": "전체 증거",
    "source.caseWideIndex": "사건 전체 색인",
    "empty.noMatches": "이 보기와 일치하는 증거가 없습니다.",
    "status.unreviewed": "열림",
    "status.reviewed": "완료",
    "status.important": "중요",
    "status.needs_verification": "검증",
    "status.candidate": "후보",
    "type.video": "영상",
    "type.photo": "사진",
    "type.candidate": "복구",
    "badge.original": "원본",
    "badge.derived": "파생",
    "badge.candidate": "후보",
    "meta.id": "ID",
    "meta.source": "소스",
    "meta.path": "경로",
    "meta.vendor": "제조사",
    "meta.parser": "파서",
    "meta.hash": "해시",
    "meta.hashState": "해시 상태",
    "meta.acquired": "입수시각",
    "meta.readMode": "읽기모드",
    "meta.offset": "오프셋",
    "meta.codec": "코덱",
    "meta.output": "출력",
    "meta.channel": "채널",
    "meta.duration": "길이",
    "meta.still": "정지 이미지",
    "meta.notRecorded": "기록 없음",
    "output.noneQueued": "대기 중 작업 없음",
    "output.mp4Queued": "MP4 출력 대기",
    "output.aviQueued": "AVI 출력 대기",
    "output.frameQueued": "프레임 캡처 대기",
    "output.validationQueued": "검증 대기",
    "activity.prototypeCaseOpened": "프로토타입 사건 열림",
    "activity.prototypeCaseOpenedDetail": "GUI 검토용 mock 사건 색인 로드",
    "activity.parserCatalogStaged": "파서 카탈로그 준비",
    "activity.parserCatalogStagedDetail": "13개 소스 유형 표현",
    "activity.selectedEvidence": "증거 선택",
    "activity.playbackSpeed": "재생 속도",
    "activity.viewerOverlay": "뷰어 오버레이",
    "activity.markedReviewed": "검토완료 표시",
    "activity.markedImportant": "중요 표시",
    "activity.addedToReport": "보고서 포함",
    "activity.removedFromReport": "보고서 제외",
    "activity.mp4Queued": "MP4 출력 대기",
    "activity.aviQueued": "AVI 출력 대기",
    "activity.frameQueued": "프레임 캡처 대기",
    "activity.validationQueued": "검증 대기",
    "activity.synchronizedView": "동기화 보기",
    "activity.packageQueued": "패키지 대기",
    "activity.enabled": "활성화",
    "activity.disabled": "비활성화",
    "activity.packageDetail": "검토 세트, 보고서 세트, 매니페스트",
    "canvas.derivedView": "파생 보기",
    "syncPane.a": "A",
    "syncPane.b": "B",
    values: {
      "mounted-volume": "마운트 볼륨",
      folder: "폴더",
      derived: "파생",
      "raw-image": "raw 이미지",
      "ffprobe-video-stream-confirmed": "ffprobe 영상 스트림 확인",
      "Impact moment visible at 00:00:27.400": "충격 시점이 00:00:27.400에 보임",
      "Rear channel synchronized with front clip": "후방 채널이 전방 클립과 동기화됨",
      "candidate needs playback verification": "재생 검증이 필요한 후보",
      "Native DAV export, preserve original and export derived MP4": "DAV 원본은 보존하고 파생 MP4 출력 필요",
      "derived artifact": "파생 산출물",
      "Report still frame, original clip linked": "보고서용 정지 프레임, 원본 클립 연결됨",
      "candidate-unvalidated": "미검증 후보",
      "Contiguous MP4 signature; verify moov/mdat and playback": "연속 MP4 시그니처, moov/mdat 및 재생 검증 필요",
      "Contrast-adjusted copy, original frame retained": "대비 보정 사본, 원본 프레임 보존",
      "source mounted read-only": "소스 읽기전용 마운트",
      "copied NVR export": "복사된 NVR 내보내기",
      "E01 exported raw image": "E01 추출 raw 이미지",
      "logical file": "논리 파일",
      "parent vid_000001 @ 00:00:27.400": "상위 vid_000001 @ 00:00:27.400",
      "parent vid_000117 @ 00:02:18.500": "상위 vid_000117 @ 00:02:18.500",
      "front": "전방",
      "rear": "후방",
      "unknown": "알 수 없음",
      "complete": "완료",
      "skipped": "건너뜀",
      "pending selected hash": "선택 해시 대기",
    },
    sources: {
      "SD001 BlackVue microSD": "SD001 BlackVue microSD",
      "HDD001 NVR export": "HDD001 NVR 내보내기",
      "Derived from vid_000001": "vid_000001 파생",
      "E01 exported raw image": "E01 추출 raw 이미지",
      "Derived from vid_000117": "vid_000117 파생",
    },
  },
  en: {
    "app.title": "FrameTrace Evidence Viewer",
    "app.subtitle": "Evidence Viewer",
    "case.summaryAria": "Case summary",
    "case.id": "Case FT-2026-0519",
    "case.description": "Client CCTV and dashcam intake",
    "case.prototype": "Prototype session",
    "lang.switch": "Switch language",
    "sync.title": "Toggle synchronized channel view",
    "package.title": "Package case",
    "sources.aria": "Evidence sources",
    "browser.aria": "Evidence browser",
    "filters.aria": "Evidence filters",
    "table.aria": "Indexed evidence files",
    "viewer.aria": "Evidence viewer",
    "viewer.canvas": "Selected evidence preview",
    "inspector.aria": "Forensic inspector",
    "panel.evidence": "Evidence",
    "panel.reviewQueue": "Review Queue",
    "panel.fileState": "File State",
    "panel.metadata": "Metadata",
    "panel.outputQueue": "Output Queue",
    "panel.sessionActivity": "Session Activity",
    "search.label": "Search",
    "search.placeholder": "path, hash, parser, note",
    "table.status": "Status",
    "table.time": "Time",
    "table.file": "File",
    "table.type": "Type",
    "table.preview": "Preview",
    "table.size": "Size",
    "table.previewFor": "{name} preview",
    "overlay.metadata": "Meta",
    "overlay.levels": "Levels",
    "overlay.compare": "Compare",
    "transport.prevFile": "Previous file",
    "transport.prevFrame": "Previous frame",
    "transport.playPause": "Play or pause",
    "transport.nextFrame": "Next frame",
    "transport.nextFile": "Next file",
    "transport.speed": "Playback speed",
    "transport.zoom": "Zoom",
    "action.reviewed": "Reviewed",
    "action.important": "Important",
    "action.report": "Report",
    "action.frame": "Frame",
    "action.queueVerify": "Queue Verify",
    "stat.files": "Files",
    "stat.videos": "Videos",
    "stat.photos": "Photos",
    "stat.carved": "Carved",
    "stat.important": "Important",
    "stat.verify": "Verify",
    "stat.report": "Report",
    "stat.reviewed": "Reviewed",
    "filter.all": "All",
    "filter.video": "Video",
    "filter.photo": "Photo",
    "filter.candidate": "Carved",
    "filter.needs_verification": "Verify",
    "filter.important": "Important",
    "filter.report": "Report",
    "queue.unreviewed": "Unreviewed",
    "queue.important": "Important",
    "queue.report": "Report set",
    "queue.needs_verification": "Needs verification",
    "queue.triage": "triage lane",
    "queue.active": "active review lane",
    "source.all": "All evidence",
    "source.caseWideIndex": "case-wide index",
    "empty.noMatches": "No matching evidence in this view.",
    "status.unreviewed": "Open",
    "status.reviewed": "Done",
    "status.important": "Key",
    "status.needs_verification": "Verify",
    "status.candidate": "Candidate",
    "type.video": "Video",
    "type.photo": "Photo",
    "type.candidate": "Carved",
    "badge.original": "ORIGINAL",
    "badge.derived": "DERIVED",
    "badge.candidate": "CANDIDATE",
    "meta.id": "ID",
    "meta.source": "Source",
    "meta.path": "Path",
    "meta.vendor": "Vendor",
    "meta.parser": "Parser",
    "meta.hash": "Hash",
    "meta.hashState": "Hash State",
    "meta.acquired": "Acquired",
    "meta.readMode": "Read Mode",
    "meta.offset": "Offset",
    "meta.codec": "Codec",
    "meta.output": "Output",
    "meta.channel": "Channel",
    "meta.duration": "Duration",
    "meta.still": "still",
    "meta.notRecorded": "not recorded",
    "output.noneQueued": "none queued",
    "output.mp4Queued": "MP4 export queued",
    "output.aviQueued": "AVI export queued",
    "output.frameQueued": "frame capture queued",
    "output.validationQueued": "validation queued",
    "activity.prototypeCaseOpened": "Prototype case opened",
    "activity.prototypeCaseOpenedDetail": "mock case index loaded for GUI review",
    "activity.parserCatalogStaged": "Parser catalog staged",
    "activity.parserCatalogStagedDetail": "13 source lanes represented",
    "activity.selectedEvidence": "Selected evidence",
    "activity.playbackSpeed": "Playback speed",
    "activity.viewerOverlay": "Viewer overlay",
    "activity.markedReviewed": "Marked reviewed",
    "activity.markedImportant": "Marked important",
    "activity.addedToReport": "Added to report",
    "activity.removedFromReport": "Removed from report",
    "activity.mp4Queued": "MP4 export queued",
    "activity.aviQueued": "AVI export queued",
    "activity.frameQueued": "Frame capture queued",
    "activity.validationQueued": "Validation queued",
    "activity.synchronizedView": "Synchronized view",
    "activity.packageQueued": "Package queued",
    "activity.enabled": "enabled",
    "activity.disabled": "disabled",
    "activity.packageDetail": "review set, report set, manifests",
    "canvas.derivedView": "DERIVED VIEW",
    "syncPane.a": "A",
    "syncPane.b": "B",
    values: {},
    sources: {},
  },
};

const filters = ["all", "video", "photo", "candidate", "needs_verification", "important", "report"];

const state = {
  locale: localStorage.getItem("frametrace.locale") || "ko",
  activeFilter: "all",
  selectedId: records[0].id,
  playback: 0,
  playing: false,
  speed: 1,
  zoom: 1,
  activeOverlay: "metadata",
  activeSource: "all",
  syncView: false,
  activity: [
    { titleKey: "activity.prototypeCaseOpened", detailKey: "activity.prototypeCaseOpenedDetail" },
    { titleKey: "activity.parserCatalogStaged", detailKey: "activity.parserCatalogStagedDetail" },
  ],
};

const els = {
  statsGrid: document.getElementById("statsGrid"),
  sourceList: document.getElementById("sourceList"),
  queueList: document.getElementById("queueList"),
  filterTabs: document.getElementById("filterTabs"),
  searchInput: document.getElementById("searchInput"),
  fileRows: document.getElementById("fileRows"),
  activeKind: document.getElementById("activeKind"),
  activeName: document.getElementById("activeName"),
  canvas: document.getElementById("viewerCanvas"),
  timelineHead: document.getElementById("timelineHead"),
  timelineRail: document.getElementById("timelineRail"),
  timelineRange: document.getElementById("timelineRange"),
  timelineEvents: document.getElementById("timelineEvents"),
  timelineScale: document.getElementById("timelineScale"),
  frameTimecode: document.getElementById("frameTimecode"),
  frameBadge: document.getElementById("frameBadge"),
  statusCard: document.getElementById("statusCard"),
  metaList: document.getElementById("metaList"),
  activityList: document.getElementById("activityList"),
  playButton: document.getElementById("playButton"),
  prevButton: document.getElementById("prevButton"),
  nextButton: document.getElementById("nextButton"),
  stepBackButton: document.getElementById("stepBackButton"),
  stepForwardButton: document.getElementById("stepForwardButton"),
  speedSelect: document.getElementById("speedSelect"),
  zoomInput: document.getElementById("zoomInput"),
  languageButton: document.getElementById("languageButton"),
};

let timer = null;

function t(key, replacements = {}) {
  const locale = translations[state.locale] ? state.locale : "ko";
  const text = translations[locale][key] ?? translations.en[key] ?? key;
  return String(text).replace(/\{(\w+)\}/g, (_, name) => replacements[name] ?? "");
}

function valueLabel(value) {
  const locale = translations[state.locale] ? state.locale : "ko";
  return translations[locale].values?.[value] ?? value;
}

function sourceLabel(value) {
  const locale = translations[state.locale] ? state.locale : "ko";
  return translations[locale].sources?.[value] ?? value;
}

function applyLocalization() {
  document.documentElement.lang = state.locale;
  document.title = t("app.title");
  document.querySelectorAll("[data-i18n]").forEach((element) => {
    element.textContent = t(element.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((element) => {
    element.setAttribute("placeholder", t(element.dataset.i18nPlaceholder));
  });
  document.querySelectorAll("[data-i18n-title]").forEach((element) => {
    element.setAttribute("title", t(element.dataset.i18nTitle));
  });
  document.querySelectorAll("[data-i18n-aria-label]").forEach((element) => {
    element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
  });
  els.languageButton.textContent = state.locale === "ko" ? "EN" : "KO";
}

function selectedRecord() {
  return records.find((record) => record.id === state.selectedId) || records[0];
}

function filteredRecords() {
  const query = els.searchInput.value.trim().toLowerCase();
  return records.filter((record) => {
    if (state.activeSource !== "all" && record.source !== state.activeSource) return false;
    if (state.activeFilter === "video" && record.type !== "video") return false;
    if (state.activeFilter === "photo" && record.type !== "photo") return false;
    if (state.activeFilter === "candidate" && record.type !== "candidate") return false;
    if (state.activeFilter === "unreviewed" && record.reviewed) return false;
    if (
      state.activeFilter === "needs_verification"
      && record.status !== "needs_verification"
      && record.status !== "candidate"
    ) return false;
    if (state.activeFilter === "important" && record.status !== "important") return false;
    if (state.activeFilter === "report" && !record.report) return false;
    if (!query) return true;
    const haystack = [
      record.id,
      record.name,
      record.path,
      record.vendor,
      record.parser,
      record.hash,
      record.note,
      record.source,
      record.validation,
    ].join(" ").toLowerCase();
    return haystack.includes(query);
  });
}

function renderStats() {
  const counts = {
    total: records.length,
    video: records.filter((r) => r.type === "video").length,
    photo: records.filter((r) => r.type === "photo").length,
    candidate: records.filter((r) => r.type === "candidate").length,
    important: records.filter((r) => r.status === "important").length,
    verify: records.filter((r) => r.status === "needs_verification" || r.status === "candidate").length,
    report: records.filter((r) => r.report).length,
    reviewed: records.filter((r) => r.reviewed).length,
  };
  const stats = [
    ["stat.files", counts.total],
    ["stat.videos", counts.video],
    ["stat.photos", counts.photo],
    ["stat.carved", counts.candidate],
    ["stat.important", counts.important],
    ["stat.verify", counts.verify],
    ["stat.report", counts.report],
    ["stat.reviewed", `${counts.reviewed}/${counts.total}`],
  ];
  els.statsGrid.innerHTML = stats.map(([label, value]) => (
    `<div class="stat"><strong>${value}</strong><span>${t(label)}</span></div>`
  )).join("");
}

function renderSources() {
  const sources = [...new Set(records.map((record) => record.source))];
  const rows = [
    { name: "all", label: t("source.all"), count: records.length, sub: t("source.caseWideIndex") },
    ...sources.map((source) => {
      const sourceRecords = records.filter((record) => record.source === source);
      return {
        name: source,
        label: sourceLabel(source),
        count: sourceRecords.length,
        sub: valueLabel(sourceRecords[0].sourceKind),
      };
    }),
  ];
  els.sourceList.innerHTML = rows.map((row) => (
    `<div class="source-item ${state.activeSource === row.name ? "active" : ""}" data-source="${escapeAttr(row.name)}" role="button" tabindex="0" aria-pressed="${state.activeSource === row.name}">
      <div class="source-line"><span>${escapeHtml(row.label)}</span><strong>${row.count}</strong></div>
      <div class="source-sub">${escapeHtml(row.sub)}</div>
    </div>`
  )).join("");
  els.sourceList.querySelectorAll(".source-item").forEach((item) => {
    item.addEventListener("click", () => {
      state.activeSource = item.dataset.source;
      ensureSelectionVisible();
      renderAll();
    });
    item.addEventListener("keydown", activateOnKeyboard(() => {
      state.activeSource = item.dataset.source;
      ensureSelectionVisible();
      renderAll();
    }));
  });
}

function renderQueue() {
  const queues = [
    ["unreviewed", records.filter((r) => !r.reviewed).length],
    ["important", records.filter((r) => r.status === "important").length],
    ["report", records.filter((r) => r.report).length],
    ["needs_verification", records.filter((r) => r.status === "needs_verification" || r.status === "candidate").length],
  ];
  els.queueList.innerHTML = queues.map(([key, count]) => (
    `<div class="queue-item ${state.activeFilter === key ? "active" : ""}" data-filter="${key}" role="button" tabindex="0" aria-pressed="${state.activeFilter === key}">
      <div class="queue-line"><span>${t(`queue.${key}`)}</span><strong>${count}</strong></div>
      <div class="queue-sub">${key === "unreviewed" ? t("queue.triage") : t("queue.active")}</div>
    </div>`
  )).join("");
  els.queueList.querySelectorAll(".queue-item").forEach((item) => {
    item.addEventListener("click", () => {
      state.activeFilter = item.dataset.filter;
      ensureSelectionVisible();
      renderAll();
    });
    item.addEventListener("keydown", activateOnKeyboard(() => {
      state.activeFilter = item.dataset.filter;
      ensureSelectionVisible();
      renderAll();
    }));
  });
}

function renderFilters() {
  els.filterTabs.innerHTML = filters.map((key) => (
    `<button class="${state.activeFilter === key ? "active" : ""}" data-filter="${key}" role="tab" aria-selected="${state.activeFilter === key}">${t(`filter.${key}`)}</button>`
  )).join("");
  els.filterTabs.querySelectorAll("button").forEach((button) => {
    button.addEventListener("click", () => {
      state.activeFilter = button.dataset.filter;
      ensureSelectionVisible();
      renderAll();
    });
  });
}

function renderFiles() {
  const visible = filteredRecords();
  if (!visible.length) {
    els.fileRows.innerHTML = `<div class="empty-state">${t("empty.noMatches")}</div>`;
    return;
  }
  els.fileRows.innerHTML = visible.map((record) => (
    `<div class="file-row data-row ${record.id === state.selectedId ? "active" : ""}" role="row" tabindex="0" aria-selected="${record.id === state.selectedId}" data-id="${record.id}">
      <span class="status-pill status-${record.status}">${statusLabel(record.status)}</span>
      <span class="time-cell">${escapeHtml(shortTimestamp(record.timestamp))}</span>
      <span class="file-name"><strong>${escapeHtml(record.name)}</strong><span>${escapeHtml(record.path)}</span></span>
      <span class="type-cell">${typeLabel(record.type)}</span>
      <canvas class="thumb" width="132" height="84" data-thumb="${record.id}" aria-label="${escapeAttr(t("table.previewFor", { name: record.name }))}"></canvas>
      <span class="size-cell">${escapeHtml(record.size)}</span>
    </div>`
  )).join("");
  els.fileRows.querySelectorAll(".data-row").forEach((row) => {
    row.addEventListener("click", () => selectRecord(row.dataset.id));
    row.addEventListener("keydown", activateOnKeyboard(() => selectRecord(row.dataset.id)));
  });
  els.fileRows.querySelectorAll("canvas[data-thumb]").forEach((canvas) => {
    const record = records.find((item) => item.id === canvas.dataset.thumb);
    if (record) drawScene(canvas, record, 0, 1, true);
  });
}

function renderViewer() {
  const record = selectedRecord();
  els.activeKind.textContent = typeLabel(record.type);
  els.activeName.textContent = record.name;
  els.frameBadge.textContent = frameBadge(record);
  if (record.type !== "video" && record.type !== "candidate") {
    state.playing = false;
    stopTimer();
  }
  drawScene(els.canvas, record, state.playback, state.zoom, false);
  const duration = Math.max(record.duration || 1, 1);
  const pct = Math.min(100, Math.max(0, (state.playback / duration) * 100));
  els.timelineHead.style.left = `${pct}%`;
  els.timelineRange.style.left = record.report ? "22%" : "8%";
  els.timelineRange.style.width = record.report ? "28%" : "16%";
  els.timelineEvents.innerHTML = (record.events || []).map((event) => (
    `<span class="event-mark" style="left:${Math.min(98, Math.max(0, (event / duration) * 100))}%"></span>`
  )).join("");
  els.timelineScale.innerHTML = [0, duration / 4, duration / 2, (duration * 3) / 4, duration]
    .map((tick) => `<span>${formatDuration(tick)}</span>`)
    .join("");
  els.frameTimecode.textContent = formatTimecode(state.playback);
  els.playButton.innerHTML = state.playing ? "&#10074;&#10074;" : "&#9654;";
  document.querySelectorAll("[data-overlay]").forEach((button) => {
    button.classList.toggle("active", button.dataset.overlay === state.activeOverlay);
  });
}

function renderInspector() {
  const record = selectedRecord();
  els.statusCard.innerHTML = `
    <span class="status-pill status-${record.status}">${statusLabel(record.status)}</span>
    <strong>${escapeHtml(valueLabel(record.validation))}</strong>
    <span class="state-note">${escapeHtml(valueLabel(record.note))}</span>
  `;
  const fields = [
    [t("meta.id"), `<code>${escapeHtml(record.id)}</code>`],
    [t("meta.source"), escapeHtml(sourceLabel(record.source))],
    [t("meta.path"), `<code>${escapeHtml(record.path)}</code>`],
    [t("meta.vendor"), escapeHtml(record.vendor)],
    [t("meta.parser"), `<code>${escapeHtml(record.parser)}</code>`],
    [t("meta.hash"), `<code>${escapeHtml(valueLabel(record.fullHash || record.hash))}</code>`],
    [t("meta.hashState"), escapeHtml(valueLabel(record.hashStatus))],
    [t("meta.acquired"), escapeHtml(record.acquiredAt || t("meta.notRecorded"))],
    [t("meta.readMode"), escapeHtml(valueLabel(record.readMode || t("meta.notRecorded")))],
    [t("meta.offset"), escapeHtml(valueLabel(record.offset || t("meta.notRecorded")))],
    [t("meta.codec"), escapeHtml(record.codec || t("meta.notRecorded"))],
    [t("meta.output"), escapeHtml(formatOutputState(record))],
    [t("meta.channel"), escapeHtml(valueLabel(record.channel))],
    [t("meta.duration"), record.type === "photo" ? t("meta.still") : formatDuration(record.duration)],
  ];
  els.metaList.innerHTML = fields.map(([key, value]) => `<dt>${key}</dt><dd>${value}</dd>`).join("");
  els.activityList.innerHTML = state.activity.slice(0, 12).map((item) => (
    `<div class="activity-item"><strong>${escapeHtml(activityTitle(item))}</strong><span>${escapeHtml(activityDetail(item))}</span></div>`
  )).join("");
}

function renderAll() {
  applyLocalization();
  renderStats();
  renderSources();
  renderQueue();
  renderFilters();
  renderFiles();
  renderViewer();
  renderInspector();
}

function selectRecord(id) {
  state.selectedId = id;
  state.playback = 0;
  stopTimer();
  addActivity("activity.selectedEvidence", id);
  renderAll();
}

function ensureSelectionVisible() {
  const visible = filteredRecords();
  if (!visible.some((record) => record.id === state.selectedId) && visible[0]) {
    state.selectedId = visible[0].id;
    state.playback = 0;
  }
}

function updateRecord(mutator) {
  const record = selectedRecord();
  mutator(record);
  ensureSelectionVisible();
  renderAll();
}

function addActivity(titleKey, detail, detailKey = null) {
  state.activity.unshift({ titleKey, detail, detailKey });
}

function activityTitle(item) {
  return item.titleKey ? t(item.titleKey) : valueLabel(item.title);
}

function activityDetail(item) {
  if (item.detailKey) return t(item.detailKey);
  return valueLabel(item.detail || "");
}

function formatOutputState(record) {
  if (!record.outputStateKey) return t("output.noneQueued");
  const label = t(record.outputStateKey);
  return record.outputStateTime ? `${label} @ ${record.outputStateTime}` : label;
}

function togglePlay() {
  const record = selectedRecord();
  if (record.type === "photo") return;
  state.playing = !state.playing;
  if (state.playing) {
    startTimer();
  } else {
    stopTimer(false);
  }
  renderViewer();
}

function startTimer() {
  stopTimer(false);
  timer = window.setInterval(() => {
    const record = selectedRecord();
    const duration = Math.max(record.duration || 1, 1);
    state.playback += 0.25 * state.speed;
    if (state.playback >= duration) state.playback = 0;
    renderViewer();
  }, 250);
}

function stopTimer(resetPlaying = true) {
  if (timer) window.clearInterval(timer);
  timer = null;
  if (resetPlaying) state.playing = false;
}

function step(seconds) {
  const record = selectedRecord();
  const duration = Math.max(record.duration || 1, 1);
  state.playback = Math.min(duration, Math.max(0, state.playback + seconds));
  renderViewer();
}

function moveSelection(delta) {
  const visible = filteredRecords();
  const index = visible.findIndex((record) => record.id === state.selectedId);
  const next = visible[Math.min(visible.length - 1, Math.max(0, index + delta))];
  if (next) selectRecord(next.id);
}

function drawScene(canvas, record, time, zoom, thumb) {
  const ctx = canvas.getContext("2d");
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  const paired = !thumb && state.syncView ? pairedRecord(record) : null;
  if (paired) {
    drawSyncedScene(ctx, width, height, record, paired, time);
    drawCameraOverlay(ctx, width, height, record, time);
    return;
  }
  ctx.save();
  if (!thumb && zoom > 1) {
    ctx.translate(width / 2, height / 2);
    ctx.scale(zoom, zoom);
    ctx.translate(-width / 2, -height / 2);
  }
  drawSceneContent(ctx, width, height, record, time);
  ctx.restore();
  if (!thumb) drawCameraOverlay(ctx, width, height, record, time);
  if (!thumb && state.activeOverlay === "levels") drawLevelOverlay(ctx, width, height);
  if (!thumb && state.activeOverlay === "compare") drawCompareOverlay(ctx, width, height);
}

function drawSceneContent(ctx, width, height, record, time) {
  if (record.scene === "road") drawRoad(ctx, width, height, time);
  else if (record.scene === "rear") drawRear(ctx, width, height, time);
  else if (record.scene === "parking") drawParking(ctx, width, height, time);
  else if (record.scene === "still") drawStill(ctx, width, height);
  else if (record.scene === "plate") drawPlate(ctx, width, height);
  else drawDamaged(ctx, width, height, time);
}

function drawSyncedScene(ctx, width, height, primary, secondary, time) {
  ctx.fillStyle = "#101815";
  ctx.fillRect(0, 0, width, height);
  const pad = width * 0.025;
  const gap = width * 0.014;
  const paneWidth = (width - pad * 2 - gap) / 2;
  const paneHeight = paneWidth * 9 / 16;
  const top = (height - paneHeight) / 2;
  drawSyncedPane(ctx, pad, top, paneWidth, paneHeight, primary, time, t("syncPane.a"));
  drawSyncedPane(ctx, pad + paneWidth + gap, top, paneWidth, paneHeight, secondary, time, t("syncPane.b"));
}

function drawSyncedPane(ctx, x, y, width, height, record, time, label) {
  ctx.save();
  ctx.beginPath();
  ctx.rect(x, y, width, height);
  ctx.clip();
  ctx.translate(x, y);
  drawSceneContent(ctx, width, height, record, time);
  ctx.restore();
  ctx.strokeStyle = "rgba(238, 247, 244, 0.42)";
  ctx.lineWidth = 2;
  ctx.strokeRect(x, y, width, height);
  ctx.fillStyle = "rgba(14, 22, 19, 0.72)";
  ctx.fillRect(x + 12, y + 12, width * 0.42, 32);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `20px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(`${label} ${valueLabel(record.channel)}`, x + 22, y + 35);
}

function pairedRecord(record) {
  if (record.type !== "video") return null;
  if (!record.timestamp || record.timestamp === "unknown") return null;
  return records.find((item) => (
    item.id !== record.id
    && item.type === "video"
    && item.timestamp === record.timestamp
    && item.vendor === record.vendor
  )) || null;
}

function drawRoad(ctx, width, height, time) {
  const sky = ctx.createLinearGradient(0, 0, 0, height * 0.55);
  sky.addColorStop(0, "#9db8c8");
  sky.addColorStop(1, "#d4ddd8");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#657468";
  ctx.fillRect(0, height * 0.54, width, height * 0.46);
  ctx.fillStyle = "#2f3635";
  ctx.beginPath();
  ctx.moveTo(width * 0.1, height);
  ctx.lineTo(width * 0.45, height * 0.54);
  ctx.lineTo(width * 0.56, height * 0.54);
  ctx.lineTo(width * 0.91, height);
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = "#f0e8a8";
  ctx.lineWidth = width * 0.008;
  for (let i = 0; i < 8; i += 1) {
    const y = height * (0.58 + i * 0.07 + (time % 1) * 0.02);
    ctx.beginPath();
    ctx.moveTo(width * 0.5, y);
    ctx.lineTo(width * 0.5, y + height * 0.035);
    ctx.stroke();
  }
  drawCar(ctx, width * 0.63, height * 0.63, width * 0.12, "#932f2d");
  drawCar(ctx, width * 0.34, height * 0.67, width * 0.16, "#d7d9d4");
}

function drawRear(ctx, width, height, time) {
  ctx.fillStyle = "#d8ded9";
  ctx.fillRect(0, 0, width, height * 0.48);
  ctx.fillStyle = "#465452";
  ctx.fillRect(0, height * 0.48, width, height * 0.52);
  for (let i = 0; i < 9; i += 1) {
    ctx.fillStyle = i % 2 ? "#bfc7c2" : "#aeb9b3";
    ctx.fillRect(i * width * 0.13 - (time % 2) * 20, height * 0.49, width * 0.08, height * 0.51);
  }
  drawCar(ctx, width * 0.44, height * 0.6, width * 0.18, "#314f69");
}

function drawParking(ctx, width, height, time) {
  ctx.fillStyle = "#c9d1cc";
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "#6e7a72";
  ctx.fillRect(0, height * 0.4, width, height * 0.6);
  ctx.strokeStyle = "#e6e1be";
  ctx.lineWidth = width * 0.005;
  for (let i = 0; i < 7; i += 1) {
    ctx.beginPath();
    ctx.moveTo(width * (0.1 + i * 0.14), height * 0.43);
    ctx.lineTo(width * (0.03 + i * 0.14), height);
    ctx.stroke();
  }
  drawCar(ctx, width * 0.2, height * 0.54, width * 0.14, "#e6e6df");
  drawCar(ctx, width * (0.62 + Math.sin(time) * 0.03), height * 0.58, width * 0.15, "#7b2f35");
  drawPerson(ctx, width * 0.73, height * 0.52, width * 0.04);
}

function drawStill(ctx, width, height) {
  drawRoad(ctx, width, height, 27.4);
  ctx.strokeStyle = "#d9b136";
  ctx.lineWidth = width * 0.012;
  ctx.strokeRect(width * 0.58, height * 0.52, width * 0.19, height * 0.17);
}

function drawPlate(ctx, width, height) {
  drawParking(ctx, width, height, 48);
  ctx.fillStyle = "rgba(238, 242, 239, 0.88)";
  ctx.fillRect(width * 0.41, height * 0.5, width * 0.28, height * 0.1);
  ctx.fillStyle = "#1f2724";
  ctx.font = `${Math.floor(width * 0.04)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText("42A 7193", width * 0.435, height * 0.565);
  ctx.strokeStyle = "#b14d42";
  ctx.lineWidth = width * 0.01;
  ctx.strokeRect(width * 0.39, height * 0.47, width * 0.32, height * 0.16);
}

function drawDamaged(ctx, width, height, time) {
  ctx.fillStyle = "#1b211f";
  ctx.fillRect(0, 0, width, height);
  drawRoad(ctx, width, height, time);
  ctx.fillStyle = "rgba(27, 33, 31, 0.62)";
  for (let i = 0; i < 18; i += 1) {
    ctx.fillRect((i * 91) % width, (i * 53) % height, width * 0.08, height * 0.03);
  }
  ctx.strokeStyle = "#9f6c36";
  ctx.lineWidth = width * 0.006;
  ctx.beginPath();
  ctx.moveTo(width * 0.22, height * 0.23);
  ctx.lineTo(width * 0.41, height * 0.37);
  ctx.lineTo(width * 0.36, height * 0.62);
  ctx.lineTo(width * 0.66, height * 0.72);
  ctx.stroke();
}

function drawCar(ctx, x, y, size, color) {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, size, size * 0.36);
  ctx.fillStyle = shade(color, -28);
  ctx.fillRect(x + size * 0.18, y - size * 0.18, size * 0.54, size * 0.22);
  ctx.fillStyle = "#111";
  ctx.beginPath();
  ctx.arc(x + size * 0.2, y + size * 0.36, size * 0.08, 0, Math.PI * 2);
  ctx.arc(x + size * 0.8, y + size * 0.36, size * 0.08, 0, Math.PI * 2);
  ctx.fill();
}

function drawPerson(ctx, x, y, size) {
  ctx.fillStyle = "#26312d";
  ctx.beginPath();
  ctx.arc(x, y, size * 0.28, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillRect(x - size * 0.18, y + size * 0.25, size * 0.36, size * 0.82);
}

function drawCameraOverlay(ctx, width, height, record, time) {
  const timestampLabel = record.timestamp === "unknown" ? valueLabel("unknown") : record.timestamp;
  ctx.fillStyle = "rgba(0, 0, 0, 0.56)";
  ctx.fillRect(width * 0.025, height * 0.035, width * 0.31, height * 0.08);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `${Math.floor(width * 0.022)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(timestampLabel, width * 0.04, height * 0.073);
  ctx.font = `${Math.floor(width * 0.017)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(`${valueLabel(record.channel)}  ${formatTimecode(time)}`, width * 0.04, height * 0.103);
}

function drawLevelOverlay(ctx, width, height) {
  const left = width * 0.72;
  const top = height * 0.08;
  const barWidth = width * 0.18;
  ctx.fillStyle = "rgba(14, 22, 19, 0.68)";
  ctx.fillRect(left, top, barWidth, height * 0.16);
  for (let i = 0; i < 18; i += 1) {
    const value = Math.sin(i * 0.7) * 0.35 + 0.55;
    ctx.fillStyle = i % 3 === 0 ? "#d9b136" : "#8ed4cb";
    ctx.fillRect(left + 12 + i * (barWidth - 24) / 18, top + height * (0.13 - value * 0.1), 5, height * value * 0.1);
  }
}

function drawCompareOverlay(ctx, width, height) {
  ctx.fillStyle = "rgba(255, 255, 255, 0.16)";
  ctx.fillRect(width * 0.5, 0, width * 0.5, height);
  ctx.strokeStyle = "#f7efe4";
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(width * 0.5, 0);
  ctx.lineTo(width * 0.5, height);
  ctx.stroke();
  ctx.fillStyle = "rgba(14, 22, 19, 0.72)";
  ctx.fillRect(width * 0.51, height * 0.06, width * 0.19, height * 0.05);
  ctx.fillStyle = "#eaf3ef";
  ctx.font = `${Math.floor(width * 0.018)}px ${getComputedStyle(document.body).fontFamily}`;
  ctx.fillText(t("canvas.derivedView"), width * 0.525, height * 0.094);
}

function frameBadge(record) {
  if (record.sourceKind === "derived") return t("badge.derived");
  if (record.type === "candidate") return t("badge.candidate");
  return t("badge.original");
}

function statusLabel(value) {
  return t(`status.${value}`);
}

function typeLabel(value) {
  return t(`type.${value}`);
}

function shortTimestamp(value) {
  if (!value || value === "unknown") return valueLabel("unknown");
  const parts = value.split(" ");
  if (parts.length < 2) return value;
  return `${parts[0].slice(5)} ${parts[1].slice(0, 5)}`;
}

function formatDuration(seconds) {
  const value = Math.max(0, Math.floor(seconds));
  const mm = String(Math.floor(value / 60)).padStart(2, "0");
  const ss = String(value % 60).padStart(2, "0");
  return `${mm}:${ss}`;
}

function formatTimecode(seconds) {
  const ms = Math.floor((seconds % 1) * 1000);
  const total = Math.floor(seconds);
  const hh = String(Math.floor(total / 3600)).padStart(2, "0");
  const mm = String(Math.floor((total % 3600) / 60)).padStart(2, "0");
  const ss = String(total % 60).padStart(2, "0");
  return `${hh}:${mm}:${ss}.${String(ms).padStart(3, "0")}`;
}

function shade(hex, amount) {
  const number = Number.parseInt(hex.slice(1), 16);
  const r = Math.max(0, Math.min(255, (number >> 16) + amount));
  const g = Math.max(0, Math.min(255, ((number >> 8) & 0xff) + amount));
  const b = Math.max(0, Math.min(255, (number & 0xff) + amount));
  return `rgb(${r},${g},${b})`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttr(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

function activateOnKeyboard(callback) {
  return (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    callback(event);
  };
}

els.searchInput.addEventListener("input", () => {
  ensureSelectionVisible();
  renderAll();
});
els.playButton.addEventListener("click", togglePlay);
els.prevButton.addEventListener("click", () => moveSelection(-1));
els.nextButton.addEventListener("click", () => moveSelection(1));
els.stepBackButton.addEventListener("click", () => step(-1 / 30));
els.stepForwardButton.addEventListener("click", () => step(1 / 30));
els.speedSelect.addEventListener("change", () => {
  state.speed = Number(els.speedSelect.value);
  addActivity("activity.playbackSpeed", `${state.speed}x`);
});
els.zoomInput.addEventListener("input", () => {
  state.zoom = Number(els.zoomInput.value);
  renderViewer();
});
els.timelineRail.addEventListener("click", (event) => {
  const record = selectedRecord();
  const bounds = els.timelineRail.getBoundingClientRect();
  const ratio = Math.min(1, Math.max(0, (event.clientX - bounds.left) / bounds.width));
  state.playback = ratio * Math.max(record.duration || 1, 1);
  renderViewer();
});
document.querySelectorAll("[data-overlay]").forEach((button) => {
  button.addEventListener("click", () => {
    state.activeOverlay = button.dataset.overlay;
    addActivity("activity.viewerOverlay", t(`overlay.${button.dataset.overlay}`));
    renderViewer();
  });
});
document.getElementById("markReviewedButton").addEventListener("click", () => updateRecord((record) => {
  record.reviewed = true;
  if (record.status === "needs_verification") record.status = "reviewed";
  addActivity("activity.markedReviewed", record.id);
}));
document.getElementById("markImportantButton").addEventListener("click", () => updateRecord((record) => {
  record.status = "important";
  addActivity("activity.markedImportant", record.id);
}));
document.getElementById("addReportButton").addEventListener("click", () => updateRecord((record) => {
  record.report = !record.report;
  addActivity(record.report ? "activity.addedToReport" : "activity.removedFromReport", record.id);
}));
document.getElementById("exportMp4Button").addEventListener("click", () => updateRecord((record) => {
  record.outputStateKey = "output.mp4Queued";
  record.outputStateTime = formatTimecode(state.playback);
  addActivity("activity.mp4Queued", record.id);
}));
document.getElementById("exportAviButton").addEventListener("click", () => updateRecord((record) => {
  record.outputStateKey = "output.aviQueued";
  record.outputStateTime = formatTimecode(state.playback);
  addActivity("activity.aviQueued", record.id);
}));
document.getElementById("captureFrameButton").addEventListener("click", () => updateRecord((record) => {
  record.outputStateKey = "output.frameQueued";
  record.outputStateTime = formatTimecode(state.playback);
  addActivity("activity.frameQueued", `${record.id} ${formatTimecode(state.playback)}`);
}));
document.getElementById("verifyButton").addEventListener("click", () => updateRecord((record) => {
  if (record.status === "candidate" || record.status === "needs_verification") {
    record.outputStateKey = "output.validationQueued";
    record.outputStateTime = null;
  }
  addActivity("activity.validationQueued", record.id);
}));
document.getElementById("syncViewButton").addEventListener("click", (event) => {
  state.syncView = !state.syncView;
  event.currentTarget.classList.toggle("active", state.syncView);
  addActivity("activity.synchronizedView", null, state.syncView ? "activity.enabled" : "activity.disabled");
  renderAll();
});
els.languageButton.addEventListener("click", () => {
  state.locale = state.locale === "ko" ? "en" : "ko";
  localStorage.setItem("frametrace.locale", state.locale);
  renderAll();
});
document.getElementById("packageButton").addEventListener("click", () => addActivity("activity.packageQueued", null, "activity.packageDetail"));
document.addEventListener("keydown", (event) => {
  if (event.target && ["INPUT", "SELECT"].includes(event.target.tagName)) return;
  if (event.key === " ") {
    event.preventDefault();
    togglePlay();
  } else if (event.key === "ArrowRight") {
    step(1 / 30);
  } else if (event.key === "ArrowLeft") {
    step(-1 / 30);
  } else if (event.key.toLowerCase() === "n") {
    moveSelection(1);
  } else if (event.key.toLowerCase() === "p") {
    moveSelection(-1);
  }
});

renderAll();
