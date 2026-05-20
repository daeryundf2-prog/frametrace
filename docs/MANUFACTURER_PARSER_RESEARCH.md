# Manufacturer Parser Research

Last searched: 2026-05-19

This document tracks manufacturer-specific parser targets for FrameTrace. The goal is not to reverse every format immediately, but to build a plugin map so evidence intake can identify the likely manufacturer, select the safest parser path, and export reviewable MP4/AVI artifacts with an audit trail.

## Parser Lanes

1. **Generic video lane**
   - MP4/MOV/AVI/MKV/TS/ASF/raw H.264/H.265.
   - Use `ffprobe`/FFmpeg first.
   - Preserve original file and export derived MP4/AVI clips.

2. **Dashcam SD-card lane**
   - Usually standard video files plus manufacturer-specific folder naming, GPS, speed, and G-sensor metadata.
   - Primary tasks: folder classification, dual-channel pairing, event type parsing, GPS/G-sensor extraction, and timeline alignment.

3. **CCTV/NVR export lane**
   - Files arrive as vendor exports such as DAV, AVE, NOV, G64/G64x, XProtect export folders, EXE self-players, ZIP packages, ASF/MKV/MP4/AVI.
   - Primary tasks: identify vendor/export type, use official player/SDK path when needed, transcode to MP4/AVI, log every conversion.

4. **DVR/NVR HDD recovery lane**
   - Raw HDDs may use proprietary circular file systems.
   - Primary tasks: device/image identification, sector/range map, metadata reconstruction, deleted/expired/overwritten status, carving with timestamps.
   - This lane needs sample disks and legal/forensic validation before claims.

## Priority Matrix

| Priority | Vendor / family | Evidence likely to arrive | Current public/official signal | Parser strategy |
| --- | --- | --- | --- | --- |
| P0 | Generic MP4/MOV/AVI/TS/H.264/H.265 | Dashcam SD cards, exported CCTV clips | FFmpeg documents `ffmpeg`, `ffprobe`, libraries, formats, codecs, muxers, demuxers | Already started. Expand detection, broken-file repair, proxy/clips. |
| P0 | Hikvision | NVR/DVR HDD, iVMS/VSPlayer exports, MP4/AVI/ASF-like files, proprietary FS | Hikvision publishes Device Network SDK and Player SDK. Player SDK covers live view, recording playback, stream info, resolution, frame rate. Research exists on the Hikvision DVR file system. | Build `hikvision` detector: exported file path first, SDK/player-assisted conversion second, HDD FS parser later. |
| P0 | Dahua / OEM DAV | DVR/NVR export files, `.dav`, `.dav_`, `.asf`, raw disk | Dahua Smart Player documentation says it plays DAV/ASF and supports AVI/MP4/FLV/ASF/MOV/DAV/DAV_. Dahua download center has SDK category. | Build `.dav` signature probe and FFmpeg/SmartPlayer strategy. Keep original DAV, export MP4/AVI with logs. HDD recovery later. |
| P0 | BlackVue | microSD card folders, front/rear MP4, GPS/speed/G-sensor | BlackVue manual documents microSD playback in BlackVue Viewer and GPS data checking. Viewer is official. | Build dashcam folder parser, channel pairing, GPS extraction probe. |
| P0 | Thinkware / iNavi family | SD-card clips, continuous/incident/parking folders, GPS/speed | Thinkware PC Viewer is for Windows and reviews speed, location, time; Thinkware docs list recording-mode folders. iNavi says PC Viewer differs by model and manages/replays saved videos. | Build folder/event parser and metadata probe per model family. |
| P1 | FineVu | SD-card clips with GPS/G-sensor | FineVu manuals describe FineVu Player with video, GPS location, speed, G-sensor/speed graph. | Build generic dashcam parser, then add FineVu folder/model rules. |
| P1 | IROAD | SD-card clips, app/PC viewer records, GPS when present | IROAD official materials describe stored video playback/download and PC viewer GPS display when GPS exists. | Build folder/event parser and GPS probe. |
| P1 | VIOFO | MP4 clips, GPS tracks | VIOFO official page says VIOFO Player can play recorded videos and view GPS tracks. | Build MP4 + GPS metadata probe. |
| P1 | Garmin Dash Cam | MP4 and GLV files under DCIM folders | Garmin support says MP4 files are high-quality computer videos and GLV are lower-quality phone/VIRB files. | Add Garmin folder parser and GLV handling. |
| P1 | Nextbase | dashcam clips with GPS/G-sensor | Nextbase app page says MyNextbase Player can play/edit clips and view GPS maps or G-sensor data. | Add GPS/G-sensor probe after generic MP4. |
| P1 | Hanwha/Wisenet | WAVE/NVR exports, NOV/EXE/MP4/MKV/AVI | Hanwha WAVE SDK/API supports pulling live/recorded video; WAVE export supports NOV/EXE and docs mention MP4/MKV/AVI options. | Treat WAVE/NOV as proprietary export; prefer official API/export path, parse standard outputs. |
| P1 | Axis | edge SD/network-share recordings, MKV exports | Axis VAPIX Edge Storage API can export recordings or clips to playable files; export output is MKV. ONVIF is also relevant. | Build network/device export connector later; local parser handles MKV now. |
| P1 | ONVIF Profile G/T | IP camera/NVR recordings via network, not raw disks | ONVIF Profile G covers recording/storage/retrieval; Profile T covers H.264/H.265 and metadata streaming. | Implement optional acquisition connector later; useful when device is alive. |
| P2 | Uniview | EZStation exports, SD-card/NVR playback | Uniview EZStation supports recording download, SD-card playback, frame playback, and record search. | Start with official export outputs; add signature samples when available. |
| P2 | IDIS | IDIS Center/Solution Suite exports, EXE/AVI | IDIS docs say exports can be self-player EXE or AVI; IDIS Center says export saves `.exe` or `.avi`. | Parse AVI directly; catalog EXE exports and preserve bundled player metadata. |
| P2 | Bosch BVMS/VRM | ZIP/native exports, ASF/MOV/Archive Player exports, MP4 via VRM export wizard | Bosch BVMS docs describe ZIP exports, Archive Player/ASF/MOV, authenticity verification; Bosch knowledge base describes VRM export and MP4 conversion. | Prefer official export outputs. Build ZIP/native export detector. |
| P2 | Milestone XProtect | XProtect export folders/DB, AVI/MKV/media player exports | Milestone docs list XProtect format, media player exports, AVI/MKV/database sample export. | Detect XProtect export package; ingest AVI/MKV directly; note proprietary player path for DB exports. |
| P2 | Genetec Security Center | G64/G64x, ASF, MP4 exports | Genetec docs list G64, G64x, ASF, MP4; G64/G64x require Security Desk or Genetec Video Player. | Ingest MP4/ASF directly; catalog G64/G64x and require player-assisted export. |
| P2 | Avigilon ACC | AVE, AVI, native player exports | Avigilon docs/manuals describe export workflows and AVE/AVI-like outputs. | Ingest AVI directly; catalog AVE as proprietary/player-assisted. |
| P2 | Milesight | MP4/AVI/MKV/ASF/EXE exports | Milesight VMS datasheet lists export types `.mp4`, `.avi`, `.mkv`, `.asf`, `.exe`. | Ingest standard media directly; preserve EXE export metadata. |
| Research | Honeywell | Proprietary surveillance HDD file system | 2026 DFRWS/arXiv paper analyzes Honeywell proprietary surveillance file system deletion/recovery. | Track as research target; do not implement until samples exist. |

