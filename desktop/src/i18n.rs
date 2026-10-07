//! 界面语言（zh-CN / en-US）
//!
//! 设计要点：
//! - 语言是**全局状态**（与 `appTheme`/`uiScale` 同级），存在 `settings.language` 里，
//!   桌面端与 Web 端读同一个键。
//! - 文案统一通过 [`t!`] / [`tf!`] 宏，键就是中文原文。这样有两层好处：
//!   ① 中文模式零查找、零分配（宏在没命中英文表时直接返回字面量）；
//!   ② **漏翻不会漏字**——最差情况显示中文原文，不会出现空白；
//!   ③ `grep -n 't!("'` 就能列出所有需要英译的位置。
//! - 语言用线程局部读取，不需要把 `&Translator` 一路传进 chart/components/fmt
//!   这些叶子函数（它们也会产出文案，例如「时长/次数」「暂无数据」）。
//! - 占位符统一用 `{}`：`tf!("共 {} 条", n)`。英文语序不同，靠占位符重排。

use std::cell::Cell;

/// 语言枚举。`Auto` 只在设置层出现，解析后不会传到这里。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    ZhCn,
    EnUs,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::ZhCn => "zh-CN",
            Lang::EnUs => "en-US",
        }
    }

    pub fn from_code(code: &str) -> Lang {
        if code.eq_ignore_ascii_case("en-US") || code.eq_ignore_ascii_case("en") {
            Lang::EnUs
        } else {
            Lang::ZhCn
        }
    }

    /// 语言自己的名字（语言选择器里显示，不随当前语言翻译）
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::ZhCn => "简体中文",
            Lang::EnUs => "English",
        }
    }
}

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::ZhCn) };
}

/// 设置当前语言（在 `app_update()` 里跟随 settings 变化调用）
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
/// 一般直接用 [`t!`] 宏，不要手动调它（宏才能保住 `'static`）。
///
/// 返回 `&'static str` 而不是 `&str`：调用点常写成
/// `if cfg { t!("开启") } else { "关闭" }`——若返回值借用了输入的 `zh`
/// （`&str` 会带着入参的生命周期），就会和 `'static` 的字面量分支对不上，
/// 直接编译报 `if and else have incompatible types`。加 `'static` 约束后
/// 非字面量入参会在编译期被拒（这正是我们要的：`t!` 只该接字面量）。
pub fn tr(zh: &'static str) -> &'static str {
    if is_en() {
        for (k, v) in EN {
            if *k == zh {
                return v;
            }
        }
    }
    zh
}

/// 文案翻译：`t!("累计开机")`。
///
/// 只接受**字符串字面量**：中文模式下直接返回字面量（无查表、无分配），
/// 英文模式下查 [`EN`] 表；两者都不会 panic，漏翻只会显示中文。
///
/// 注意 `$s:literal` 会连 `t!(if on { "开启" } else { "关闭" })` 这种非字面量
/// 一起匹配掉，所以第二条臂用 `$e:expr` 兜底，直接吐出表达式本身。
#[macro_export]
macro_rules! t {
    ($s:literal) => {{
        #[allow(unused_braces)]
        {
            $crate::i18n::tr($s)
        }
    }};
    ($e:expr) => {
        $e
    };
}

/// 带占位符的文案：`tf!("共 {} 条", n)`，英文语序可自由重排 `{}`。
#[macro_export]
macro_rules! tf {
    ($s:literal $(, $arg:expr)* $(,)?) => {
        $crate::i18n::format_placeholders(
            $crate::t!($s),
            &[$(&$arg as &dyn ::std::fmt::Display),*],
        )
    };
}

/// 替换模板里的占位符，支持三种写法，参数一律**按出现顺序**消费：
///
/// - `{}`：位置占位（推荐，中英一致）；
/// - `{e}` / `{what}`：命名占位——中文原文里大量使用 `format!("导出失败: {e}")`
///   这种具名插值，译文里保留同名占位符更不容易错位，这里统一当位置占位处理；
/// - `{s:02}`：带格式化规格的命名占位——目前只需要 `0` 填充（`{:02}`），
///   其余规格退化成普通输出，避免为了一个补零上 `format_args!` 的动态分发。
///
/// 参数不足时原样保留占位符（便于一眼发现漏传）。
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
        // `{{` 之类的转义不处理（界面文案里不会出现），`{}`/`{name}`/`{name:spec}` 都接受
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

/// 按 `{:spec}` 渲染一个参数。只实现 `0` 填充（`02` 这类），其余走默认 Display。
fn render_arg(a: &dyn std::fmt::Display, spec: Option<&str>) -> String {
    if let Some(s) = spec {
        if let Some(width) = s.strip_prefix('0').and_then(|w| w.parse::<usize>().ok()) {
            return format!("{:0width$}", a, width = width);
        }
    }
    a.to_string()
}

/// 探测操作系统界面语言。
///
/// 与后端 `server/settings.py:35 detect_system_language()` 保持同一套判断：
/// 先问 Windows 的 `GetUserDefaultLocaleName`，失败再退回 `LANG`/`LC_ALL` 环境变量；
/// 只有 `zh` 开头才判中文，其余一律 `en-US`（用户可在设置页覆盖）。
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
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("zh") {
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
    // 返回写入的字符数（含结尾 NUL），0 表示失败
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH as i32) };
    if n <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..(n as usize - 1)]))
}

