use std::fs;

pub fn seed_repro_case(
    case_dir: &std::path::Path,
    timestamp: u64,
    validation_status: &str,
    artifact_hash: &str,
) {
    fs::create_dir_all(case_dir.join("db/filesystem")).unwrap();
    fs::create_dir_all(case_dir.join("artifacts/carved")).unwrap();
    fs::create_dir_all(case_dir.join("evidence/logs")).unwrap();
    fs::create_dir_all(case_dir.join("reports/package")).unwrap();
    let artifact = case_dir.join("artifacts/carved/carve_000001.mp4");
    fs::write(
        case_dir.join("db/videos.jsonl"),
        format!(
            "{{\"id\":\"video_000001\",\"source_path\":\"/evidence/source.mp4\",\"sha256\":\"sourcehash\",\"first_indexed_unix\":{},\"last_indexed_unix\":{}}}\n",
            timestamp,
            timestamp + 1
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("db/video_paths.tsv"),
        "id\tsource_path\nvideo_000001\t/evidence/source.mp4\n",
    )
    .unwrap();
    fs::write(
        case_dir.join("artifacts/carved/carve-log.jsonl"),
        format!(
            "{{\"id\":\"carve_000001\",\"output_path\":\"{}\",\"sha256\":\"{}\",\"validation_status\":\"candidate-unvalidated\",\"carved_unix\":{},\"previous_entry_sha256\":\"left\",\"entry_sha256\":\"right\"}}\n",
            artifact.display(),
            artifact_hash,
            timestamp
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("evidence/logs/validation-log.jsonl"),
        format!(
            "{{\"selector\":\"carve_000001\",\"target_path\":\"{}\",\"target_sha256\":\"{}\",\"validation_status\":\"{}\",\"validated_unix\":{}}}\n",
            artifact.display(),
            artifact_hash,
            validation_status,
            timestamp + 2
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("db/filesystem/tsk-files-123.jsonl"),
        format!(
            "{{\"path\":\"{}\",\"inode\":\"42\",\"deleted\":true,\"inspected_unix\":{}}}\n",
            artifact.display(),
            timestamp + 3
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("reports/package/package-manifest.json"),
        format!(
            "{{\"schema_version\":1,\"created_unix\":{},\"files\":[{{\"relative_path\":\"artifacts/carved/carve_000001.mp4\",\"sha256\":\"{}\"}}]}}\n",
            timestamp + 4,
            artifact_hash
        ),
    )
    .unwrap();
    fs::write(
        case_dir.join("reports/package/manifest.sha256"),
        format!("{artifact_hash}  artifacts/carved/carve_000001.mp4\n"),
    )
    .unwrap();
}
