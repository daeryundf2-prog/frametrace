use super::types::{FlsEntry, MmlsPartition};

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "avi", "mkv", "wmv", "asf", "mpg", "mpeg", "mts", "m2ts", "ts", "3gp",
    "webm", "flv", "dav", "dav_", "nov", "ave", "g64", "g64x", "glv", "blk", "264", "265", "h264",
    "h265", "hevc",
];

pub(super) fn parse_mmls_partitions(text: &str) -> Vec<MmlsPartition> {
    text.lines()
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 5 || !fields[0].ends_with(':') {
                return None;
            }
            let start = fields.get(2)?.parse::<u64>().ok()?;
            let end = fields.get(3)?.parse::<u64>().ok()?;
            let length = fields.get(4)?.parse::<u64>().ok()?;
            let slot_type = fields[1];
            let description = if fields.len() > 5 {
                fields[5..].join(" ")
            } else {
                String::new()
            };
            let description_lc = description.to_ascii_lowercase();
            let allocated = slot_type != "Meta"
                && slot_type != "-------"
                && !description_lc.contains("unallocated")
                && !description_lc.contains("primary table");
            Some(MmlsPartition {
                slot: fields[0].trim_end_matches(':').to_string(),
                start,
                end,
                length,
                description,
                allocated,
            })
        })
        .collect()
}

pub(super) fn choose_partition_offset(partitions: &[MmlsPartition], explicit: Option<u64>) -> u64 {
    explicit.unwrap_or_else(|| {
        partitions
            .iter()
            .find(|partition| partition.allocated)
            .map(|partition| partition.start)
            .unwrap_or(0)
    })
}

pub(super) fn parse_fls_entries(text: &str) -> Vec<FlsEntry> {
    text.lines().filter_map(parse_fls_entry).collect()
}

fn parse_fls_entry(line: &str) -> Option<FlsEntry> {
    let raw = line.trim();
    if raw.is_empty() {
        return None;
    }
    let (left, right) = raw.split_once(':')?;
    let tokens = left.split_whitespace().collect::<Vec<_>>();
    let file_type = tokens
        .iter()
        .find(|token| token.contains('/'))
        .map(|token| token.trim_matches('+').to_string());
    let inode = tokens
        .iter()
        .rev()
        .find(|token| token.chars().any(|ch| ch.is_ascii_digit()))
        .map(|token| token.trim_matches('+').to_string());
    let path = right.trim().to_string();
    let deleted = tokens.contains(&"*") || path.contains("(deleted)");
    let video_candidate = path_has_video_extension(&path);

    Some(FlsEntry {
        raw_line: raw.to_string(),
        file_type,
        inode,
        path: Some(path),
        deleted,
        video_candidate,
    })
}

fn path_has_video_extension(path: &str) -> bool {
    let Some((_, extension)) = path.rsplit_once('.') else {
        return false;
    };
    let extension = extension
        .trim()
        .trim_end_matches("(deleted)")
        .trim()
        .to_ascii_lowercase();
    VIDEO_EXTENSIONS
        .iter()
        .any(|known| extension.eq_ignore_ascii_case(known))
}

#[cfg(test)]
mod tests {
    use super::{choose_partition_offset, parse_fls_entry, parse_mmls_partitions};

    #[test]
    fn parses_mmls_allocated_partition_offsets() {
        let text = "\
DOS Partition Table
Offset Sector: 0
Units are in 512-byte sectors

      Slot      Start        End          Length       Description
000:  Meta      0000000000   0000000000   0000000001   Primary Table (#0)
001:  -------   0000000000   0000002047   0000002048   Unallocated
002:  000:000   0000002048   0004095999   0004093952   NTFS / exFAT (0x07)
";
        let partitions = parse_mmls_partitions(text);
        assert_eq!(partitions.len(), 3);
        assert_eq!(choose_partition_offset(&partitions, None), 2048);
        assert_eq!(choose_partition_offset(&partitions, Some(63)), 63);
    }

    #[test]
    fn parses_deleted_fls_video_entries() {
        let entry =
            parse_fls_entry("r/r * 1304-128-1: /BLACKBOX/event001.mp4 (deleted)").expect("entry");
        assert_eq!(entry.file_type.as_deref(), Some("r/r"));
        assert_eq!(entry.inode.as_deref(), Some("1304-128-1"));
        assert_eq!(
            entry.path.as_deref(),
            Some("/BLACKBOX/event001.mp4 (deleted)")
        );
        assert!(entry.deleted);
        assert!(entry.video_candidate);
    }
}
