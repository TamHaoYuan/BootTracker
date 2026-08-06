#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""版本路由模块 - 版本管理相关 API"""

from typing import Tuple, Dict

from ..version import load_version, save_version, bump_version, check_update, UPDATE_CHECK_URL
from ..logging_config import logger
from . import get, post


@get("/api/version")
def get_version(req, body) -> Tuple[int, Dict]:
    """获取当前版本"""
    ver_data = load_version()
    return 200, {
        "version": ver_data["version"],
        "updateUrl": UPDATE_CHECK_URL,
    }


@get("/api/version/history")
def get_version_history(req, body) -> Tuple[int, Dict]:
    """获取版本历史"""
    ver_data = load_version()
    return 200, {"history": ver_data.get("history", [])}


@post("/api/version/bump")
def bump_version_handler(req, body) -> Tuple[int, Dict]:
    """升级版本"""
    bump_type = body.get("type", "patch")
    notes = body.get("notes", "")
    
    if bump_type not in ("major", "minor", "patch"):
        return 400, {"error": "type must be major/minor/patch"}
    
    try:
        result = bump_version(bump_type, notes)
        return 200, {
            "ok": True,
            "version": result["version"],
            "previous": result["history"][0]["from"] if result["history"] else "",
        }
    except Exception as e:
        logger.error(f"bump_version error: {e}")
        return 400, {"error": str(e)}


@post("/api/check-update")
def check_update_handler(req, body) -> Tuple[int, Dict]:
    """检查更新"""
    return 200, check_update()
