#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""桌面小组件模块 — 基于 Tkinter 的无边框置顶浮动卡片

在桌面常驻显示今日开机次数和当前会话运行时长，
无需打开浏览器即可一眼查看关键信息。

特性：
- 无边框、置顶、半透明、圆角
- 可拖动定位，位置记忆
- 右键菜单：打开主界面 / 隐藏组件 / 退出
- 双通道数据获取（HTTP API 优先，回退直接读文件）
"""
import os
import sys
import json
import time
import threading
import urllib.request
from datetime import datetime, timezone

from .logging_config import logger


# ——— 工具函数 ———

def _utc_iso_to_local_dt(iso_str):
    """ISO 字符串 → 本地 datetime 对象"""
    try:
        dt = datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
        return dt.astimezone()
    except Exception:
        return None


def _local_today_str():
    """本地今天的 YYYY-MM-DD"""
    return time.strftime("%Y-%m-%d")


def _fmt_duration(ms):
    """毫秒 → 'H时M分S秒' 格式"""
    if not ms or ms <= 0:
        return "—"
    s = int(ms // 1000)
    h = s // 3600
    m = (s % 3600) // 60
    sec = s % 60
    if h > 0:
        return f"{h}时{m}分{sec}秒"
    if m > 0:
        return f"{m}分{sec}秒"
    return f"{sec}秒"


# ——— 数据获取 ———

def _fetch_data_via_http(port):
    """通过 HTTP API 获取数据（优先通道）"""
    try:
        url = f"http://127.0.0.1:{port}/api/data"
        with urllib.request.urlopen(url, timeout=2) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except Exception:
        return None


def _fetch_data_via_file():
    """直接读取数据文件（回退通道）"""
    try:
        from .data_store import load_data
        return load_data()
    except Exception:
        return None


def _fetch_data(port):
    """双通道获取数据，优先 HTTP，失败回退文件"""
    data = _fetch_data_via_http(port)
    if data and "sessions" in data:
        return data
    return _fetch_data_via_file()


# ——— 组件核心 ———

class DesktopWidget:
    """桌面浮动小组件"""

    WIDTH = 220
    HEIGHT = 108

    # 透明色（用于圆角效果，Windows 专用）
    TRANSPARENT_COLOR = "#000001"

    def __init__(self, port, stop_event):
        self.port = port
        self.stop_event = stop_event
        self._widget_stop = threading.Event()  # 组件独立停止事件
        self._data = None
        self._active_session = None
        self._boot_dt = None
        self._drag_start_x = 0
        self._drag_start_y = 0
        self._last_data_fetch = 0

    def request_close(self):
        """从外部请求关闭组件（不影响整个应用）"""
        self._widget_stop.set()
        try:
            self.root.after(100, self._close)
        except Exception:
            pass

    def _load_position(self):
        """从设置加载组件位置，默认右下角"""
        try:
            from .settings import load_settings
            settings = load_settings()
            pos = settings.get("widgetPosition", "")
            if pos and "," in pos:
                x, y = pos.split(",", 1)
                return int(x), int(y)
        except Exception:
            pass
        # 默认：屏幕右下角，留出边距
        try:
            import tkinter as tk
            screen_w = tk.Tk().winfo_screenwidth()
            screen_h = tk.Tk().winfo_screenheight()
            return screen_w - self.WIDTH - 20, screen_h - self.HEIGHT - 80
        except Exception:
            return 100, 100

    def _save_position(self, x, y):
        """保存组件位置到设置"""
        try:
            from .settings import load_settings, save_settings
            settings = load_settings()
            settings["widgetPosition"] = f"{x},{y}"
            save_settings(settings)
        except Exception:
            pass

    def _create_window(self):
        """创建 Tkinter 窗口"""
        import tkinter as tk
        from tkinter import font as tkfont

        self.root = tk.Tk()
        self.root.title("BootTracker Widget")
        self.root.overrideredirect(True)
        self.root.attributes("-topmost", True)
        self.root.attributes("-alpha", 0.92)

        # 字体选择：优先思源黑体，回退微软雅黑
        available = set(tkfont.families())
        self._font_family = "Microsoft YaHei UI"
        for name in ("Source Han Sans SC", "Source Han Sans CN",
                     "Noto Sans SC", "Source Han Sans Heavy"):
            if name in available:
                self._font_family = name
                break

        # Windows 透明色实现圆角
        if sys.platform == "win32":
            try:
                self.root.attributes("-transparentcolor", self.TRANSPARENT_COLOR)
                self.root.config(bg=self.TRANSPARENT_COLOR)
            except Exception:
                self.root.config(bg="#1e293b")
        else:
            self.root.config(bg="#1e293b")

        # 定位窗口
        x, y = self._load_position()
        self.root.geometry(f"{self.WIDTH}x{self.HEIGHT}+{x}+{y}")

        # 主卡片容器（使用 Canvas 绘制圆角背景）
        self.canvas = tk.Canvas(
            self.root,
            width=self.WIDTH,
            height=self.HEIGHT,
            bg=self.TRANSPARENT_COLOR if sys.platform == "win32" else "#1e293b",
            highlightthickness=0,
        )
        self.canvas.pack()

        # 绘制圆角矩形背景
        self._draw_background()

        # 状态点
        self.status_dot = self.canvas.create_oval(
            14, 14, 24, 24,
            fill="#64748b", outline="",
        )

        # 标题
        self.canvas.create_text(
            32, 19,
            text="开机记录",
            fill="#94a3b8",
            font=tkfont.Font(family=self._font_family, size=9, weight="normal"),
            anchor="w",
        )

        # 今日开机次数（大号数字）
        self.count_label = self.canvas.create_text(
            20, 56,
            text="0",
            fill="#f1f5f9",
            font=tkfont.Font(family=self._font_family, size=28, weight="bold"),
            anchor="w",
        )

        # "今日开机"标签
        self.canvas.create_text(
            68, 64,
            text="今日开机",
            fill="#64748b",
            font=tkfont.Font(family=self._font_family, size=9),
            anchor="w",
        )

        # 运行时长
        self.time_label = self.canvas.create_text(
            20, 90,
            text="运行时长 —",
            fill="#94a3b8",
            font=tkfont.Font(family=self._font_family, size=9),
            anchor="w",
        )

        # 绑定拖动事件
        self.canvas.bind("<Button-1>", self._on_drag_start)
        self.canvas.bind("<B1-Motion>", self._on_drag_motion)
        # 绑定右键菜单
        self.canvas.bind("<Button-3>", self._on_right_click)

    def _draw_background(self):
        """绘制圆角背景"""
        r = 14
        w, h = self.WIDTH, self.HEIGHT
        self.canvas.create_arc(0, 0, 2 * r, 2 * r, start=90, extent=90,
                               style="pieslice", fill="#1e293b", outline="")
        self.canvas.create_arc(w - 2 * r, 0, w, 2 * r, start=0, extent=90,
                               style="pieslice", fill="#1e293b", outline="")
        self.canvas.create_arc(0, h - 2 * r, 2 * r, h, start=180, extent=90,
                               style="pieslice", fill="#1e293b", outline="")
        self.canvas.create_arc(w - 2 * r, h - 2 * r, w, h, start=270, extent=90,
                               style="pieslice", fill="#1e293b", outline="")
        self.canvas.create_rectangle(r, 0, w - r, h, fill="#1e293b", outline="")
        self.canvas.create_rectangle(0, r, w, h - r, fill="#1e293b", outline="")

    def _on_drag_start(self, event):
        self._drag_start_x = event.x
        self._drag_start_y = event.y

    def _on_drag_motion(self, event):
        x = self.root.winfo_x() + event.x - self._drag_start_x
        y = self.root.winfo_y() + event.y - self._drag_start_y
        self.root.geometry(f"+{x}+{y}")

    def _on_right_click(self, event):
        """右键菜单"""
        import tkinter as tk
        menu = tk.Menu(self.root, tearoff=0, bg="#1e293b", fg="#f1f5f9",
                       activebackground="#334155", activeforeground="#f1f5f9",
                       borderwidth=0, relief="flat")
        menu.add_command(label="打开主界面", command=self._open_main_ui)
        menu.add_command(label="隐藏组件", command=self._hide_widget)
        menu.add_separator()
        menu.add_command(label="退出程序", command=self._exit_app)
        try:
            menu.tk_popup(event.x_root, event.y_root)
        finally:
            menu.grab_release()

    def _open_main_ui(self):
        """打开主界面：Qt 窗口优先，回退浏览器"""
        try:
            from .qt_window import show_main_window
            if show_main_window():
                return
        except Exception:
            pass
        import webbrowser
        try:
            webbrowser.open(f"http://localhost:{self.port}")
        except Exception:
            pass

    def _hide_widget(self):
        """隐藏组件并保存设置"""
        try:
            from .settings import load_settings, save_settings
            settings = load_settings()
            settings["widgetEnabled"] = False
            save_settings(settings)
        except Exception:
            pass
        self.root.after(100, self._close)

    def _exit_app(self):
        """退出整个应用"""
        self.stop_event.set()
        self.root.after(100, self._close)

    def _close(self):
        try:
            x = self.root.winfo_x()
            y = self.root.winfo_y()
            self._save_position(x, y)
        except Exception:
            pass
        self.root.quit()
        self.root.destroy()

    def _update_data(self):
        """拉取最新数据并更新活跃会话"""
        now = time.time()
        # 每 30 秒拉取一次数据
        if now - self._last_data_fetch > 30 or self._data is None:
            self._data = _fetch_data(self.port)
            self._last_data_fetch = now
            # 查找活跃会话
            self._active_session = None
            self._boot_dt = None
            if self._data and self._data.get("sessions"):
                for s in self._data["sessions"]:
                    if not s.get("shutdownTime"):
                        self._active_session = s
                        self._boot_dt = _utc_iso_to_local_dt(s.get("bootTime", ""))
                        break

    def _refresh_ui(self):
        """刷新 UI 显示"""
        import tkinter as tk

        if not self._data:
            self._update_data()

        if not self._data:
            return

        # 今日开机次数
        today = _local_today_str()
        today_count = 0
        for s in self._data.get("sessions", []):
            boot_dt = _utc_iso_to_local_dt(s.get("bootTime", ""))
            if boot_dt and boot_dt.strftime("%Y-%m-%d") == today:
                today_count += 1

        self.canvas.itemconfig(self.count_label, text=str(today_count))

        # 运行时长和状态
        if self._active_session and self._boot_dt:
            now_local = datetime.now().astimezone()
            duration_ms = (now_local - self._boot_dt).total_seconds() * 1000
            self.canvas.itemconfig(self.time_label,
                                   text=f"运行时长 {_fmt_duration(duration_ms)}")
            self.canvas.itemconfig(self.status_dot, fill="#10b981")
        else:
            self.canvas.itemconfig(self.time_label, text="已关机")
            self.canvas.itemconfig(self.status_dot, fill="#64748b")

    def _tick(self):
        """定时刷新循环"""
        if self.stop_event.is_set() or self._widget_stop.is_set():
            self._close()
            return
        # 每秒更新 UI（计时），每 30 秒拉取数据
        self._update_data()
        self._refresh_ui()
        self.root.after(1000, self._tick)

    def run(self):
        """启动组件主循环"""
        try:
            self._create_window()
            self._tick()
            logger.info("[widget] desktop widget started")
            self.root.mainloop()
        except Exception as e:
            logger.error(f"[widget] error: {e}")
        finally:
            logger.info("[widget] desktop widget stopped")


def start_widget(port, stop_event):
    """在子线程中启动桌面组件（阻塞调用）

    Args:
        port: HTTP 服务器端口
        stop_event: 停止事件，set() 后组件退出
    """
    widget = DesktopWidget(port, stop_event)
    _set_widget_instance(widget)
    widget.run()
    _set_widget_instance(None)


# ——— 全局组件实例引用（用于外部停止）———

_widget_instance = None
_widget_lock = threading.Lock()


def _set_widget_instance(widget):
    global _widget_instance
    with _widget_lock:
        _widget_instance = widget


def stop_widget():
    """从外部停止正在运行的桌面组件"""
    global _widget_instance
    with _widget_lock:
        widget = _widget_instance
    if widget is not None:
        try:
            widget.request_close()
        except Exception:
            pass


def is_widget_running():
    """查询桌面组件是否正在运行"""
    with _widget_lock:
        return _widget_instance is not None
