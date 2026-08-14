#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""系统托盘模块 — 负责创建和管理系统托盘图标"""
import os
import math
import threading

from PIL import Image, ImageDraw


def draw_icon_image():
    """绘制应用图标（显示器 + 紫色进度弧），返回 PIL RGBA Image。

    供系统托盘与 Qt 主窗口任务栏图标共用，保证视觉一致。
    """
    size = 64
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    scr_x1, scr_y1, scr_x2, scr_y2 = 10, 8, 54, 44
    draw.rounded_rectangle(
        [scr_x1, scr_y1, scr_x2, scr_y2], radius=4,
        outline=(230, 230, 230, 255), width=2
    )
    stand_cx = 32
    draw.line([(stand_cx - 8, scr_y2), (stand_cx + 8, scr_y2)],
              fill=(230, 230, 230, 255), width=2)
    draw.line([(stand_cx, scr_y2), (stand_cx, scr_y2 + 5)],
              fill=(230, 230, 230, 255), width=2)
    draw.line([(stand_cx - 6, scr_y2 + 5), (stand_cx + 6, scr_y2 + 5)],
              fill=(230, 230, 230, 255), width=2)

    pcx, pcy = 32, 26
    pr = 10
    draw.line([(pcx, pcy - pr + 3), (pcx, pcy - 3)],
              fill=(139, 92, 246, 255), width=2)
    points = []
    for deg in range(210, 331, 3):
        rad = math.radians(deg)
        px = pcx + pr * math.cos(rad)
        py = pcy - pr * math.sin(rad)
        points.append((px, py))
    if len(points) > 1:
        draw.line(points, fill=(139, 92, 246, 255), width=2)
    return img


def create_tray_icon(stop_event):
    img = draw_icon_image()

    def show_window(icon, item=None):
        """唤起主窗口：Qt 优先，回退系统浏览器"""
        try:
            from .qt_window import show_main_window
            if show_main_window():
                return
        except Exception as e:
            from .logging_config import logger
            logger.error(f"[tray] show_main_window failed: {e}")
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
        """干净退出：Qt 主循环退出后由 main() 走清理流程；无 Qt 时硬退出"""
        if stop_event:
            stop_event.set()
        try:
            icon.stop()
        except Exception:
            pass
        try:
            from .qt_window import quit_app
            if quit_app():
                return  # Qt 主循环将退出，main() 执行清理
        except Exception as e:
            from .logging_config import logger
            logger.error(f"[tray] quit_app failed: {e}")
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