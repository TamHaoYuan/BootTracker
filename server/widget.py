#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""桌面小组件模块 — 启动纯原生 Rust 浮窗进程

替代旧版 Tkinter 组件（见 deprecated/widget_tkinter.py）与原 Tauri 小组件。
小组件本体是独立原生浮窗（desktop-widget crate，egui/eframe）：
- 无边框、置顶、半透明、可拖动、位置记忆（settings.json widgetPosition）
- 每 30 秒从 /api/data 拉取数据
- 右键菜单：打开主界面 / 隐藏组件（PUT settings widgetEnabled=false 后由本模块停止）

本模块仅负责进程生命周期管理，接口与旧版保持一致：
start_widget(port, stop_event) / stop_widget() / is_widget_running()
"""
import os
import sys
import subprocess
import threading

from .config import APP_DIR, SETTINGS_FILE, _RESOURCE_DIR
from .logging_config import logger

_widget_proc: subprocess.Popen | None = None
_proc_lock = threading.Lock()


def _get_widget_binary_path() -> str:
    """获取 Rust 小组件编译产物路径（release 优先，debug 兜底）

    打包后二进制在 _RESOURCE_DIR，开发时在 APP_DIR
    """
    for base in (APP_DIR, _RESOURCE_DIR):
        for build in ("release", "debug"):
            path = os.path.join(base, "desktop-widget", "target", build, "boot-tracker-widget.exe")
            if os.path.exists(path):
                return path
    return ""


def is_widget_available() -> bool:
    """检测小组件二进制是否已编译"""
    return bool(_get_widget_binary_path())


def start_widget(port, stop_event) -> None:
    """启动 Rust 桌面小组件进程（非阻塞）

    Args:
        port: HTTP 服务器端口（注入 BOOTTRACKER_PORT 供组件拉取数据）
        stop_event: 保留参数（与旧版签名兼容），进程停止由 stop_widget() 负责
    """
    global _widget_proc

    with _proc_lock:
        if _widget_proc is not None and _widget_proc.poll() is None:
            logger.info("[widget] already running")
            return

        binary = _get_widget_binary_path()
        if not binary:
            logger.error(
                "[widget] no compiled widget binary found; build first: "
                "cd desktop-widget && cargo build --release"
            )
            return

        env = os.environ.copy()
        env["BOOTTRACKER_PORT"] = str(port)
        env["BOOTTRACKER_SETTINGS"] = SETTINGS_FILE

        try:
            _widget_proc = subprocess.Popen(
                [binary],
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
            )
            logger.info(f"[widget] rust widget started (pid={_widget_proc.pid})")
        except Exception as e:
            logger.error(f"[widget] failed to start: {e}")
            _widget_proc = None


def stop_widget() -> None:
    """停止小组件进程"""
    global _widget_proc

    with _proc_lock:
        if _widget_proc is None:
            return

        if _widget_proc.poll() is None:
            logger.info("[widget] stopping process")
            try:
                _widget_proc.terminate()
                _widget_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                logger.warning("[widget] force killing process")
                _widget_proc.kill()
            except Exception as e:
                logger.error(f"[widget] error stopping: {e}")

        _widget_proc = None
        logger.info("[widget] process stopped")


def is_widget_running() -> bool:
    """查询小组件进程是否在运行"""
    with _proc_lock:
        return _widget_proc is not None and _widget_proc.poll() is None
