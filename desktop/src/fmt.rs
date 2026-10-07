//! 时间/时长格式化（本地时区 UTC+8）
//!
//! 小时制（24h / 12h）由用户设置 `timeFormat` 控制：所有对外展示时间的函数都
//! 接收 `hour12` 参数，由 `BootTrackerApp::hour12()` 统一传入，避免出现
//! 「设置里选了 12h、界面仍显示 24h」的死控件。日期类函数（`fmt_date`）不含
//! 小时，不受小时制影响——过滤逻辑依赖它的字符串比较，必须保持稳定。

use chrono::{DateTime, FixedOffset, Utc};
use std::str::FromStr;

/// 本机展示时区（UTC+8）。web 端同样按东八区展示。
const LOCAL_OFFSET_SECS: i32 = 8 * 3600;

/// 解析 UTC ISO → 本地时区时间
fn parse_local(iso: &str) -> Option<DateTime<FixedOffset>> {
    let dt = match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => dt.with_timezone(&Utc),
        Err(_) => DateTime::<Utc>::from_str(iso).ok()?,
    };
    Some(dt.with_timezone(&FixedOffset::east_opt(LOCAL_OFFSET_SECS)?))
}

/// 解析 UTC ISO（"2026-09-30T12:00:00Z"）→ DateTime<Utc>
pub fn parse_utc(iso: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => Some(dt.with_timezone(&Utc)),
        Err(_) => DateTime::<Utc>::from_str(iso).ok(),
    }
}

/// 本地时间格式串：24 小时制或 12 小时制（中文 上午/下午）
fn fmt_pattern(hour12: bool) -> &'static str {
    if hour12 {
        "%Y-%m-%d %I:%M:%S %p"
    } else {
        "%Y-%m-%d %H:%M:%S"
    }
}

/// 把 chrono 的英文 AM/PM 换成本地化的上午/下午
fn localize_ampm(s: String) -> String {
    s.replace("AM", crate::t!("上午")).replace("PM", crate::t!("下午"))
}

/// 格式化为本地时间字符串："2026-09-30 20:00:00" / "2026-09-30 08:00:00 下午"
pub fn fmt_datetime(iso: &str, hour12: bool) -> String {
    match parse_local(iso) {
        Some(local) => {
            let s = local.format(fmt_pattern(hour12)).to_string();
            if hour12 {
                localize_ampm(s)
            } else {
                s
            }
        }
        None => iso.to_string(),
    }
}

/// 仅时间部分："20:00:00" / "08:00:00 下午"（时间轴与「本次会话」用）
pub fn fmt_time(iso: &str, hour12: bool) -> String {
    match parse_local(iso) {
        Some(local) => {
            let pattern = if hour12 { "%I:%M:%S %p" } else { "%H:%M:%S" };
            let s = local.format(pattern).to_string();
            if hour12 {
                localize_ampm(s)
            } else {
                s
            }
        }
        None => iso.to_string(),
    }
}

/// 仅日期："2026-09-30"
pub fn fmt_date(iso: &str) -> String {
    match parse_local(iso) {
        Some(local) => local.format("%Y-%m-%d").to_string(),
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

/// 毫秒 → 短时长（用于图表 Y 轴/紧凑显示）："1:02:03"、"45秒"、"12分"
///
/// 图表轴刻度不追求秒级精确，省掉尾部为零的段，读数更清爽。
pub fn fmt_duration_short(ms: i64) -> String {
    let secs = ms.max(0) / 1000;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else if m > 0 {
        format!("{m}分{s:02}秒")
    } else {
        format!("{s}秒")
    }
}

/// 当前 UTC 时间，ISO 格式（带 Z）
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// 当前本地日期 "YYYY-MM-DD"
pub fn today_str() -> String {
    match FixedOffset::east_opt(LOCAL_OFFSET_SECS) {
        Some(off) => Utc::now().with_timezone(&off).format("%Y-%m-%d").to_string(),
        None => Utc::now().format("%Y-%m-%d").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2026-09-30T12:00:00Z → UTC+8 = 2026-09-30 20:00:00
    const NOON_UTC: &str = "2026-09-30T12:00:00Z";
    // 2026-10-06T13:43:56.217505Z → UTC+8 = 2026-10-06 21:43:56
    const ZONE_SAMPLE: &str = "2026-10-06T13:43:56.217505Z";

    #[test]
    fn datetime_24h() {
        assert_eq!(fmt_datetime(NOON_UTC, false), "2026-09-30 20:00:00");
    }

    #[test]
    fn datetime_12h_is_localized_and_padded() {
        // %I 零填充，AM/PM 中文化
        assert_eq!(fmt_datetime(NOON_UTC, true), "2026-09-30 08:00:00 下午");
    }

    #[test]
    fn datetime_12h_afternoon_value() {
        assert_eq!(fmt_datetime(ZONE_SAMPLE, true), "2026-10-06 09:43:56 下午");
    }

    #[test]
    fn datetime_midnight_and_noon_boundaries() {
        // 16:00Z = 次日 00:00（本地），12 小时制应为 12 点而不是 0 点
        assert_eq!(fmt_datetime("2026-09-30T16:00:00Z", true), "2026-10-01 12:00:00 上午");
        // 04:00Z = 本地 12:00 正午
        assert_eq!(fmt_datetime("2026-09-30T04:00:00Z", true), "2026-09-30 12:00:00 下午");
        assert_eq!(fmt_datetime("2026-09-30T16:00:00Z", false), "2026-10-01 00:00:00");
    }

    #[test]
    fn time_only_respects_hour12() {
        assert_eq!(fmt_time(ZONE_SAMPLE, false), "21:43:56");
        assert_eq!(fmt_time(ZONE_SAMPLE, true), "09:43:56 下午");
    }

    #[test]
    fn date_crosses_utc_day_boundary() {
        // 本地日期要比 UTC 日期大一天
        assert_eq!(fmt_date("2026-09-30T16:00:00Z"), "2026-10-01");
        assert_eq!(fmt_date(ZONE_SAMPLE), "2026-10-06");
    }

    #[test]
    fn date_falls_back_to_prefix_for_bad_input() {
        assert_eq!(fmt_date("2026-10-01"), "2026-10-01");
        assert_eq!(fmt_date("bad"), "bad");
        // 解析失败时时间函数原样返回，不 panic
        assert_eq!(fmt_datetime("bad", true), "bad");
    }

    #[test]
    fn duration_short_is_axis_friendly() {
        assert_eq!(fmt_duration_short(3_600_000 + 2 * 60_000 + 3_000), "1:02:03");
        assert_eq!(fmt_duration_short(12 * 60_000), "12分00秒");
        assert_eq!(fmt_duration_short(45_000), "45秒");
        assert_eq!(fmt_duration_short(-5), "0秒");
    }

    #[test]
    fn duration_full_text() {
        assert_eq!(fmt_duration(3_600_000 + 2 * 60_000 + 3_000), "1时2分3秒");
        assert_eq!(fmt_duration(12 * 60_000 + 5_000), "12分5秒");
        assert_eq!(fmt_duration(47_000), "47秒");
    }
}
