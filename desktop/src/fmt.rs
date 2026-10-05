//! 时间/时长格式化（本地时区 UTC+8）

use chrono::{DateTime, FixedOffset, Utc};
use std::str::FromStr;

/// 解析 UTC ISO（"2026-09-30T12:00:00Z"）→ DateTime<Utc>
pub fn parse_utc(iso: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => Some(dt.with_timezone(&Utc)),
        Err(_) => DateTime::<Utc>::from_str(iso).ok(),
    }
}

/// 格式化为本地时间字符串（UTC+8）："2026-09-30 20:00:00"
pub fn fmt_datetime(iso: &str) -> String {
    match parse_utc(iso) {
        Some(dt) => {
            let local = dt.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
            local.format("%Y-%m-%d %H:%M:%S").to_string()
        }
        None => iso.to_string(),
    }
}

/// 仅日期： "2026-09-30"
pub fn fmt_date(iso: &str) -> String {
    match parse_utc(iso) {
        Some(dt) => {
            let local = dt.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
            local.format("%Y-%m-%d").to_string()
        }
        None => {
            // 兜底：直接取前 10 位
            if iso.len() >= 10 {
                iso[..10].to_string()
            } else {
                iso.to_string()
            }
        }
    }
}

/// 毫秒 → 中文时长："1时2分3秒" / "5分12秒" / "47秒"
pub fn fmt_duration(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}时{m}分{s}秒")
    } else if m > 0 {
        format!("{m}分{s}秒")
    } else {
        format!("{s}秒")
    }
}

/// 毫秒 → 短时长（用于图表轴/紧凑显示）："1:02:03"
pub fn fmt_duration_short(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h}:{m:02}:{s:02}")
}

/// 当前 UTC 时间，ISO 格式（带 Z）
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// 当前本地日期 "YYYY-MM-DD"
pub fn today_str() -> String {
    let local = Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
    local.format("%Y-%m-%d").to_string()
}
