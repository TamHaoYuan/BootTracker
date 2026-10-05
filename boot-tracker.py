#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 - 桌面应用入口

应用启动流程：
1. 实例锁检查（防止多开）
2. 自动关闭未结束会话并创建新会话（调试模式 --debug 跳过，不记录开机）
3. 自动备份数据
4. 启动 HTTP 服务器
5. 启动 Tauri 窗口（新版本默认，不可用时回退浏览器）
6. 启动隧道（如配置）
7. 托盘：Tauri 模式由 Rust 托盘接管；仅浏览器回退模式用 pystray
8. 空闲检测线程（如配置）+ Tauri/小组件看门狗
"""

import os
import sys
import socket
import subprocess
import threading
import webbrowser
import time

# 添加应用目录到路径
_APP_DIR = os.path.dirname(os.path.abspath(__file__))
if _APP_DIR not in sys.path:
    sys.path.insert(0, _APP_DIR)


# 导入标准 logging
from server.logging_config import logger


def _ensure_instance_lock(lock_port: int) -> socket.socket | None:
    """创建实例锁（通过端口绑定实现）。

    注意：此处故意不设置 SO_REUSEADDR。在 Windows 上 SO_REUSEADDR 允许多个套接字
    绑定同一端口，会使实例锁失效（导致可多开）。保持默认行为，第二个实例 bind() 会
    失败从而走“唤起已存在窗口”的路径。
    """
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.bind(("127.0.0.1", lock_port))
        sock.listen(1)
        return sock
    except OSError:
        return None


def _tauri_watchdog(stop_event: threading.Event, port: int) -> None:
    """看门狗：Tauri 进程异常退出自动重启；小组件启用但进程不在时自动拉起"""
    from server import tauri_window
    from server.settings import load_settings
    from server.widget import is_widget_available, is_widget_running, start_widget

    restarts = 0
    while not stop_event.is_set():
        proc = tauri_window.get_tauri_proc()
        if proc is not None and proc.poll() is not None:
            # Tauri 已退出：stop_event 由 /api/stop（Rust 托盘“退出”）触发的属正常退出
            if stop_event.is_set():
                break
            restarts += 1
            if restarts > 3:
                logger.error("[watchdog] tauri exited unexpectedly too many times, giving up")
                break
            logger.warning(
                f"[watchdog] tauri exited unexpectedly (code={proc.returncode}), "
                f"restarting ({restarts}/3)"
            )
            stop_event.wait(2)
            if stop_event.is_set():
                break
            tauri_window.reset_tauri_proc()
            if not tauri_window.start_tauri():
                logger.error("[watchdog] tauri restart failed")
                break
            continue

        # 小组件自愈：settings 启用但进程不在（崩溃/未拉起）时重启
        try:
            if load_settings().get("widgetEnabled", False) \
                    and is_widget_available() and not is_widget_running():
                logger.info("[watchdog] widget not running, restarting")
                start_widget(port, stop_event)
        except Exception as e:
            logger.error(f"[watchdog] widget restart failed: {e}")

        stop_event.wait(5)


def _idle_check(stop_event: threading.Event, settings: dict) -> None:
    """空闲检测线程：检测用户空闲时间，超时后关闭应用"""
    while not stop_event.is_set():
        if settings.get("autoCloseIdle", False):
            try:
                import ctypes
                class LASTINPUTINFO(ctypes.Structure):
                    _fields_ = [("cbSize", ctypes.c_uint), ("dwTime", ctypes.c_uint)]
                lii = LASTINPUTINFO()
                lii.cbSize = ctypes.sizeof(LASTINPUTINFO)
                ctypes.windll.user32.GetLastInputInfo(ctypes.byref(lii))
                idle_ms = ctypes.windll.kernel32.GetTickCount() - lii.dwTime
                idle_min = idle_ms // 60000
                if idle_min >= settings.get("idleCloseMinutes", 5):
                    logger.info("[idle] idle timeout, shutting down")
                    stop_event.set()
                    break
            except Exception:
                pass
        time.sleep(10)


def main() -> None:
    """主入口函数"""
    from server import (
        PORT, LOCK_PORT,
        start_server, shutdown_server, set_stop_event, set_lock_socket,
        auto_close_and_new_session, backup_data,
        load_settings, setup_autostart, is_autostart_registered,
        start_tunnel, stop_tunnel,
        create_tray_icon, start_widget, stop_widget,
        is_tauri_available, start_tauri, stop_tauri,
    )

    logger.info("[startup] === BootTracker starting ===")

    # 实例锁检查
    lock_socket = _ensure_instance_lock(LOCK_PORT)
    if not lock_socket:
        logger.info("[startup] another instance running, raising its window")
        try:
            import urllib.request
            req = urllib.request.Request(
                f"http://127.0.0.1:{PORT}/api/raise-window",
                method="POST", data=b"{}",
                headers={"Content-Type": "application/json"},
            )
            urllib.request.urlopen(req, timeout=2).read()
        except Exception as e:
            logger.info(f"[startup] raise-window failed ({e}), falling back to browser")
            try:
                webbrowser.open(f"http://localhost:{PORT}")
            except Exception:
                pass
        sys.exit(0)
    set_lock_socket(lock_socket)

    # 停止事件
    stop_event = threading.Event()
    set_stop_event(stop_event)

    # 加载设置并配置开机自启
    settings = load_settings()
    if settings.get("autoStart", True):
        if not is_autostart_registered():
            logger.info("[startup] registering autostart")
            setup_autostart(enable=True)

    # 调试模式（--debug 或 BOOTTRACKER_DEBUG=1）：不记录开机会话，避免调试启动污染真实数据
    debug_mode = ("--debug" in sys.argv
                  or os.environ.get("BOOTTRACKER_DEBUG", "").lower() in ("1", "true", "yes"))
    if debug_mode:
        logger.info("[startup] DEBUG mode: boot session will NOT be recorded")
    else:
        # 自动关闭未结束会话并创建新会话
        auto_close_and_new_session()

    # 自动备份
    if settings.get("autoBackup", True):
        try:
            backup_data()
            logger.info("[startup] auto backup done")
        except Exception as e:
            logger.error(f"auto backup failed: {e}")

    # 启动 HTTP 服务器
    bind_addr = "0.0.0.0" if settings.get("lanAccess", True) else "127.0.0.1"
    logger.info(f"[startup] starting server on {bind_addr}:{PORT}")
    start_server(bind_addr=bind_addr, port=PORT)

    # 打开主界面：默认纯原生 egui 桌面端（desktop/），不可用时回退系统浏览器
    # 注：下方变量/函数名仍沿用历史 tauri 命名，实际启动的是 desktop/ 的 egui 程序
    use_tauri = False
    
    if is_tauri_available():
        # 开发模式可通过环境变量控制
        dev_mode = os.environ.get("BOOTTRACKER_DEV_MODE", "").lower() in ("1", "true", "dev")
        use_tauri = start_tauri(dev_mode=dev_mode)
        if use_tauri:
            logger.info("[startup] main UI opened as native desktop window (egui)")
    
    if not use_tauri:
        logger.info("[startup] no native window available, falling back to browser")
        try:
            webbrowser.open(f"http://localhost:{PORT}")
        except Exception:
            pass

    # 启动隧道（如配置）
    if settings.get("tunnelEnabled", False):
        logger.info("[startup] starting tunnel")
        start_tunnel(
            tunnel_token=settings.get("tunnelToken", ""),
            custom_domain=settings.get("customDomain", ""),
        )

    # 托盘：Tauri 模式由 Rust 托盘接管（避免双托盘）；仅浏览器回退模式用 pystray
    icon = None
    if use_tauri:
        logger.info("[startup] rust tray active, skipping legacy pystray tray")
    else:
        icon = create_tray_icon(stop_event)

    # 启动桌面小组件（如启用）
    if settings.get("widgetEnabled", False):
        logger.info("[startup] starting desktop widget")
        threading.Thread(
            target=start_widget, args=(PORT, stop_event), daemon=True
        ).start()

    # 启动空闲检测线程
    idle_thread = threading.Thread(target=_idle_check, args=(stop_event, settings), daemon=True)
    idle_thread.start()

    # 启动看门狗（Tauri 崩溃自动重启 + 小组件自愈）
    threading.Thread(target=_tauri_watchdog, args=(stop_event, PORT), daemon=True).start()

    if icon is not None:
        # pystray 占用主线程（仅浏览器回退模式）
        logger.info("[startup] running pystray loop on main thread")
        icon.run()
        logger.info("[shutdown] tray icon stopped, shutting down")
    else:
        # Tauri 模式：退出由 Rust 托盘经 /api/stop 触发 stop_event
        logger.info("[startup] running main loop (rust tray mode)")
        while not stop_event.is_set():
            stop_event.wait(1)
        logger.info("[shutdown] stop event received, shutting down")
    stop_event.set()

    # 统一关闭流程
    stop_tauri()
    stop_tunnel()
    shutdown_server()
    try:
        stop_widget()
    except Exception:
        pass
    try:
        lock_socket.close()
    except Exception:
        pass
    logger.info("[shutdown] === BootTracker shutdown complete ===")


if __name__ == "__main__":
    logger.info(f"=== boot-tracker start, argv={sys.argv}")
    try:
        main()
        logger.info("main() returned normally")
    except Exception:
        import traceback
        logger.critical(f"FATAL: {traceback.format_exc()}")
        if not sys.executable.endswith("pythonw.exe"):
            traceback.print_exc()
        sys.exit(1)