// Deterministic local mock corpus for the static FrameTrace evidence viewer.
const seedRecords = [
  {
    id: "vid_000001",
    type: "video",
    name: "20260519_120000_F.mp4",
    path: "[redacted-source:vid_000001]",
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
    path: "[redacted-source:vid_000002]",
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
    path: "[redacted-source:vid_000117]",
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
    path: "artifacts/thumbnails/frame_capture_20260519_120027.jpg",
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
    path: "artifacts/carved/carved_000009_offset_771751936.mp4",
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
    path: "artifacts/derived/parking_lot_plate_crop.jpg",
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

window.FrameTraceRecords = generateMockInventory(seedRecords, 10000).map(applyDefaultPathRedaction);

function generateMockInventory(seed, count) {
  const statuses = ["unreviewed", "reviewed", "important", "needs_verification", "candidate"];
  const sources = [
    "SD001 BlackVue microSD",
    "HDD001 NVR export",
    "E01 exported raw image",
    "Recovered filesystem",
  ];
  return Array.from({ length: count }, (_, index) => {
    const base = seed[index % seed.length];
    const status = statuses[index % statuses.length];
    const source = sources[index % sources.length];
    const extension = base.name.split(".").pop() || "bin";
    const folder = String(Math.floor(index / 250) + 1).padStart(3, "0");
    const sequence = String(index + 1).padStart(6, "0");
    const idPrefix = base.type === "photo" ? "img" : base.type === "candidate" ? "carve" : "vid";
    const hash = pseudoHash(index);
    const sizeBytes = 120000 + ((index * 7919) % 900000000);
    return {
      ...base,
      id: index < seed.length ? base.id : `${idPrefix}_${sequence}`,
      name: index < seed.length ? base.name : `${base.name.replace(`.${extension}`, "")}_${sequence}.${extension}`,
      path: index < seed.length ? base.path : `${source}/case-folder-${folder}/${base.name}`,
      source,
      sourceKind: source.includes("E01") ? "raw-image" : source.includes("Recovered") ? "derived" : "mounted-volume",
      status,
      reviewed: status === "reviewed",
      report: index % 17 === 0 || base.report,
      hashStatus: index % 11 === 0 ? "pending selected hash" : "complete",
      hash: `${hash.slice(0, 12)}...${hash.slice(-4)}`,
      fullHash: hash,
      size: index < seed.length ? base.size : formatBytes(sizeBytes),
      sizeBytes: index < seed.length ? parseSize(base.size) : sizeBytes,
      timestamp: `2026-05-${String(1 + (index % 28)).padStart(2, "0")} ${String(index % 24).padStart(2, "0")}:${String((index * 7) % 60).padStart(2, "0")}:00`,
      acquiredAt: "2026-05-19 13:12:04 KST",
      validation: status === "candidate" ? "candidate-unvalidated" : base.validation,
      note: status === "candidate" ? "Contiguous candidate awaiting validation" : base.note,
      duration: Math.max(base.duration || 1, 1),
    };
  });
}

function applyDefaultPathRedaction(record) {
  return {
    ...record,
    path: distributablePathLabel(record.path, record.id),
  };
}

function distributablePathLabel(path, id) {
  const value = String(path || "");
  const normalized = value.replaceAll("\\", "/");
  const artifactIndex = normalized.indexOf("/artifacts/");
  if (artifactIndex >= 0) return normalized.slice(artifactIndex + 1);
  if (/^(file:)?\/\//.test(normalized) || /^\/|^[A-Za-z]:\//.test(normalized)) {
    return `[redacted-source:${id || "record"}]`;
  }
  return value;
}

function pseudoHash(index) {
  const alphabet = "0123456789abcdef";
  let value = (index + 1) * 2654435761;
  let out = "";
  for (let i = 0; i < 64; i += 1) {
    value = (value * 1664525 + 1013904223) >>> 0;
    out += alphabet[value & 15];
  }
  return out;
}

function formatBytes(bytes) {
  if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(bytes / 1000))} KB`;
}

function parseSize(value) {
  const match = String(value).match(/^([\d.]+)\s*(KB|MB|GB)$/i);
  if (!match) return 0;
  const number = Number(match[1]);
  const unit = match[2].toUpperCase();
  if (unit === "GB") return Math.round(number * 1_000_000_000);
  if (unit === "MB") return Math.round(number * 1_000_000);
  return Math.round(number * 1_000);
}
