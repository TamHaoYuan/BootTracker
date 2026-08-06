#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 - 桌面应用入口

应用启动流程：
1. 实例锁检查（防止多开）
2. 自动关闭未结束会话并创建新会话
3. 自动备份数据
4. 启动 HTTP 服务器
5. 打开浏览器
6. 启动隧道（如配置）
7. 创建系统托盘图标
8. 空闲检测线程（如配置）
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
    """创建实例锁（通过端口绑定实现）"""
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.bind(("127.0.0.1", lock_port))
        sock.listen(1)
        return sock
    except OSError:
        return None


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
        create_tray_icon,
    )

    logger.info("[startup] === BootTracker starting ===")

    # 实例锁检查
    lock_socket = _ensure_instance_lock(LOCK_PORT)
    if not lock_socket:
        logger.info("[startup] another instance already running, opening browser")
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

    # 打开浏览器
    logger.info("[startup] server started, opening browser")
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

    # 创建系统托盘图标
    icon = create_tray_icon(stop_event)

    # 启动空闲检测线程
    idle_thread = threading.Thread(target=_idle_check, args=(stop_event, settings), daemon=True)
    idle_thread.start()

    # 运行托盘图标
    icon.run()

    # 关闭流程
    logger.info("[shutdown] tray icon stopped, shutting down")
    stop_event.set()
    stop_tunnel()
    shutdown_server()
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