use crate::model::SourceProfile;

pub fn detect_source_profile(
    relative_path: &str,
    extension: &str,
    format_name: Option<&str>,
) -> SourceProfile {
    let normalized = normalize_path(relative_path);
    let ext = extension.to_ascii_lowercase();

    if let Some(profile) = detect_proprietary_extension(&normalized, &ext) {
        return profile;
    }
    if let Some(profile) = detect_named_vendor(&normalized, &ext) {
        return profile;
    }
    if let Some(profile) = detect_dashcam_pattern(&normalized, &ext) {
        return profile;
    }

    let evidence = match format_name {
        Some(format_name) if !format_name.is_empty() => {
            format!("ffprobe format: {format_name}")
        }
        _ if !ext.is_empty() => format!("standard media extension: .{ext}"),
        _ => "video-like file signature".to_string(),
    };
    SourceProfile::generic_media(evidence)
}

fn detect_proprietary_extension(path: &str, extension: &str) -> Option<SourceProfile> {
    let profile = match extension {
        "dav" | "dav_" => vendor_profile(
            "cctv-nvr-export",
            "Dahua / OEM DAV",
            "dahua_dav",
            "high",
            "Preserve DAV original, then use FFmpeg or official Smart Player assisted export to MP4/AVI.",
            vec![format!("proprietary extension: .{extension}")],
        ),
        "nov" => vendor_profile(
            "cctv-nvr-export",
            "Hanwha/Wisenet WAVE",
            "hanwha_wisenet_wave",
            "high",
            "Treat NOV as proprietary WAVE export; prefer official WAVE/player assisted conversion.",
            vec!["proprietary extension: .nov".to_string()],
        ),
        "ave" => vendor_profile(
            "cctv-nvr-export",
            "Avigilon ACC",
            "avigilon_ave",
            "high",
            "Treat AVE as proprietary export; preserve original and use player-assisted export when required.",
            vec!["proprietary extension: .ave".to_string()],
        ),
        "g64" | "g64x" => vendor_profile(
            "cctv-nvr-export",
            "Genetec Security Center",
            "genetec_g64",
            "high",
            "Treat G64/G64x as proprietary export; use Genetec Video Player/Security Desk for review or conversion.",
            vec![format!("proprietary extension: .{extension}")],
        ),
        "glv" => vendor_profile(
            "dashcam-sd-card",
            "Garmin Dash Cam",
            "garmin_dashcam",
            "high",
            "Classify as Garmin low-resolution companion video; preserve alongside MP4 files.",
            vec!["Garmin GLV companion video extension".to_string()],
        ),
        "blk" => vendor_profile(
            "cctv-nvr-export",
            "Milestone XProtect",
            "milestone_xprotect",
            "medium",
            "Treat as XProtect package artifact; ingest standard media exports directly.",
            vec!["XProtect-like .blk artifact extension".to_string()],
        ),
        _ => return detect_path_extension_combo(path, extension),
    };
    Some(profile)
}

fn detect_path_extension_combo(path: &str, extension: &str) -> Option<SourceProfile> {
    if extension == "asf" && contains_any(path, &["dahua", "smartpss", "dss"]) {
        return Some(vendor_profile(
            "cctv-nvr-export",
            "Dahua / OEM DAV",
            "dahua_dav",
            "medium",
            "Dahua exports may arrive as ASF; preserve and transcode only into derived artifacts.",
            vec!["ASF file under Dahua/SmartPSS/DSS-like path".to_string()],
        ));
    }
    if extension == "mkv" && contains_any(path, &["axis", "vapix"]) {
        return Some(vendor_profile(
            "cctv-nvr-export",
            "Axis",
            "axis_onvif",
            "medium",
            "Axis edge-storage exports are commonly playable MKV; ingest directly and preserve API/export logs.",
            vec!["MKV file under Axis/VAPIX-like path".to_string()],
        ));
    }
    None
}