## Implementation Backlog

Current prototype status: `src/detector.rs` implements the first lightweight detector registry. It classifies likely parser lanes from extensions, path/folder names, and standard media metadata. This is intentionally conservative: it creates review/report signals and future parser routing, but it does not claim deep proprietary recovery support yet.

Current implementation also includes Korean dashcam folder signals (`상시`, `이벤트`, `주차`, `충격`), Urive/PAPAGO/FineVu-Mando path signals, and raw H.264/H.265 DVR recovery lanes. These remain routing hints until sample-backed metadata extraction and validation are added.

### Plugin registry

Add a registry with small detector functions:

```text
plugins/
  generic_media
  blackvue
  thinkware_inavi
  finevu
  iroad
  viofo
  garmin
  nextbase
  hikvision
  dahua
  hanwha_wisenet
  axis_onvif
  uniview
  idis
  bosch
  milestone
  genetec
  avigilon
  milesight
```

Each plugin should expose:

```text
detect(path | folder | image) -> confidence + evidence
index(source) -> files/events/channels/time ranges
metadata(file) -> codec/time/gps/g-sensor/vendor fields
export(file/range, mp4|avi) -> derived artifact + audit log
recover(image/ranges) -> recovered candidates, only where supported
```

### Detection rules to add first

- Extension and magic:
  - `.dav`, `.dav_` -> Dahua/DVR365 lane.
  - `.nov` -> Wisenet WAVE.
  - `.ave` -> Avigilon.
  - `.g64`, `.g64x` -> Genetec.
  - `.glv` -> Garmin low-resolution dashcam video.
  - `.blk` / XProtect package tree -> Milestone.
  - `.exe` bundled player -> preserve and classify, do not execute automatically.
