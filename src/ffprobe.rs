use crate::model::ProbeSummary;
use crate::tool_policy::resolve_tool_binary;
use crate::util::compact_json_value_if_well_formed;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

pub fn probe(path: &Path) -> ProbeSummary {
    probe_with_binary("ffprobe", path)
}

pub fn probe_with_binary(binary: &str, path: &Path) -> ProbeSummary {
    let binary = match resolve_tool_binary(binary, &["ffprobe"]) {
        Ok(binary) => binary,
        Err(error) => {
            return ProbeSummary {
                ok: false,
                raw_json: None,
                error: Some(error),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
            };
        }
    };

    let mut command = Command::new(&binary);
    command
        .arg("-v")
        .arg("error")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path);
    let output = crate::util::run_with_timeout(&mut command, Some(crate::util::PROBE_TIMEOUT_SECS));

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
            error: Some(sanitize_probe_error(&String::from_utf8_lossy(
                &output.stderr,
            ))),
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
        };
    }

    let raw = String::from_utf8_lossy(&output.stdout).to_string();
    if compact_json_value_if_well_formed(&raw).is_none() {
        return ProbeSummary {
            ok: false,
            raw_json: None,
            error: Some("invalid ffprobe JSON output".to_string()),
            duration_seconds: None,
            format_name: None,
            video_codec: None,
            audio_codec: None,
            width: None,
            height: None,
        };
    }
    parse_probe_summary(raw)
}

/// Strips ffprobe's per-run log prefixes ("[module @ 0xADDR] ") which embed a
/// memory address; without this, failed-probe error text makes the case index
/// non-reproducible byte-for-byte.
fn sanitize_probe_error(stderr: &str) -> String {
    stderr
        .lines()
        .map(|line| {
            let Some(close) = line.find("] ") else {
                return line.to_string();
            };
            let head = &line[..close];
            let prefix_ok = head.starts_with('[')
                && head.contains(" @ ")
                && head
                    .rsplit(" @ ")
                    .next()
                    .map(|token| {
                        !token.is_empty()
                            && token
                                .trim_start_matches("0x")
                                .chars()
                                .all(|c| c.is_ascii_hexdigit())
                    })
                    .unwrap_or(false);
            if prefix_ok {
                line[close + 2..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        )
}

/// Maps a well-formed ffprobe JSON payload to a summary. Split out from
/// `probe_with_binary` so the parsing rules are unit-testable without the
/// binary.
#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_name: Option<String>,
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    #[serde(rename = "format_name")]
    name: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

fn first_duration(value: &Option<String>) -> Option<f64> {
    value.as_deref().and_then(|text| text.parse().ok())
}

/// Maps a well-formed ffprobe JSON payload to a summary. Split out from
/// `probe_with_binary` so the parsing rules are unit-testable without the
/// binary. Format-level metadata wins over stream-level values, which can be
/// per-stream and misleading for container duration.
fn parse_probe_summary(raw: String) -> ProbeSummary {
    let parsed: FfprobeOutput = match serde_json::from_str(&raw) {
        Ok(parsed) => parsed,
        Err(error) => {
            return ProbeSummary {
                ok: false,
                raw_json: None,
                error: Some(format!("invalid ffprobe JSON output: {error}")),
                duration_seconds: None,
                format_name: None,
                video_codec: None,
                audio_codec: None,
                width: None,
                height: None,
            };
        }
    };
    let video = parsed.streams.as_ref().and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("video"))
    });
    let audio = parsed.streams.as_ref().and_then(|streams| {
        streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("audio"))
    });
    ProbeSummary {
        ok: true,
        duration_seconds: parsed
            .format
            .as_ref()
            .and_then(|format| first_duration(&format.duration))
            .or_else(|| {
                video
                    .and_then(|stream| first_duration(&stream.duration))
                    .or_else(|| {
                        audio
                            .as_ref()
                            .and_then(|stream| first_duration(&stream.duration))
                    })
            }),
        format_name: parsed
            .format
            .as_ref()
            .and_then(|format| format.name.clone()),
        video_codec: video.and_then(|stream| stream.codec_name.clone()),
        audio_codec: audio.and_then(|stream| stream.codec_name.clone()),
        width: video.and_then(|stream| stream.width),
        height: video.and_then(|stream| stream.height),
        raw_json: Some(raw),
        error: None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sanitizes_run_specific_probe_prefixes() {
        let raw = concat!(
            "[mov,mp4,m4a,3gp,3g2,mj2 @ 0000019f1d4a66c0] moov atom not found",
            "\r\n",
            "C:",
            r"\src\a.mp4"
        );
        let cleaned = super::sanitize_probe_error(raw);
        // line-based sanitization normalizes CRLF to LF
        assert_eq!(
            cleaned,
            concat!("moov atom not found", "\n", "C:", r"\src\a.mp4")
        );
        let untouched = super::sanitize_probe_error("plain failure text");
        assert_eq!(untouched, "plain failure text");
    }

    #[test]
    fn prefers_format_duration_over_stream_duration() {
        let raw = r#"{"streams":[{"duration":"5.000000","codec_type":"video"}],"format":{"duration":"8.000000"}}"#;
        let summary = super::parse_probe_summary(raw.to_string());
        assert_eq!(summary.duration_seconds, Some(8.0));
    }

    #[test]
    fn maps_streams_and_format_through_serde() {
        let raw = r#"{"streams":[{"codec_name":"h264","codec_type":"video","width":1920,"height":1080},{"codec_name":"aac","codec_type":"audio"}],"format":{"duration":"12.345","format_name":"mov,mp4"}}"#;
        let summary = super::parse_probe_summary(raw.to_string());
        assert!(summary.ok);
        assert_eq!(summary.duration_seconds, Some(12.345));
        assert_eq!(summary.format_name.as_deref(), Some("mov,mp4"));
        assert_eq!(summary.video_codec.as_deref(), Some("h264"));
        assert_eq!(summary.audio_codec.as_deref(), Some("aac"));
        assert_eq!(summary.width, Some(1920));
        assert_eq!(summary.height, Some(1080));
    }

    #[test]
    fn stream_duration_is_fallback_when_format_missing() {
        let raw = r#"{"streams":[{"codec_type":"video","duration":"5.500"}]}"#;
        let summary = super::parse_probe_summary(raw.to_string());
        assert_eq!(summary.duration_seconds, Some(5.5));
    }

    #[test]
    fn invalid_json_is_reported_as_failure() {
        let summary = super::parse_probe_summary("not json".to_string());
        assert!(!summary.ok);
        assert!(summary.error.unwrap().contains("invalid ffprobe JSON"));
    }
}