fn detect_named_vendor(path: &str, _extension: &str) -> Option<SourceProfile> {
    let checks: &[(&[&str], &str, &str, &str, &str, &str)] = &[
        (
            &["hikvision", "ivms", "vsplayer"],
            "cctv-nvr-export",
            "Hikvision",
            "hikvision",
            "high",
            "Prefer exported media first; use Hikvision Player SDK/VSPlayer path for proprietary streams.",
        ),
        (
            &["dahua", "smartpss", "dss"],
            "cctv-nvr-export",
            "Dahua / OEM DAV",
            "dahua_dav",
            "high",
            "Use DAV/ASF detection and official player-assisted conversion when FFmpeg cannot decode.",
        ),
        (
            &["blackvue"],
            "dashcam-sd-card",
            "BlackVue",
            "blackvue",
            "high",
            "Pair front/rear channels and extract GPS/G-sensor metadata when present.",
        ),
        (
            &["thinkware", "inavi", "아이나비"],
            "dashcam-sd-card",
            "Thinkware / iNavi",
            "thinkware_inavi",
            "high",
            "Classify recording mode folders and extract GPS/speed metadata when present.",
        ),
        (
            &["finevu", "fine-vu"],
            "dashcam-sd-card",
            "FineVu",
            "finevu",
            "high",
            "Extract player-compatible GPS/G-sensor metadata when present.",
        ),
        (
            &["iroad"],
            "dashcam-sd-card",
            "IROAD",
            "iroad",
            "high",
            "Classify event folders and GPS metadata when present.",
        ),
        (
            &["viofo"],
            "dashcam-sd-card",
            "VIOFO",
            "viofo",
            "high",
            "Use MP4 as primary media and extract GPS tracks when present.",
        ),
        (
            &["garmin"],
            "dashcam-sd-card",
            "Garmin Dash Cam",
            "garmin_dashcam",
            "high",
            "Preserve DCIM layout, MP4 files, and GLV companion files.",
        ),
        (
            &["nextbase"],
            "dashcam-sd-card",
            "Nextbase",
            "nextbase",
            "high",
            "Extract GPS map/G-sensor metadata when present.",
        ),
        (
            &["hanwha", "wisenet", "/wave/"],
            "cctv-nvr-export",
            "Hanwha/Wisenet WAVE",
            "hanwha_wisenet_wave",
            "high",
            "Prefer WAVE official export path; ingest standard MP4/MKV/AVI outputs directly.",
        ),
        (
            &["axis", "vapix"],
            "cctv-nvr-export",
            "Axis",
            "axis_onvif",
            "high",
            "Use MKV exports directly; device/API acquisition is a future connector path.",
        ),
        (
            &["onvif"],
            "cctv-nvr-export",
            "ONVIF Profile G/T",
            "axis_onvif",
            "medium",
            "Treat as network/video export evidence; local files stay immutable.",
        ),
        (
            &["uniview", "ezstation"],
            "cctv-nvr-export",
            "Uniview",
            "uniview",
            "high",
            "Start with official export outputs; add proprietary samples later.",
        ),
        (
            &["idis"],
            "cctv-nvr-export",
            "IDIS",
            "idis",
            "high",
            "Ingest AVI directly; preserve self-player exports without running them.",
        ),
        (
            &["bosch", "bvms", "vrm"],
            "cctv-nvr-export",
            "Bosch BVMS/VRM",
            "bosch_bvms",
            "high",
            "Detect ZIP/native exports and ingest ASF/MOV/MP4 derived media directly.",
        ),
        (
            &["milestone", "xprotect"],
            "cctv-nvr-export",
            "Milestone XProtect",
            "milestone_xprotect",
            "high",
            "Detect XProtect packages; ingest AVI/MKV exports directly.",
        ),
        (
            &["genetec"],
            "cctv-nvr-export",
            "Genetec Security Center",
            "genetec_g64",
            "high",
            "Ingest MP4/ASF directly; preserve G64/G64x for player-assisted review.",
        ),
        (
            &["avigilon", "/acc/"],
            "cctv-nvr-export",
            "Avigilon ACC",
            "avigilon_ave",
            "high",
            "Ingest AVI directly; preserve AVE exports for player-assisted review.",
        ),
        (
            &["milesight"],
            "cctv-nvr-export",
            "Milesight",
            "milesight",
            "high",
            "Ingest standard MP4/AVI/MKV/ASF exports directly; preserve EXE packages.",
        ),
        (
            &["honeywell"],
            "dvr-hdd-recovery",
            "Honeywell",
            "honeywell_research",
            "medium",
            "Research target only until validated sample images exist.",
        ),
    ];

    for &(needles, lane, vendor, parser, confidence, action) in checks {
        if contains_any(path, needles) {
            return Some(vendor_profile(
                lane,
                vendor,
                parser,
                confidence,
                action,
                vec![format!(
                    "path contains vendor signal: {}",
                    first_match(path, needles)
                )],
            ));
        }
    }
    None
}

