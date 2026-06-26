# Baseline path leak proof
tmp_root=/tmp/FrameTrace Client ACME 유출 Case pV66
case_dir=/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha
source_file=/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4
## Commands
target/debug/frametrace make-report '/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha'
report written: /tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/reports/case-report.html
exit_make_report=0
target/debug/frametrace make-review '/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha'
error: failed to inspect SQLite schema: file is not a database
exit_make_review=1
target/debug/frametrace package-case '/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha' --output '/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default'
case package written
output: /tmp/FrameTrace Client ACME 유출 Case pV66/exported package default
files: 8
manifest: /tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/package-manifest.json
exit_package=0
## Grep default distributable leak
grep -R -n -F '/tmp/FrameTrace Client ACME 유출 Case pV66' '/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/reports' '/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/review' '/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default'
/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/reports/case-report.html:138:const scan = {"schema_version":1,"source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01","video_count":1,"total_bytes":5,"options":{"hash_files":false,"use_ffprobe":false},"videos":[{"id":"vid_000001","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","file_url":"file:///tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","relative_path":"Camera 01/parking lot clip.mp4","extension":"mp4","size_bytes":5,"hash_status":"not-hashed","confidence":"candidate","source_profile":{"vendor":"ACME","parser":"synthetic","lane":"fixture","confidence":"candidate","recommended_action":"review"}}]}
/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/reports/case-report.html:143:const frameLog = [{"event":"make-frame-capture","kind":"frame-capture","artifact_state":"derived","operator":"Shin","method":"ffmpeg-frame-capture","source_artifact_id":"vid_000001","derived_artifact_id":"derived-frame","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","output_artifact_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_artifact_sha256":"abc"}];
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/artifacts/frames/frame-log.jsonl:1:{"event":"make-frame-capture","kind":"frame-capture","artifact_state":"derived","operator":"Shin","method":"ffmpeg-frame-capture","source_artifact_id":"vid_000001","derived_artifact_id":"derived-frame","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","output_artifact_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_artifact_sha256":"abc"}
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/videos.jsonl:1:{"id":"vid_000001","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","file_url":"file:///tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","relative_path":"Camera 01/parking lot clip.mp4","extension":"mp4","size_bytes":5,"hash_status":"not-hashed","confidence":"candidate","source_profile":{"vendor":"ACME","parser":"synthetic","lane":"fixture","confidence":"candidate","recommended_action":"review"}}
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/video_index.json:1:{"schema_version":1,"source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01","video_count":1,"total_bytes":5,"options":{"hash_files":false,"use_ffprobe":false},"videos":[{"id":"vid_000001","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","file_url":"file:///tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","relative_path":"Camera 01/parking lot clip.mp4","extension":"mp4","size_bytes":5,"hash_status":"not-hashed","confidence":"candidate","source_profile":{"vendor":"ACME","parser":"synthetic","lane":"fixture","confidence":"candidate","recommended_action":"review"}}]}
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/video_paths.tsv:2:vid_000001	/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/reports/case-report.html:138:const scan = {"schema_version":1,"source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01","video_count":1,"total_bytes":5,"options":{"hash_files":false,"use_ffprobe":false},"videos":[{"id":"vid_000001","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","file_url":"file:///tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","relative_path":"Camera 01/parking lot clip.mp4","extension":"mp4","size_bytes":5,"hash_status":"not-hashed","confidence":"candidate","source_profile":{"vendor":"ACME","parser":"synthetic","lane":"fixture","confidence":"candidate","recommended_action":"review"}}]}
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/reports/case-report.html:143:const frameLog = [{"event":"make-frame-capture","kind":"frame-capture","artifact_state":"derived","operator":"Shin","method":"ffmpeg-frame-capture","source_artifact_id":"vid_000001","derived_artifact_id":"derived-frame","source_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Client ACME Source/Camera 01/parking lot clip.mp4","output_artifact_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_path":"/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/artifacts/frames/frame_0001.jpg","output_artifact_sha256":"abc"}];
grep_exit=0
## Artifact list
/tmp/FrameTrace Client ACME 유출 Case pV66/Examiner Shin/Case Alpha/reports/case-report.html
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/README.txt
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/artifacts/frames/frame-log.jsonl
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/artifacts/frames/frame_0001.jpg
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/case.json
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/case.db
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/video_index.json
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/video_paths.tsv
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/db/videos.jsonl
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/manifest.sha256
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/package-manifest.json
/tmp/FrameTrace Client ACME 유출 Case pV66/exported package default/reports/case-report.html
cleanup_target=/tmp/FrameTrace Client ACME 유출 Case pV66
