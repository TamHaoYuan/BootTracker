//! 界面语言（zh-CN / en-US）——桌面浮窗小组件版。
//!
//! 与 `desktop/src/i18n.rs` 同源，但小组件是**独立 crate**，用不了 desktop 的
//! `crate::t!`，所以这里保留一份精简实现：语言是线程局部状态，文案统一走
//! [`t!`] 宏，键就是中文原文，漏翻时回退显示中文（不会出现空白）。
//!
//! 语言来源：后端 `/api/settings` 的 `language`（用户可在 Web/桌面端设置页改），
//! 取不到时按操作系统语言判断（与后端 `detect_system_language()` 同一套规则）。

use std::cell::Cell;

/// 生成好的英文表（中文原文 → 英文译文），条目与 `desktop/src/i18n/en.rs` 同源。
pub mod en;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    ZhCn,
    EnUs,
}

impl Lang {
    pub fn from_code(code: &str) -> Lang {
        if code.eq_ignore_ascii_case("en-US") || code.eq_ignore_ascii_case("en") {
            Lang::EnUs
        } else {
            Lang::ZhCn
        }
    }
}

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::ZhCn) };
}

pub fn set_lang(lang: Lang) {
    CURRENT.with(|c| c.set(lang));
}

pub fn lang() -> Lang {
    CURRENT.with(|c| c.get())
}

pub fn is_en() -> bool {
    lang() == Lang::EnUs
}

/// 把中文原文翻成当前语言的字符串；未登记时返回中文原文。
/// 一般直接用 [`t!`] 宏，不要手动调它。
///
/// 返回 `&str` 而不是 `&'static str`：查不到时要把调用方传进来的 `zh` 原样还回去，
/// 那个引用的生命周期归调用方（宏的 `$s:literal` 臂因此天然是 `'static`）。
pub fn tr(zh: &str) -> &str {
    if is_en() {
        for (k, v) in en::EN_DATA {
            if *k == zh {
                return v;
            }
        }
    }
    zh
}

/// 文案翻译：`t!("本次开机")`。只接受字符串字面量（非字面量原样吐出）。
#[macro_export]
macro_rules! t {
    ($s:literal) => {{
        $crate::i18n::tr($s)
    }};
    ($e:expr) => {
        $e
    };
}

/// 运行期字符串的翻译入口（与 `tr` 等价，语义更直白）。
pub fn label(zh: &str) -> &str {
    tr(zh)
}

/// 带占位符的模板：按出现顺序替换 `{}` / `{name}`（英文译文保留同名占位符）。
pub fn format_placeholders(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    let mut i = 0;
    while let Some(open) = rest.find('{') {
        let Some(close_rel) = rest[open..].find('}') else {
            break;
        };
        let close = open + close_rel;
        let body = &rest[open + 1..close];
        out.push_str(&rest[..open]);
        let (name, spec) = match body.split_once(':') {
            Some((n, s)) => (n, Some(s)),
            None => (body, None),
        };
        if name.is_empty() || name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            if let Some(a) = args.get(i) {
                out.push_str(&render_arg(a, spec));
            } else {
                out.push('{');
                out.push_str(body);
                out.push('}');
            }
            i += 1;
        } else {
            out.push('{');
            out.push_str(body);
            out.push('}');
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// 按 `{:spec}` 渲染参数；只实现 `0` 填充，其余走默认 Display。
fn render_arg(a: &dyn std::fmt::Display, spec: Option<&str>) -> String {
    if let Some(s) = spec {
        if let Some(width) = s.strip_prefix('0').and_then(|w| w.parse::<usize>().ok()) {
            return format!("{:0width$}", a, width = width);
        }
    }
    a.to_string()
}

/// 探测操作系统界面语言（与后端 `server/settings.py` 同一套规则：
/// 先问 Windows，再退回 `LANG`/`LC_ALL`/`LC_MESSAGES`；只有 `zh` 开头算中文）。
pub fn detect_os_language() -> String {
    #[cfg(target_os = "windows")]
    {
        if let Some(name) = windows_locale_name() {
            return normalize_locale(&name);
        }
    }
    for key in ["LANG", "LC_ALL", "LC_MESSAGES"] {
        if let Ok(v) = std::env::var(key) {
            if !v.is_empty() {
                return normalize_locale(&v);
            }
        }
    }
    "en-US".to_string()
}

fn normalize_locale(raw: &str) -> String {
    if raw.to_ascii_lowercase().starts_with("zh") {
        "zh-CN".to_string()
    } else {
        "en-US".to_string()
    }
}

#[cfg(target_os = "windows")]
fn windows_locale_name() -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(lp_locale_name: *mut u16, cch_locale_name: i32) -> i32;
    }
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    let mut buf = [0u16; LOCALE_NAME_MAX_LENGTH];
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH as i32) };
    if n <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..(n as usize - 1)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_switches_copy() {
        set_lang(Lang::EnUs);
        assert_eq!(t!("{}  本次开机"), "{}  this boot");
        assert_eq!(t!("{}  隐藏组件"), "{}  hide widget");
        set_lang(Lang::ZhCn);
        assert_eq!(t!("{}  本次开机"), "{}  本次开机");
    }

    #[test]
    fn placeholders_are_replaced() {
        assert_eq!(format_placeholders("{h}h {m}m {s}s", &[&2, &3, &4]), "2h 3m 4s");
        assert_eq!(format_placeholders("{}  x {}", &[&1, &"y"]), "1  x y");
        assert_eq!(format_placeholders("{}  保留 {}", &[&1]), "1  保留 {}");
    }

    /// 英文表里不应存在「键等于值」的条目（漏翻）。
    ///
    /// 注意：这张表与 desktop 共用同一份生成产物（含 desktop 的条目），
    /// 其中 `简体中文` 是语言自称，刻意保持原样，所以这里放行。
    #[test]
    fn no_untranslated_entries() {
        let bad: Vec<&str> = en::EN_DATA
            .iter()
            .filter(|(k, v)| k == v && *k != "简体中文")
            .map(|(k, _)| *k)
            .collect();
        assert!(bad.is_empty(), "未翻译条目: {bad:?}");
    }
}
