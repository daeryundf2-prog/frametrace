use crate::model::SourceProfile;
use crate::util::json_escape;

#[derive(Debug, Clone)]
pub struct ParserPlugin {
    pub id: &'static str,
    pub lane: &'static str,
    pub vendor: &'static str,
    pub confidence: &'static str,
    pub path_needles: &'static [&'static str],
    pub extensions: &'static [&'static str],
    pub recommended_action: &'static str,
}

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

pub fn parser_catalog_json() -> String {
    let mut plugins = Vec::new();
    for plugin in proprietary_extension_plugins()
        .iter()
        .chain(named_vendor_plugins().iter())
    {
        plugins.push(plugin_json(plugin));
    }
    format!(
        "{{\"schema_version\":1,\"plugin_count\":{},\"plugins\":[{}]}}\n",
        plugins.len(),
        plugins.join(",")
    )
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
    for plugin in named_vendor_plugins() {
        if contains_any(path, plugin.path_needles) {
            return Some(profile_from_plugin(
                plugin,
                vec![format!(
                    "path contains vendor signal: {}",
                    first_match(path, plugin.path_needles)
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
            "BlackVue",
            "blackvue",
            "high",
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

fn profile_from_plugin(plugin: &ParserPlugin, evidence: Vec<String>) -> SourceProfile {
    vendor_profile(
        plugin.lane,
        plugin.vendor,
        plugin.id,
        plugin.confidence,
        plugin.recommended_action,
        evidence,
    )
}

fn proprietary_extension_plugins() -> &'static [ParserPlugin] {
    &[
        ParserPlugin {
            id: "dahua_dav",
            lane: "cctv-nvr-export",
            vendor: "Dahua / OEM DAV",
            confidence: "high",
            path_needles: &["dahua", "smartpss", "dss"],
            extensions: &["dav", "dav_", "asf"],
            recommended_action: "Preserve DAV original, then use FFmpeg or official Smart Player assisted export to MP4/AVI.",
        },
        ParserPlugin {
            id: "genetec_g64",
            lane: "cctv-nvr-export",
            vendor: "Genetec Security Center",
            confidence: "high",
            path_needles: &["genetec"],
            extensions: &["g64", "g64x"],
            recommended_action: "Treat G64/G64x as proprietary export; use Genetec Video Player/Security Desk for review or conversion.",
        },
        ParserPlugin {
            id: "avigilon_ave",
            lane: "cctv-nvr-export",
            vendor: "Avigilon ACC",
            confidence: "high",
            path_needles: &["avigilon", "/acc/"],
            extensions: &["ave"],
            recommended_action: "Treat AVE as proprietary export; preserve original and use player-assisted export when required.",
        },
    ]
}

fn named_vendor_plugins() -> &'static [ParserPlugin] {
    &[
        ParserPlugin {
            id: "hikvision",
            lane: "cctv-nvr-export",
            vendor: "Hikvision",
            confidence: "medium",
            path_needles: &["hikvision", "ivms", "vsplayer"],
            extensions: &["mp4", "avi", "dav"],
            recommended_action: "Path signal only: prefer exported media first and verify with file metadata or vendor player before making a vendor claim.",
        },
        ParserPlugin {
            id: "dahua_dav",
            lane: "cctv-nvr-export",
            vendor: "Dahua / OEM DAV",
            confidence: "medium",
            path_needles: &["dahua", "smartpss", "dss"],
            extensions: &["dav", "asf", "mp4", "avi"],
            recommended_action: "Path signal only: use DAV/ASF detection and official player-assisted conversion when FFmpeg cannot decode.",
        },
        ParserPlugin {
            id: "blackvue",
            lane: "dashcam-sd-card",
            vendor: "BlackVue",
            confidence: "medium",
            path_needles: &["blackvue"],
            extensions: &["mp4"],
            recommended_action: "Path signal only: pair front/rear channels and extract GPS/G-sensor metadata when present.",
        },
        ParserPlugin {
            id: "thinkware_inavi",
            lane: "dashcam-sd-card",
            vendor: "Thinkware / iNavi",
            confidence: "medium",
            path_needles: &["thinkware", "inavi", "아이나비"],
            extensions: &["mp4", "avi", "mov"],
            recommended_action: "Path signal only: classify recording mode folders and extract GPS/speed metadata when present.",
        },
        ParserPlugin {
            id: "finevu",
            lane: "dashcam-sd-card",
            vendor: "FineVu",
            confidence: "medium",
            path_needles: &["finevu", "fine-vu"],
            extensions: &["mp4", "avi"],
            recommended_action: "Path signal only: extract player-compatible GPS/G-sensor metadata when present.",
        },
        ParserPlugin {
            id: "iroad",
            lane: "dashcam-sd-card",
            vendor: "IROAD",
            confidence: "medium",
            path_needles: &["iroad"],
            extensions: &["mp4", "avi"],
            recommended_action: "Path signal only: classify event folders and GPS metadata when present.",
        },
        ParserPlugin {
            id: "viofo",
            lane: "dashcam-sd-card",
            vendor: "VIOFO",
            confidence: "medium",
            path_needles: &["viofo"],
            extensions: &["mp4"],
            recommended_action: "Path signal only: use MP4 as primary media and extract GPS tracks when present.",
        },
        ParserPlugin {
            id: "garmin_dashcam",
            lane: "dashcam-sd-card",
            vendor: "Garmin Dash Cam",
            confidence: "medium",
            path_needles: &["garmin"],
            extensions: &["mp4", "glv"],
            recommended_action: "Path signal only: preserve DCIM layout, MP4 files, and GLV companion files.",
        },
        ParserPlugin {
            id: "nextbase",
            lane: "dashcam-sd-card",
            vendor: "Nextbase",
            confidence: "medium",
            path_needles: &["nextbase"],
            extensions: &["mp4"],
            recommended_action: "Path signal only: extract GPS map/G-sensor metadata when present.",
        },
        ParserPlugin {
            id: "hanwha_wisenet_wave",
            lane: "cctv-nvr-export",
            vendor: "Hanwha/Wisenet WAVE",
            confidence: "medium",
            path_needles: &["hanwha", "wisenet", "/wave/"],
            extensions: &["nov", "mp4", "mkv", "avi"],
            recommended_action: "Path signal only: prefer WAVE official export path; ingest standard MP4/MKV/AVI outputs directly.",
        },
        ParserPlugin {
            id: "axis_onvif",
            lane: "cctv-nvr-export",
            vendor: "Axis",
            confidence: "medium",
            path_needles: &["axis", "vapix", "onvif"],
            extensions: &["mkv", "mp4", "asf"],
            recommended_action: "Path signal only: use MKV exports directly; device/API acquisition is a future connector path.",
        },
        ParserPlugin {
            id: "uniview",
            lane: "cctv-nvr-export",
            vendor: "Uniview",
            confidence: "medium",
            path_needles: &["uniview", "ezstation"],
            extensions: &["mp4", "avi"],
            recommended_action: "Path signal only: start with official export outputs; add proprietary samples later.",
        },
        ParserPlugin {
            id: "idis",
            lane: "cctv-nvr-export",
            vendor: "IDIS",
            confidence: "medium",
            path_needles: &["idis"],
            extensions: &["avi", "mp4"],
            recommended_action: "Path signal only: ingest AVI directly; preserve self-player exports without running them.",
        },
        ParserPlugin {
            id: "bosch_bvms",
            lane: "cctv-nvr-export",
            vendor: "Bosch BVMS/VRM",
            confidence: "medium",
            path_needles: &["bosch", "bvms", "vrm"],
            extensions: &["asf", "mov", "mp4"],
            recommended_action: "Path signal only: detect ZIP/native exports and ingest ASF/MOV/MP4 derived media directly.",
        },
        ParserPlugin {
            id: "milestone_xprotect",
            lane: "cctv-nvr-export",
            vendor: "Milestone XProtect",
            confidence: "medium",
            path_needles: &["milestone", "xprotect"],
            extensions: &["blk", "avi", "mkv"],
            recommended_action: "Path signal only: detect XProtect packages; ingest AVI/MKV exports directly.",
        },
        ParserPlugin {
            id: "genetec_g64",
            lane: "cctv-nvr-export",
            vendor: "Genetec Security Center",
            confidence: "medium",
            path_needles: &["genetec"],
            extensions: &["g64", "g64x", "mp4", "asf"],
            recommended_action: "Path signal only: ingest MP4/ASF directly; preserve G64/G64x for player-assisted review.",
        },
        ParserPlugin {
            id: "avigilon_ave",
            lane: "cctv-nvr-export",
            vendor: "Avigilon ACC",
            confidence: "medium",
            path_needles: &["avigilon", "/acc/"],
            extensions: &["ave", "avi"],
            recommended_action: "Path signal only: ingest AVI directly; preserve AVE exports for player-assisted review.",
        },
        ParserPlugin {
            id: "milesight",
            lane: "cctv-nvr-export",
            vendor: "Milesight",
            confidence: "medium",
            path_needles: &["milesight"],
            extensions: &["mp4", "avi", "mkv", "asf"],
            recommended_action: "Path signal only: ingest standard MP4/AVI/MKV/ASF exports directly; preserve EXE packages.",
        },
        ParserPlugin {
            id: "honeywell_research",
            lane: "dvr-hdd-recovery",
            vendor: "Honeywell",
            confidence: "medium",
            path_needles: &["honeywell"],
            extensions: &[],
            recommended_action: "Research target only until validated sample images exist.",
        },
    ]
}

fn plugin_json(plugin: &ParserPlugin) -> String {
    let path_needles = plugin
        .path_needles
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    let extensions = plugin
        .extensions
        .iter()
        .map(|item| format!("\"{}\"", json_escape(item)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"id\":\"{}\",\"lane\":\"{}\",\"vendor\":\"{}\",\"confidence\":\"{}\",\"path_needles\":[{}],\"extensions\":[{}],\"recommended_action\":\"{}\"}}",
        json_escape(plugin.id),
        json_escape(plugin.lane),
        json_escape(plugin.vendor),
        json_escape(plugin.confidence),
        path_needles,
        extensions,
        json_escape(plugin.recommended_action)
    )
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
        assert_eq!(profile.confidence, "high");
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
