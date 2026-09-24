#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 - 后端服务包

核心模块说明：
- config: 配置常量（端口、路径等）
- data_store: 数据存储（开机记录、回收站、备份）
- http_handler: HTTP 服务（API 路由、静态文件）
- settings: 设置管理（用户配置、开机自启）
- version: 版本管理（版本号、更新检查）
- tunnel: 隧道管理（Cloudflare Tunnel）
- tray: 系统托盘（图标、菜单）
- widget: 桌面小组件（Rust/Tauri 组件进程管理）
- tauri_window: 桌面窗口（Tauri v2，新版本默认）
"""

from .config import (
    PORT, LOCK_PORT, APP_DIR, HTML_FILE, DB_FILE,
    SETTINGS_FILE, VERSION_FILE, PID_FILE, LOG_DIR, LOG_FILE, BACKUP_DIR,
    UPLOAD_DIR, TUNNEL_LOG_FILE, CLOUDFLARED_PATH, MAX_BACKUPS, APP_VERSION, UPDATE_CHECK_URL,
)
from .http_handler import (
    start_server, shutdown_server, set_stop_event, set_lock_socket,
)
from .data_store import (
    load_data, save_data, load_trash, save_trash, backup_data, clean_backups,
    auto_close_and_new_session, is_already_running, write_pid_file, remove_pid_file,
)
from .settings import (
    load_settings, save_settings, setup_autostart, is_autostart_registered,
)
from .version import (
    load_version, save_version, bump_version, check_update,
)
from .tunnel import (
    start_tunnel, stop_tunnel, get_tunnel_url,
)
from .tray import create_tray_icon
from .widget import start_widget, stop_widget
from .tauri_window import (
    is_tauri_available, start_tauri, stop_tauri,
    raise_tauri_window, set_tauri_theme,
)
from .logging_config import logger, get_logger, setup_logging
from .routes import registry, get, post, put, delete, route

__all__ = [
    # 配置常量
    "PORT", "LOCK_PORT", "APP_DIR", "HTML_FILE", "DB_FILE",
    "SETTINGS_FILE", "VERSION_FILE", "PID_FILE", "LOG_DIR", "LOG_FILE", "BACKUP_DIR",
    "UPLOAD_DIR", "TUNNEL_LOG_FILE", "CLOUDFLARED_PATH", "MAX_BACKUPS",
    "APP_VERSION", "UPDATE_CHECK_URL",
    
    # HTTP 服务
    "start_server", "shutdown_server", "set_stop_event", "set_lock_socket",
    
    # 数据存储
    "load_data", "save_data", "load_trash", "save_trash", "backup_data",
    "clean_backups", "auto_close_and_new_session", "is_already_running",
    "write_pid_file", "remove_pid_file",
    
    # 设置管理
    "load_settings", "save_settings", "setup_autostart", "is_autostart_registered",
    
    # 版本管理
    "load_version", "save_version", "bump_version", "check_update",
    
    # 隧道管理
    "start_tunnel", "stop_tunnel", "get_tunnel_url",
    
    # UI
    "create_tray_icon", "start_widget", "stop_widget",
    # Tauri 窗口（新版本默认）
    "is_tauri_available", "start_tauri", "stop_tauri",
    "raise_tauri_window", "set_tauri_theme",
    
    # 日志
    "logger", "get_logger", "setup_logging",
    
    # 路由
    "registry", "get", "post", "put", "delete", "route",
]