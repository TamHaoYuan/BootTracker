#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""设置路由模块 - 设置相关 API"""

from typing import Tuple, Dict

from ..settings import load_settings, save_settings, setup_autostart, is_autostart_registered
from ..data_store import MAX_BACKUPS
from ..config import PORT
from ..tunnel import start_tunnel, stop_tunnel, tunnel_state
from ..widget import start_widget, stop_widget, is_widget_running
from ..logging_config import logger
from . import get, post, put


_VALID_KEYS = {
    "autoStart", "autoBackup", "backupCount", "autoCloseIdle",
    "idleCloseMinutes", "defaultChartType", "timeFormat",
    "lanAccess", "tunnelEnabled", "tunnelToken", "customDomain", "customBgImage",
    "widgetEnabled", "appTheme", "widgetPosition", "uiScale", "language",
}


def _validate_and_convert(key: str, value) -> Tuple[bool, any]:
    """验证并转换设置值"""
    if key == "backupCount":
        try:
            v = int(value)
            if v < 1 or v > 100:
                return False, "backupCount must be 1-100"
            return True, v
        except ValueError:
            return False, "backupCount must be integer"
    elif key == "idleCloseMinutes":
        try:
            v = int(value)
            if v < 1 or v > 1440:
                return False, "idleCloseMinutes must be 1-1440"
            return True, v
        except ValueError:
            return False, "idleCloseMinutes must be integer"
    elif key in ("autoStart", "autoBackup", "autoCloseIdle", "lanAccess", "tunnelEnabled", "widgetEnabled"):
        return True, bool(value)
    elif key == "uiScale":
        try:
            v = float(value)
        except (TypeError, ValueError):
            return False, "uiScale must be a number"
        if v < 0.5 or v > 2.0:
            return False, "uiScale must be 0.5-2.0"
        # 保留两位小数，避免前端滚轮缩放写入一长串浮点尾数
        return True, round(v, 2)
    elif key == "defaultChartType":
        if value not in ("bar", "line"):
            return False, "defaultChartType must be bar or line"
        return True, value
    elif key == "timeFormat":
        if value not in ("24h", "12h"):
            return False, "timeFormat must be 24h or 12h"
        return True, value
    elif key in ("tunnelToken", "customDomain"):
        return True, str(value).strip()
    elif key == "language":
        if value not in ("", "zh-CN", "en-US"):
            return False, "language must be zh-CN, en-US or empty"
        return True, value
    return True, value


@get("/api/settings")
def get_settings(req, body) -> Tuple[int, Dict]:
    """获取设置"""
    try:
        settings = load_settings()
        settings["_autoStartRegistered"] = is_autostart_registered()
        return 200, settings
    except Exception as e:
        logger.error(f"get_settings error: {e}")
        return 400, {"error": str(e)}


@put("/api/settings")
def update_settings(req, body) -> Tuple[int, Dict]:
    """更新设置"""
    updates = {}
    
    for key, value in body.items():
        if key not in _VALID_KEYS:
            continue
        
        valid, converted = _validate_and_convert(key, value)
        if not valid:
            return 400, {"error": converted}
        updates[key] = converted
    
    if not updates:
        # 无有效更新（例如提交值与现有一致），幂等返回成功，前端不需要将其视为错误
        return 200, {"ok": True, "updated": []}
    
    if "autoStart" in updates:
        setup_autostart(enable=updates["autoStart"])
    
    if "backupCount" in updates:
        global MAX_BACKUPS
        MAX_BACKUPS = updates["backupCount"]
    
    if "lanAccess" in updates:
        logger.info(f"[settings] lanAccess changed to {updates['lanAccess']}, will apply on restart")
    
    settings = load_settings()

    old_token = settings.get("tunnelToken", "").strip()
    old_domain = settings.get("customDomain", "").strip()
    tunnel_was_running = tunnel_state()["running"]
    widget_was_enabled = bool(settings.get("widgetEnabled", False))

    settings.update(updates)
    save_settings(settings)

    if "tunnelEnabled" in updates:
        if updates["tunnelEnabled"]:
            start_tunnel(
                tunnel_token=settings.get("tunnelToken", ""),
                custom_domain=settings.get("customDomain", ""),
            )
        else:
            stop_tunnel()
    elif tunnel_was_running and ("tunnelToken" in updates or "customDomain" in updates):
        new_token = updates.get("tunnelToken", old_token).strip()
        new_domain = updates.get("customDomain", old_domain).strip()
        if new_token != old_token or new_domain != old_domain:
            logger.info("[settings] tunnel config changed, restarting tunnel")
            stop_tunnel()
            start_tunnel(tunnel_token=new_token, custom_domain=new_domain)

    # 桌面组件开关：实时启停，无需重启
    if "widgetEnabled" in updates:
        want_on = bool(updates["widgetEnabled"])
        widget_running = is_widget_running()
        if want_on and not widget_running:
            import threading
            logger.info("[settings] enabling desktop widget")
            threading.Thread(
                target=start_widget, args=(PORT, threading.Event()), daemon=True
            ).start()
        elif not want_on and (widget_running or widget_was_enabled):
            logger.info("[settings] disabling desktop widget")
            stop_widget()

    return 200, {"ok": True, "updated": list(updates.keys())}


@post("/api/widget-toggle")
def toggle_widget(req, body) -> Tuple[int, Dict]:
    """翻转桌面小组件开关（实时启停），供 Rust 托盘菜单调用

    返回新状态，调用方据此同步菜单勾选。
    """
    import threading
    try:
        settings = load_settings()
        want_on = not bool(settings.get("widgetEnabled", False))
        settings["widgetEnabled"] = want_on
        save_settings(settings)

        if want_on and not is_widget_running():
            logger.info("[settings] widget toggled on")
            threading.Thread(
                target=start_widget, args=(PORT, threading.Event()), daemon=True
            ).start()
        elif not want_on:
            logger.info("[settings] widget toggled off")
            stop_widget()

        return 200, {"ok": True, "widgetEnabled": want_on}
    except Exception as e:
        logger.error(f"toggle_widget error: {e}")
        return 400, {"error": str(e)}


@post("/api/widget-sync")
def sync_widget(req, body) -> Tuple[int, Dict]:
    """立即把最新设置/数据同步到桌面小组件。

    小组件是独立进程，靠轮询拉取（30s 数据 / 60s 设置），改完设置不会立刻生效。
    这里重启浮窗进程——启动时 WidgetApp::new() 会立即拉一次 settings 与 data，
    从而跳过最长 60 秒的等待。未启用小组件时直接跳过。
    """
    import threading
    try:
        settings = load_settings()
        if not bool(settings.get("widgetEnabled", False)):
            return 200, {"ok": True, "synced": False, "enabled": False}

        def _do_sync():
            try:
                if is_widget_running():
                    stop_widget()
                start_widget(PORT, threading.Event())
                logger.info("[settings] widget synced (restarted)")
            except Exception as e:
                logger.error(f"[settings] widget sync failed: {e}")

        # 重启可能阻塞（停止有 5s 等待），放后台执行，路由立即返回
        threading.Thread(target=_do_sync, daemon=True).start()
        return 200, {"ok": True, "synced": True, "enabled": True}
    except Exception as e:
        logger.error(f"sync_widget error: {e}")
        return 400, {"error": str(e)}
