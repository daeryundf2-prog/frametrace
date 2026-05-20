use crate::model::ProbeSummary;
use std::path::Path;
use std::process::Command;

pub fn probe(path: &Path) -> ProbeSummary {
    probe_with_binary("ffprobe", path)
}

pub fn probe_with_binary(binary: &str, path: &Path) -> ProbeSummary {
    let output = Command::new(binary)
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return ProbeSummary {
                ok: false,
                raw_json: None,
                error: Some(format!("failed to run ffprobe: {error}")),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
            };
        }
    };

    if !output.status.success() {
        return ProbeSummary {
            ok: false,
            raw_json: None,
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
        };
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    let video_section = stream_section(&raw, "video");
    let audio_section = stream_section(&raw, "audio");
    ProbeSummary {
        ok: true,
        duration_seconds: find_json_string(&raw, "duration").and_then(|value| value.parse().ok()),
        format_name: find_json_string(&raw, "format_name"),
        video_codec: video_section
            .as_deref()
            .and_then(|section| find_json_string(section, "codec_name")),
        audio_codec: audio_section
            .as_deref()
            .and_then(|section| find_json_string(section, "codec_name")),
        width: video_section
            .as_deref()
            .and_then(|section| find_json_u32(section, "width")),
        height: video_section
            .as_deref()
            .and_then(|section| find_json_u32(section, "height")),
        raw_json: Some(raw),
        error: None,
    }
}

fn stream_section(raw: &str, codec_type: &str) -> Option<String> {
    let mut offset = 0;
    while let Some(relative_hit) = raw[offset..].find("\"codec_type\"") {
        let hit = offset + relative_hit;
        if find_json_string(&raw[hit..], "codec_type").as_deref() == Some(codec_type) {
            let before = raw[..hit].rfind('{')?;
            let after = raw[hit..].find('}')? + hit + 1;
            return Some(raw[before..after].to_string());
        }
        offset = hit + "\"codec_type\"".len();
    }
    None
}

fn find_json_string(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let key_start = raw.find(&needle)?;
    let after_key = &raw[key_start + needle.len()..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..].trim_start();
    if let Some(stripped) = value.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let end = value
            .find(|ch: char| ch == ',' || ch == '}' || ch.is_whitespace())
            .unwrap_or(value.len());
        Some(value[..end].to_string())
    }
}

fn find_json_u32(raw: &str, key: &str) -> Option<u32> {
    find_json_string(raw, key).and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{find_json_string, stream_section};

    #[test]
    fn extracts_json_values_from_ffprobe_like_payload() {
        let raw = r#"{"streams":[{"codec_name":"h264","codec_type":"video","width":1920,"height":1080}],"format":{"duration":"12.345","format_name":"mov,mp4"}}"#;
        assert_eq!(find_json_string(raw, "duration").as_deref(), Some("12.345"));
        let video = stream_section(raw, "video").unwrap();
        assert_eq!(
            find_json_string(&video, "codec_name").as_deref(),
            Some("h264")
        );
    }
}
