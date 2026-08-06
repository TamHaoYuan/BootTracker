#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""设置路由模块 - 设置相关 API"""

from typing import Tuple, Dict

from ..settings import load_settings, save_settings, setup_autostart, is_autostart_registered
from ..data_store import MAX_BACKUPS
from ..tunnel import start_tunnel, stop_tunnel, _tunnel_proc
from ..logging_config import logger
from . import get, put


_VALID_KEYS = {
    "autoStart", "autoBackup", "backupCount", "autoCloseIdle", 
    "idleCloseMinutes", "defaultChartType", "timeFormat", 
    "lanAccess", "tunnelEnabled", "tunnelToken", "customDomain", "customBgImage"
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
    elif key in ("autoStart", "autoBackup", "autoCloseIdle", "lanAccess", "tunnelEnabled"):
        return True, bool(value)
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
        return 400, {"error": "no valid fields"}
    
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
    tunnel_was_running = _tunnel_proc is not None
    
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
    
    return 200, {"ok": True, "updated": list(updates.keys())}
