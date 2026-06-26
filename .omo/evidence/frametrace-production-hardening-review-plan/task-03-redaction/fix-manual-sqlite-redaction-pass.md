# Manual QA SQLite package redaction after compile fix
start_utc=2026-06-24T08:29:13Z
temp_root=/tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj
case_dir=/tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite
source_file=/tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Client ACME Source/Camera 77/sqlite clip.mp4

## COMMAND: cargo build --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
exit_code=0

## COMMAND: target/debug/frametrace init-case /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite --title ACME\ SQLite --operator Shin
case created: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite
case id: FT-1782289753
exit_code=0

## COMMAND: target/debug/frametrace scan-folder /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Client\ ACME\ Source/Camera\ 77 --no-ffprobe
scan complete
source registered: src_3bbf95f16830781d (folder)
job: job_1782289753_000001 (scan-folder)
videos indexed: 1
bytes indexed: 5
index: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/db/video_index.json
sqlite: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/db/case.db
exit_code=0

## COMMAND: sqlite3 /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite/db/case.db UPDATE\ videos\ SET\ record_json\ =\ \'\{\"id\":\"vid_000001\"\,\"source_path\":\"/tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Client\ ACME\ Source/Camera\ 77/sqlite\ clip.mp4\"\,\"file_url\":\"file:///tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Client\ ACME\ Source/Camera\ 77/sqlite\ clip.mp4\"\,\"output_path\":\"/tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite/artifacts/frames/frame.jpg\"\}\'\ WHERE\ id\ =\ \'vid_000001\'\;
exit_code=0

## COMMAND: target/debug/frametrace make-report /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite
report written: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/reports/case-report.html
exit_code=0

## COMMAND: target/debug/frametrace make-review /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite
review written: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/review/index.html
evidence viewer written: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/review/evidence-viewer.html
exit_code=0

## COMMAND: target/debug/frametrace package-case /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/Examiner\ Shin/Case\ SQLite --output /tmp/FrameTrace\ Client\ ACME\ SQLite\ fix\ QA.TOiqBj/sqlite\ package
case package written
output: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/sqlite package
files: 9
manifest: /tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/sqlite package/package-manifest.json
exit_code=0

## ASSERT: packaged SQLite copy exists
sqlite_exists_exit=0

## ASSERT: active case SQLite still retains full internal provenance
{"id":"vid_000001","source_path":"/tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Client ACME Source/Camera 77/sqlite clip.mp4","file_url":"file:///tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Client ACME Source/Camera 77/sqlite clip.mp4","output_path":"/tmp/FrameTrace Client ACME SQLite fix QA.TOiqBj/Examiner Shin/Case SQLite/artifacts/frames/frame.jpg"}
active_record_source_path_exit=0

## ASSERT: packaged SQLite record_json removes temp/source path
{"file_url":"","id":"vid_000001","local_operator_full_path_disclosure":false,"output_path":"artifacts/frames/frame.jpg","path_disclosure_mode":"redacted","path_disclosure_notice":"Distributable output redacts local workstation/source paths by default.","source_path":"[redacted-source:vid_000001]"}
package_record_tmp_root_grep_exit=1
package_record_source_path_grep_exit=1
package_record_redacted_source_exit=0
package_record_case_relative_artifact_exit=0

## ASSERT: package distributable text outputs do not contain temp/source path
package_text_tmp_root_grep_exit=1
package_text_source_path_grep_exit=1

## CLEANUP
temp_root_removed_exit=0
end_utc=2026-06-24T08:29:13Z
final_exit=0
