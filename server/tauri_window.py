#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""桌面窗口管理模块（纯原生 egui/eframe）

替代原 PyQt5 WebEngine 窗口层与 Tauri WebView 壳，通过子进程方式启动
纯原生 Rust 桌面程序（无 WebView / 无 HTML）。

设计要点：
- 桌面程序作为独立进程运行，经 HTTP 访问 Python 后端 REST API
- 开发模式：使用 `cargo run` 启动
- 生产模式：运行编译好的原生可执行文件（desktop/target/.../boot-tracker.exe）
- 窗口生命周期由 Python 主进程管理（启动/停止）
- 打包后二进制在 _RESOURCE_DIR，开发时在 APP_DIR
- raise_tauri_window / set_tauri_theme 通过 Windows API 直接操作窗口句柄
"""

import os
import sys
import subprocess
import threading
import logging

from .config import APP_DIR, PORT, _RESOURCE_DIR

logger = logging.getLogger(__name__)

# 模块级状态
_tauri_proc: subprocess.Popen | None = None
_proc_lock = threading.Lock()

# 桌面主窗口标题（与原生程序设置的窗口标题一致，FindWindowW 据此定位）
_WINDOW_TITLE = "开机记录"


def _get_tauri_binary_path() -> str:
    """获取 Tauri 编译后的可执行文件路径（生产模式）

    打包后二进制在 _RESOURCE_DIR，开发时在 APP_DIR
    """
    for base in (APP_DIR, _RESOURCE_DIR):
        for build in ("release", "debug"):
            path = os.path.join(base, "desktop", "target", build, "boot-tracker.exe")
            if os.path.exists(path):
                return path
    return ""


def _get_tauri_source_dir() -> str:
    """获取桌面端源码目录（开发模式用）"""
    for base in (APP_DIR, _RESOURCE_DIR):
        cargo_toml = os.path.join(base, "desktop", "Cargo.toml")
        if os.path.exists(cargo_toml):
            return os.path.join(base, "desktop")
    return ""


def is_tauri_available() -> bool:
    """检测 Tauri 应用是否可用（已编译或可通过 cargo 运行）"""
    if _get_tauri_binary_path():
        return True
    return bool(_get_tauri_source_dir())


def start_tauri(dev_mode: bool = False) -> bool:
    """启动 Tauri 应用进程

    Args:
        dev_mode: 是否使用开发模式（cargo tauri dev，支持热重载）

    Returns:
        是否成功启动
    """
    global _tauri_proc

    with _proc_lock:
        if _tauri_proc is not None and _tauri_proc.poll() is None:
            logger.info("[tauri] already running")
            return True

        tauri_dir = None
        if dev_mode:
            tauri_dir = _get_tauri_source_dir()
            if not tauri_dir:
                logger.error("[tauri] no Cargo.toml found for dev mode")
                return False
            cmd = ["cargo", "tauri", "dev"]
            env = os.environ.copy()
            env["RUSTUP_DIST_SERVER"] = "https://rsproxy.cn"
        else:
            binary = _get_tauri_binary_path()
            if not binary:
                logger.error("[tauri] no compiled binary found; build first with: cargo build --release")
                return False
            cmd = [binary]
            env = os.environ.copy()
            # 注入端口：原生桌面端经此端口访问 Python 后端 REST API
            env["BOOTTRACKER_PORT"] = str(PORT)

        try:
            logger.info(f"[tauri] starting: {' '.join(cmd)}")
            _tauri_proc = subprocess.Popen(
                cmd,
                cwd=tauri_dir,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                creationflags=subprocess.CREATE_NO_WINDOW if sys.platform == "win32" else 0,
            )
            logger.info(f"[tauri] process started (pid={_tauri_proc.pid})")
            return True
        except Exception as e:
            logger.error(f"[tauri] failed to start: {e}")
            _tauri_proc = None
            return False


def stop_tauri() -> None:
    """停止 Tauri 应用进程"""
    global _tauri_proc

    with _proc_lock:
        if _tauri_proc is None:
            return

        if _tauri_proc.poll() is None:
            logger.info("[tauri] stopping process")
            try:
                _tauri_proc.terminate()
                _tauri_proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                logger.warning("[tauri] force killing process")
                _tauri_proc.kill()
            except Exception as e:
                logger.error(f"[tauri] error stopping: {e}")

        _tauri_proc = None
        logger.info("[tauri] process stopped")


def get_tauri_proc() -> subprocess.Popen | None:
    """获取当前 Tauri 进程对象（看门狗用于 poll/wait）"""
    with _proc_lock:
        return _tauri_proc


def reset_tauri_proc() -> None:
    """清除已退出的进程引用，使 start_tauri 可再次启动（看门狗重启用）"""
    global _tauri_proc
    with _proc_lock:
        _tauri_proc = None


def _find_tauri_window() -> int:
    """通过窗口标题查找 Tauri 主窗口句柄（Windows）"""
    if sys.platform != "win32":
        return 0
    try:
        import ctypes
        user32 = ctypes.windll.user32
        return user32.FindWindowW(None, _WINDOW_TITLE)
    except Exception:
        return 0


def raise_tauri_window() -> bool:
    """唤起 Tauri 窗口。

    先检查进程存活，再通过 Windows API 查找窗口句柄并唤起（SW_RESTORE + SetForegroundWindow）。
    进程存活但窗口操作失败时仍返回 True（不回退浏览器）。
    """
    with _proc_lock:
        if _tauri_proc is None or _tauri_proc.poll() is not None:
            return False

    if sys.platform == "win32":
        try:
            import ctypes
            user32 = ctypes.windll.user32
            hwnd = _find_tauri_window()
            if hwnd:
                # SW_RESTORE = 9
                user32.ShowWindow(hwnd, 9)
                user32.SetForegroundWindow(hwnd)
                return True
            logger.warning("[tauri] window not found by title")
        except Exception as e:
            logger.error(f"[tauri] raise window failed: {e}")

    # 进程存活但窗口操作失败，仍返回 True（不回退浏览器）
    return True


def set_tauri_theme(mode: str) -> bool:
    """设置 Tauri 窗口主题（通过 Windows DWM API 直接设置标题栏暗色/浅色）

    本地用户通常通过前端 Tauri IPC 调用 Rust set_theme 命令；
    远程浏览器用户通过 HTTP API 触发此函数，同步桌面端窗口标题栏。
    """
    if sys.platform != "win32":
        return True
    try:
        import ctypes
        hwnd = _find_tauri_window()
        if not hwnd:
            return False
        dark = mode != "light"
        val = ctypes.c_int(1 if dark else 0)
        # DWMWA_USE_IMMERSIVE_DARK_MODE = 20 (Win10 2004+/Win11), 19 for older
        for attr in (20, 19):
            ok = ctypes.windll.dwmapi.DwmSetWindowAttribute(
                hwnd, attr, ctypes.byref(val), ctypes.sizeof(val)
            )
            if ok == 0:
                break
        return True
    except Exception as e:
        logger.error(f"[tauri] set theme failed: {e}")
        return False
