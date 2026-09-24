#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""系统托盘模块 — 负责创建和管理系统托盘图标"""
import os
import threading

from PIL import Image


# 静态图标路径（相对于项目根目录）
_ICON_PATH = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "static", "icons", "icon.png")


def load_icon_image():
    """加载静态应用图标，返回 PIL RGBA Image。

    供系统托盘与 Tauri 主窗口任务栏图标共用。
    """
    return Image.open(_ICON_PATH).convert("RGBA")


def create_tray_icon(stop_event):
    img = load_icon_image()

    def show_window(icon, item=None):
        """唤起主窗口：Tauri 优先，回退系统浏览器"""
        try:
            from .tauri_window import raise_tauri_window
            if raise_tauri_window():
                return
        except Exception as e:
            from .logging_config import logger
            logger.error(f"[tray] raise_tauri_window failed: {e}")
        try:
            import webbrowser
            from .config import PORT
            webbrowser.open(f"http://localhost:{PORT}")
        except Exception:
            pass

    def _is_widget_enabled():
        try:
            from .settings import load_settings
            return load_settings().get("widgetEnabled", False)
        except Exception:
            return False

    def toggle_widget(icon, item=None):
        try:
            from .settings import load_settings, save_settings
            settings = load_settings()
            settings["widgetEnabled"] = not settings.get("widgetEnabled", False)
            save_settings(settings)
            if settings["widgetEnabled"]:
                from .config import PORT
                from .widget import start_widget
                threading.Thread(
                    target=start_widget,
                    args=(PORT, stop_event),
                    daemon=True,
                ).start()
            else:
                from .widget import stop_widget
                stop_widget()
        except Exception as e:
            from .logging_config import logger
            logger.error(f"[tray] toggle widget failed: {e}")

    def exit_app(icon, item=None):
        """干净退出：设置 stop_event 后由 main() 走清理流程"""
        if stop_event:
            stop_event.set()
        try:
            icon.stop()
        except Exception:
            pass
        os._exit(0)

    import pystray
    menu = pystray.Menu(
        pystray.MenuItem("显示窗口", show_window, default=True),
        pystray.MenuItem(
            "桌面组件",
            toggle_widget,
            checked=lambda item: _is_widget_enabled(),
        ),
        pystray.MenuItem("退出", exit_app),
    )
    return pystray.Icon("boottracker", img, "开机记录", menu)