fn detect_dashcam_pattern(path: &str, extension: &str) -> Option<SourceProfile> {
    if contains_any(path, &["/dcim/"]) && matches!(extension, "mp4" | "mov" | "glv") {
        return Some(vendor_profile(
            "dashcam-sd-card",
            "Garmin-like DCIM dashcam",
            "garmin_dashcam",
            "medium",
            "Preserve DCIM layout and classify MP4/GLV companion files.",
            vec!["DCIM folder with dashcam-compatible media".to_string()],
        ));
    }

    if has_blackvue_suffix(path) {
        return Some(vendor_profile(
            "dashcam-sd-card",
            "BlackVue-style dashcam",
            "blackvue",
            "medium",
            "Pair front/rear channel clips before review/export.",
            vec!["BlackVue-style front/rear event suffix".to_string()],
        ));
    }

    let recording_mode = first_match(
        path,
        &[
            "/continuous/",
            "/incident/",
            "/event/",
            "/parking/",
            "/motion/",
            "/manual/",
        ],
    );
    if !recording_mode.is_empty() && matches!(extension, "mp4" | "mov" | "avi") {
        return Some(vendor_profile(
            "dashcam-sd-card",
            "Dashcam event-folder layout",
            "thinkware_inavi",
            "medium",
            "Treat folder name as recording mode; verify model-specific metadata before making vendor claims.",
            vec![format!("dashcam recording folder signal: {recording_mode}")],
        ));
    }

    None
}

fn vendor_profile(
    lane: &str,
    vendor: &str,
    parser: &str,
    confidence: &str,
    recommended_action: &str,
    evidence: Vec<String>,
) -> SourceProfile {
    SourceProfile {
        lane: lane.to_string(),
        vendor: vendor.to_string(),
        parser: parser.to_string(),
        confidence: confidence.to_string(),
        recommended_action: recommended_action.to_string(),
        evidence,
    }
}

fn contains_any(path: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| path.contains(needle))
}

fn first_match(path: &str, needles: &[&str]) -> String {
    needles
        .iter()
        .find(|needle| path.contains(**needle))
        .map(|needle| needle.trim_matches('/').to_string())
        .unwrap_or_default()
}

fn has_blackvue_suffix(path: &str) -> bool {
    let Some(file_name) = path.rsplit('/').next() else {
        return false;
    };
    [
        "_nf.", "_nr.", "_ef.", "_er.", "_pf.", "_pr.", "_mf.", "_mr.",
    ]
    .iter()
    .any(|suffix| file_name.contains(suffix))
}

fn normalize_path(input: &str) -> String {
    let normalized = input.replace('\\', "/").to_ascii_lowercase();
    if normalized.starts_with('/') {
        normalized
    } else {
        format!("/{normalized}")
    }
}

#[cfg(test)]
mod tests {
    use super::detect_source_profile;

    #[test]
    fn detects_dahua_dav_exports() {
        let profile = detect_source_profile("export/cam01/clip.dav", "dav", None);
        assert_eq!(profile.parser, "dahua_dav");
        assert_eq!(profile.confidence, "high");
    }

    #[test]
    fn detects_blackvue_channel_suffixes() {
        let profile = detect_source_profile("Record/20260519_120000_NF.mp4", "mp4", None);
        assert_eq!(profile.parser, "blackvue");
        assert_eq!(profile.confidence, "medium");
    }

    #[test]
    fn detects_thinkware_style_event_folders() {
        let profile = detect_source_profile("continuous/20260519_120000.mp4", "mp4", None);
        assert_eq!(profile.parser, "thinkware_inavi");
        assert_eq!(profile.lane, "dashcam-sd-card");
    }

    #[test]
    fn detects_genetec_g64x_exports() {
        let profile = detect_source_profile("SecurityDesk/export.g64x", "g64x", None);
        assert_eq!(profile.parser, "genetec_g64");
    }

    #[test]
    fn falls_back_to_generic_media() {
        let profile = detect_source_profile("camera01/clip.mp4", "mp4", Some("mov,mp4"));
        assert_eq!(profile.parser, "generic_media");
        assert_eq!(profile.vendor, "Generic media");
    }

    #[test]
    fn avoids_overbroad_common_word_matches() {
        let profile = detect_source_profile("soundwave/clip.mp4", "mp4", None);
        assert_eq!(profile.parser, "generic_media");
    }
}
