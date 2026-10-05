//! 应用图标加载（编译期内嵌 icon.ico，避免运行时路径问题）

use egui::IconData;

/// icon.ico 位于仓库根目录，相对本文件向上两级
const ICON_BYTES: &[u8] = include_bytes!("../../icon.ico");

/// 解码为 RGBA 像素 + 尺寸，供 egui 与 tray-icon 共用
pub fn load_rgba() -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(ICON_BYTES)
        .expect("无法解码 icon.ico")
        .to_rgba8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w, h)
}

/// egui 窗口图标
pub fn egui_icon() -> IconData {
    let (rgba, w, h) = load_rgba();
    IconData {
        rgba,
        width: w,
        height: h,
    }
}

/// 系统托盘图标
pub fn tray_icon() -> tray_icon::Icon {
    let (rgba, w, h) = load_rgba();
    tray_icon::Icon::from_rgba(rgba, w, h).expect("无法构造托盘图标")
}
