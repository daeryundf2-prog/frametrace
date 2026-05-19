use super::*;
use crate::model::{ScanResult, VideoRecord};
use crate::util::{json_escape, now_unix, path_to_file_url};
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(crate) fn string_array_json(values: &[String]) -> String {
    let body = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

pub(crate) fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

pub(crate) fn u32_to_i64(value: u32) -> i64 {
    i64::from(value)
}

pub(crate) fn u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(crate) fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}