/// 运行期字符串的翻译入口：和 [`t!`] 等价，只是接受 `&str` 而不是字面量。
///
/// 宏版本更省（中文模式零查找），但这个函数在文案来自变量
/// （数据库里的标签、枚举转出来的名字）时更好用：
/// `egui::RichText::new(i18n::label(&s))`。返回 `String` 是因为查表命中的是
/// `&'static str`、未命中要回退调用方传入的 `&str`，两者生命周期不同。
pub fn label(zh: &str) -> String {
    if is_en() {
        for (k, v) in EN {
            if *k == zh {
                return (*v).to_string();
            }
        }
    }
    zh.to_string()
}

/// 带占位符的文案的**安全版**：`tr_or(&t!("{what}成功"), &format!("{what}成功"))`。
///
/// 直接写 `t!("{what}成功")` 在英文模式下如果表里没有这条，会返回 `""`（因为
/// 原串是运行期拼出来的，没法回退成 `'static`）。这个函数在查不到时用调用方
/// 传进来的中文原文兜底，保证界面不会出现空提示。
pub fn tr_or<'a>(translated: &'a str, fallback: &'a str) -> &'a str {
    if translated.is_empty() {
        fallback
    } else {
        translated
    }
}

/// 英文表：中文原文 → 英文译文。由 `.i18n/build_rust_en.py` 从翻译工作单生成。
///
/// 刻意只生成**一个** `en.rs`：`concat!` 不能展开宏产出的数组（实测
/// `error: expected a literal`），而在 stable Rust 里合并两段 `&[(&str, &str)]`
/// 常量需要额外依赖，不值得。
pub mod en;

/// 英文表（中文原文 → 英文译文）。
static EN: &[(&str, &str)] = en::EN_DATA;