- Folder signals:
  - `DCIM/Video`, `Snapshot` -> Garmin-like.
  - continuous/incident/manual/motion/parking folders -> Thinkware-like.
  - front/rear paired clip naming -> dashcam pairing lane.
- Container signals:
  - MP4/MOV with GPS/G-sensor custom boxes -> dashcam metadata lane.
  - raw H.264/H.265 Annex B start codes -> raw stream lane.

### Validation rules

- Never auto-run vendor EXE players.
- Preserve original exports and raw device images unchanged.
- Every conversion writes:
  - source path/hash
  - detected vendor/plugin
  - command/tool version
  - output path/hash
  - start/duration
  - warnings about transcoding or dropped metadata
- For proprietary HDD parsers, no admissibility claim until tested against known images.

## Sources

- FFmpeg documentation: https://www.ffmpeg.org/documentation.html
- Hikvision SDK downloads: https://www.hikvision.com/content/hikvision/us-en/support/download/sdk.html
- Hikvision Player SDK: https://www.hikvision.com/ca-fr/support/download/sdk/player-sdk--for-linux-32-bit-/
- Hikvision DVR file system paper: https://eudl.eu/pdf/10.1007/978-3-319-25512-5_13
- Dahua Smart Player: https://dahuawiki.com/NVR/Playback/Smart_Player
- Dahua Smart Player guide PDF: https://dahuawiki.com/images/4/46/SmartPlayer_User_Guide_Eng.pdf
- Dahua download center: https://www.dahuasecurity.com/download-center
- BlackVue computer viewer manual: https://manual.blackvue.com/docs/dr770x-/playing-and-managing-videos/using-your-computer-windows-macos-dr970x-2ch-lte/
- Thinkware PC Viewer: https://support.thinkware.com/hc/en-us/articles/6987549044115-How-to-Download-the-PC-Viewer
- Thinkware recorded file modes: https://support.thinkware.com/hc/en-us/articles/26015165222931--Thinkware-Cloud-How-to-view-Recorded-Video-Files
- iNavi blackbox upgrade/viewer: https://www.inavi.com/upgrades/blackboxinfo/
- FineVu manual: https://www.finevu.com/en/down/GX4K/GX4K_Web%20Manual_EN.pdf
- IROAD manual: https://iroad.kr/download/Manual/IROAD_PRIVACY_Manual_KR.pdf
- VIOFO app/player: https://www.viofo.com/pages/viofo-app
- Garmin dashcam files: https://support.garmin.com/en-US/?faq=e0CquAtGGb7LwB5eDrqQF8
- Nextbase app/player: https://nextbase.com/nbapp-start/
- Hanwha WAVE SDK/API: https://support.hanwhavisionamerica.com/hc/en-us/articles/115013501208-WAVE-SDK-API
- Hanwha WAVE export executable/NOV: https://support.hanwhavision.com/hc/en-001/articles/47286866050707-WAVE-Export-to-Executable
- Axis Edge Storage API: https://developer.axis.com/vapix/network-video/edge-storage-api/
- ONVIF Profile G: https://www.onvif.org/video/ovif-profile-g-for-edge-storage-and-retrieval/
- ONVIF Profile T: https://www.onvif.org/profiles/profile-t/
- Uniview EZStation: https://www.uniview.com/Products/Software/PC/EZStation/
- IDIS Center recorded playback/export: https://support.idisamericas.com/hc/en-us/articles/41851951410451-IDIS-Center-Recorded-video-playback
- IDIS Solution Suite manual: https://www.idisglobal.ru/files/manual/software/idis-solution-suite-standart-manual-eng.pdf
- Bosch BVMS operation manual: https://cdn.commerce.boschsecurity.com/public/documents/BVMS_12.2_Operation_Manual_enUS_122217598091.pdf
- Bosch VRM export wizard: https://knowledge.keenfinity-group.com/video-systems/article/how-to-export-vrm-recordings-of-cameras-convert--1
- Milestone export quick guide: https://www.milestonesys.com/globalassets/materials/documents/quick-guide/xprotect_smart_client_how_to_search_and_export.pdf
- Milestone MIP SDK export sample: https://doc.developer.milestonesys.com/mipsdk/samples/PluginSamples/SCExport/README.html
- Genetec video export docs: https://techdocs.genetec.com/r/en-US/Security-Center-User-Guide-5.11/Video-export
- Genetec Video Player: https://www.genetec.com/products/unified-security/omnicast/video-player
- Hanwha/Nx WAVE export formats help: https://wavevms.com/data/help/single_camera_export.html
- Milesight VMS Enterprise datasheet/product page: https://www.milesight.com/product/vms-enterprise/index
- Honeywell surveillance file system research: https://dfrws.org/presentation/forensic-analysis-of-video-data-deletion-and-recovery-in-honeywell-surveillance-file-system/
