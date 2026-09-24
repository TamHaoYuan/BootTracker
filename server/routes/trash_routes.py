#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""回收站路由模块 - 回收站相关 API"""

from typing import Tuple, Dict

from ..data_store import load_trash, save_trash, load_data, save_data
from ..logging_config import logger
from . import get, post


def _update_counts(data: Dict) -> None:
    """更新统计计数"""
    data["bootCount"] = len(data["sessions"])
    data["shutdownCount"] = sum(1 for s in data["sessions"] if s.get("shutdownTime"))


@get("/api/trash")
def get_trash(req, body) -> Tuple[int, Dict]:
    """获取回收站数据"""
    return 200, load_trash()


@post("/api/trash")
def save_trash_handler(req, body) -> Tuple[int, Dict]:
    """保存回收站数据"""
    try:
        save_trash(body)
        return 200, {"ok": True}
    except Exception as e:
        logger.error(f"save_trash error: {e}")
        return 400, {"error": str(e)}


@post("/api/trash/restore")
def restore_from_trash(req, body) -> Tuple[int, Dict]:
    """从回收站恢复记录"""
    session_id = body.get("id")
    
    if not session_id:
        return 400, {"error": "missing id"}
    
    trash = load_trash()
    sessions = trash.get("sessions", [])
    idx = next((i for i, s in enumerate(sessions) if s.get("id") == session_id), None)
    
    if idx is None:
        return 404, {"error": "not found"}
    
    session = sessions.pop(idx)
    
    data = load_data()
    data["sessions"].append(session)
    data["sessions"].sort(key=lambda s: s.get("bootTime", ""))
    _update_counts(data)
    
    save_trash(trash)
    save_data(data)
    
    return 200, {"ok": True}


@post("/api/trash/clear")
def clear_trash(req, body) -> Tuple[int, Dict]:
    """清空回收站"""
    save_trash({"sessions": []})
    return 200, {"ok": True}


@post("/api/trash/delete")
def delete_from_trash(req, body) -> Tuple[int, Dict]:
    """从回收站永久删除一条记录（不进入 boot-data，直接清除）"""
    session_id = body.get("id")

    if not session_id:
        return 400, {"error": "missing id"}

    trash = load_trash()
    sessions = trash.get("sessions", [])
    new_sessions = [s for s in sessions if s.get("id") != session_id]

    if len(new_sessions) == len(sessions):
        return 404, {"error": "not found"}

    trash["sessions"] = new_sessions
    save_trash(trash)

    return 200, {"ok": True}
