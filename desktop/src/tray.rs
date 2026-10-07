//! 系统托盘（tray-icon 原生实现，独立于 eframe 事件循环）
//!
//! tray-icon 0.24 的事件模型：菜单事件与托盘事件都通过**全局通道**分发，
//! 并没有 per-builder 回调（没有 `with_on_menu_event`）。
//! 这里用两个后台线程分别消费 `MenuEvent` / `TrayIconEvent` 通道，
//! 再转发成 `TrayAction` 交给 UI 线程处理。

use std::sync::mpsc::Sender;

use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem},
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

/// 托盘菜单触发的动作，由 App 在 update 中消费
pub enum TrayAction {
    Show,
    Hide,
    Quit,
    ToggleWidget,
}

/// 构建托盘图标与菜单
///
/// 返回 (TrayIcon 句柄, 小组件勾选项)。句柄需保留以防止被丢弃导致托盘消失。
pub fn build_tray(
    tx: Sender<TrayAction>,
    icon: tray_icon::Icon,
) -> Option<(TrayIcon, CheckMenuItem)> {
    let show_item = MenuItem::new(crate::t!("显示窗口"), true, None);
    let hide_item = MenuItem::new(crate::t!("隐藏窗口"), true, None);
    let widget_item = CheckMenuItem::new(crate::t!("桌面小组件"), true, false, None);
    let quit_item = MenuItem::new(crate::t!("退出"), true, None);

    // 记录各菜单项 id，事件到来时据此判定来源（id 由 muda 自动生成）
    let show_id = show_item.id().clone();
    let hide_id = hide_item.id().clone();
    let widget_id = widget_item.id().clone();
    let quit_id = quit_item.id().clone();

    let menu = Menu::with_items(&[&show_item, &hide_item, &widget_item, &quit_item]).ok()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(crate::t!("开机记录"))
        .with_icon(icon)
        .build()
        .ok()?;

    // 菜单事件
    let mtx = tx.clone();
    std::thread::spawn(move || {
        while let Ok(ev) = MenuEvent::receiver().recv() {
            let id = ev.id;
            let action = if id == show_id {
                TrayAction::Show
            } else if id == hide_id {
                TrayAction::Hide
            } else if id == widget_id {
                TrayAction::ToggleWidget
            } else if id == quit_id {
                TrayAction::Quit
            } else {
                continue;
            };
            let _ = mtx.send(action);
        }
    });

    // 托盘左键点击 → 显示窗口
    std::thread::spawn(move || {
        while let Ok(ev) = TrayIconEvent::receiver().recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                let _ = tx.send(TrayAction::Show);
            }
        }
    });

    Some((tray, widget_item))
}