/// 解析 `settings.json` 文本里的 `language`。
///
/// 抽成纯函数是为了可测：直接测文件读取就得 `set_current_dir`，而测试是并行跑的，
/// 改进程级当前目录会让别的用例抽风。
pub fn lang_from_settings_text(text: &str) -> Option<Lang> {
    let json: serde_json::Value = serde_json::from_str(text).ok()?;
    let code = json.get("language").and_then(|v| v.as_str())?;
    if code.is_empty() {
        // 空串 = 跟随系统；返回 None 让调用方去问操作系统
        None
    } else {
        Some(Lang::from_code(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 设置文件里的 `language` 决定启动语言；空串/缺字段时交回系统语言。
    #[test]
    fn settings_text_decides_startup_language() {
        assert_eq!(
            lang_from_settings_text(r#"{"language":"en-US","appMode":"dark"}"#),
            Some(Lang::EnUs)
        );
        assert_eq!(lang_from_settings_text(r#"{"language":"zh-CN"}"#), Some(Lang::ZhCn));
        assert_eq!(lang_from_settings_text(r#"{"language":""}"#), None);
        assert_eq!(lang_from_settings_text(r#"{"appMode":"dark"}"#), None);
        assert_eq!(lang_from_settings_text("not json"), None);
    }

    #[test]
    fn missing_entry_falls_back_to_source() {
        set_lang(Lang::ZhCn);
        assert_eq!(t!("这是一个未登记的文案"), "这是一个未登记的文案");
        set_lang(Lang::EnUs);
        assert_eq!(t!("这是一个未登记的文案"), "这是一个未登记的文案");
        set_lang(Lang::ZhCn);
    }

    #[test]
    fn placeholders_are_replaced_in_order() {
        assert_eq!(format_placeholders("共 {} 条", &[&3]), "共 3 条");
        assert_eq!(format_placeholders("a {} b {} c", &[&1, &"x"]), "a 1 b x c");
        // 参数不足时保留占位符，便于发现漏传
        assert_eq!(format_placeholders("a {}", &[]), "a {}");
    }

    #[test]
    fn lang_roundtrip() {
        assert_eq!(Lang::from_code("en-US"), Lang::EnUs);
        assert_eq!(Lang::from_code("EN-us"), Lang::EnUs);
        assert_eq!(Lang::from_code("zh-CN"), Lang::ZhCn);
        assert_eq!(Lang::from_code(""), Lang::ZhCn);
        assert_eq!(Lang::EnUs.code(), "en-US");
    }

    /// 真正的端到端校验：切到英文后，**真实界面文案**必须查到译文。
    /// 这条用几个跨文件的代表串（侧栏、KPI、设置、托盘、图表轴单位）钉住
    /// 「EN 表被正确接进 i18n.rs」这件事——表没接上时它们会原样返回中文。
    #[test]
    fn english_table_is_actually_wired() {
        set_lang(Lang::EnUs);
        assert_eq!(t!("仪表盘"), "Dashboard");
        assert_eq!(t!("累计开机"), "Total boot time");
        assert_eq!(t!("累计关机"), "Total shutdown time");
        assert_eq!(t!("平均时长"), "Average duration");
        assert_eq!(t!("进行中"), "In progress");
        assert_eq!(t!("已连接"), "Connected");
        assert_eq!(t!("开机记录"), "Boot Records");
        assert_eq!(t!("关闭"), "Off");
        set_lang(Lang::ZhCn);
    }

    /// 带命名占位符的模板也要能被 `format_placeholders` 吃掉（英文表里保留了
    /// `{e}` / `{what}` / `{s:02}` 这类写法）。
    #[test]
    fn named_placeholders_and_padding() {
        set_lang(Lang::EnUs);
        let tpl = t!("导出失败: {e}");
        assert_eq!(format_placeholders(tpl, &[&"disk full"]), "Export failed: disk full");
        set_lang(Lang::ZhCn);
        assert_eq!(
            format_placeholders("时长 {s:02}", &[&7]),
            "时长 07"
        );
    }

    /// 操作系统语言探测必须返回后端认识的两个码之一，否则 `PUT /api/settings`
    /// 会把 `language` 判为非法。这条同时覆盖 Windows FFI（`GetUserDefaultLocaleName`）
    /// 那一支：写缓冲区长度算错会直接 panic，跑得过就说明 ABI 没问题。
    #[test]
    fn os_language_is_one_of_the_supported_codes() {
        let code = detect_os_language();
        assert!(
            code == "zh-CN" || code == "en-US",
            "非法的语言码: {code:?}"
        );
    }

}

