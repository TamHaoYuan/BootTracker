//! 日期时间工具（与 React 网页端行为对齐）
//!
//! - ISO 解析与本地时间转换走 `js_sys::Date`（与 `new Date(iso)` 语义一致）
//! - 纯日期算术（位移/星期）用 Howard Hinnant 的 civil 日期算法，零时区歧义

use wasm_bindgen::JsCast;

/// ISO 字符串 → 本地 epoch 毫秒（无效返回 None）
pub fn parse_iso_ms(iso: &str) -> Option<f64> {
    let ms = js_sys::Date::parse(iso);
    if ms.is_nan() {
        None
    } else {
        Some(ms)
    }
}

/// 当前 epoch 毫秒
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn date_of(ms: f64) -> js_sys::Date {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms))
}

/// epoch 毫秒 → 本地 "YYYY-MM-DD"
pub fn local_date_str(ms: f64) -> String {
    let d = date_of(ms);
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// ISO → 本地日期 "YYYY-MM-DD"（空/无效返回空串）
pub fn iso_to_local_date(iso: &str) -> String {
    parse_iso_ms(iso).map(local_date_str).unwrap_or_default()
}

/// 今天（本地）
pub fn get_local_today() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date()
    )
}

/// ISO → 完整本地时间 "YYYY/MM/DD HH:mm:ss"（zh-CN 2-digit 风格）
pub fn fmt_full_time(iso: Option<&str>) -> String {
    match iso.and_then(parse_iso_ms) {
        None => "—".to_string(),
        Some(ms) => {
            let d = date_of(ms);
            format!(
                "{:04}/{:02}/{:02} {:02}:{:02}:{:02}",
                d.get_full_year(),
                d.get_month() + 1,
                d.get_date(),
                d.get_hours(),
                d.get_minutes(),
                d.get_seconds()
            )
        }
    }
}

/// ISO → "HH:mm"
pub fn fmt_clock(iso: &str) -> String {
    match parse_iso_ms(iso) {
        None => "--:--".to_string(),
        Some(ms) => {
            let d = date_of(ms);
            format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
        }
    }
}

/// 当前本地 "HH:mm:ss"
pub fn fmt_now_clock() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}

/// 毫秒时长 → "x时x分x秒"
pub fn fmt_duration(ms: f64) -> String {
    if !ms.is_finite() || ms <= 0.0 {
        return "—".to_string();
    }
    let s = (ms / 1000.0).floor() as i64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}时{m}分{sec}秒")
    } else if m > 0 {
        format!("{m}分{sec}秒")
    } else {
        format!("{sec}秒")
    }
}

/// 实时运行时长：基于当前时间
pub fn live_duration(boot_ms: f64, now: f64) -> String {
    fmt_duration((now - boot_ms).max(0.0))
}

/// 会话时长（毫秒）：已结束用记录值，进行中算到当前时刻
pub fn session_duration(s: &crate::api::BootSession, now: f64) -> f64 {
    if let Some(_shut) = &s.shutdown_time {
        if let Some(d) = s.duration {
            return d;
        }
        match (parse_iso_ms(&s.boot_time), parse_iso_ms(_shut)) {
            (Some(b), Some(sh)) => sh - b,
            _ => 0.0,
        }
    } else {
        parse_iso_ms(&s.boot_time)
            .map(|b| (now - b).max(0.0))
            .unwrap_or(0.0)
    }
}

/* ================= civil 日期算法（无时区歧义） ================= */

/// (y, m, d) → 自 1970-01-01 起的天数
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// 自 1970-01-01 起的天数 → (y, m, d)
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 解析 "YYYY-MM-DD" → 天数
pub fn parse_date_str(s: &str) -> Option<i64> {
    let mut it = s.splitn(3, '-');
    let y = it.next()?.parse::<i64>().ok()?;
    let m = it.next()?.parse::<i64>().ok()?;
    let d = it.next()?.parse::<i64>().ok()?;
    Some(days_from_civil(y, m, d))
}

/// 天数 → "YYYY-MM-DD"
pub fn date_str_from_days(z: i64) -> String {
    let (y, m, d) = civil_from_days(z);
    format!("{y:04}-{m:02}-{d:02}")
}

/// "YYYY-MM-DD" ± n 天
pub fn shift_date(date: &str, days: i64) -> String {
    match parse_date_str(date) {
        Some(z) => date_str_from_days(z + days),
        None => date.to_string(),
    }
}

/// 星期（周一=0 … 周日=6）
pub fn weekday_mon0(z: i64) -> i64 {
    (z + 3).rem_euclid(7)
}

/// 所在周的周一 "YYYY-MM-DD"
pub fn week_start(date: &str) -> String {
    match parse_date_str(date) {
        Some(z) => date_str_from_days(z - weekday_mon0(z)),
        None => date.to_string(),
    }
}

/* ================= datetime-local 输入值转换 ================= */

/// ISO → "YYYY-MM-DDTHH:mm"（供 <input type="datetime-local">）
pub fn iso_to_datetime_local(iso: Option<&str>) -> String {
    match iso.and_then(parse_iso_ms) {
        None => String::new(),
        Some(ms) => {
            let d = date_of(ms);
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}",
                d.get_full_year(),
                d.get_month() + 1,
                d.get_date(),
                d.get_hours(),
                d.get_minutes()
            )
        }
    }
}

/// "YYYY-MM-DDTHH:mm"（本地） → UTC ISO（带 Z），与旧前端 toISOString 行为一致
pub fn datetime_local_to_iso(v: &str) -> Option<String> {
    if v.is_empty() {
        return None;
    }
    let ms = js_sys::Date::parse(&format!("{v}:00"));
    if ms.is_nan() {
        return None;
    }
    let d = date_of(ms);
    // toISOString
    let iso = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        d.get_utc_full_year(),
        d.get_utc_month() + 1,
        d.get_utc_date(),
        d.get_utc_hours(),
        d.get_utc_minutes(),
        d.get_utc_seconds()
    );
    Some(iso)
}

/// 当前本地时间 → UTC ISO（记录关机用）
pub fn now_iso() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        d.get_utc_full_year(),
        d.get_utc_month() + 1,
        d.get_utc_date(),
        d.get_utc_hours(),
        d.get_utc_minutes(),
        d.get_utc_seconds()
    )
}

/// 下载文本/二进制为文件（Blob + URL + <a> click）
pub fn download_blob(bytes: Vec<u8>, mime: &str, filename: &str) {
    let window = web_sys::window().expect("window");
    let arr = js_sys::Uint8Array::new_with_length(bytes.len() as u32);
    arr.copy_from(&bytes);
    let parts = js_sys::Array::new();
    parts.push(&arr);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
        &parts,
        web_sys::BlobPropertyBag::new().type_(mime),
    )
    .expect("blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");
    let a = window
        .document()
        .expect("doc")
        .create_element("a")
        .expect("a")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("anchor");
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    web_sys::Url::revoke_object_url(&url).ok();
}

/// 今日文件名戳 "YYYYMMDD"
pub fn today_filename_stamp() -> String {
    get_local_today().replace('-', "")
}
