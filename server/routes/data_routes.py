#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""数据路由模块 - 开机记录相关 API"""

import uuid
from datetime import datetime
from typing import Tuple, Dict

from ..data_store import load_data, save_data
from ..logging_config import logger
from . import get, post, put, delete


def _calculate_duration(boot_time: str, shutdown_time: str) -> int | None:
    """计算会话持续时间（毫秒）"""
    if not boot_time or not shutdown_time:
        return None
    try:
        boot = datetime.fromisoformat(boot_time.replace("Z", "+00:00"))
        shut = datetime.fromisoformat(shutdown_time.replace("Z", "+00:00"))
        return int((shut - boot).total_seconds() * 1000)
    except Exception:
        return None


def _update_counts(data: Dict) -> None:
    """更新统计计数"""
    data["bootCount"] = len(data["sessions"])
    data["shutdownCount"] = sum(1 for s in data["sessions"] if s.get("shutdownTime"))


@get("/api/data")
def get_data(req, body) -> Tuple[int, Dict]:
    """获取所有开机记录数据"""
    return 200, load_data()


@post("/api/data")
def save_data_handler(req, body) -> Tuple[int, Dict]:
    """保存数据"""
    try:
        save_data(body)
        return 200, {"ok": True}
    except Exception as e:
        logger.error(f"save_data error: {e}")
        return 400, {"error": str(e)}


@post("/api/clear")
def clear_data(req, body) -> Tuple[int, Dict]:
    """清空所有数据"""
    save_data({"bootCount": 0, "shutdownCount": 0, "sessions": []})
    return 200, {"ok": True}


@get("/api/ping")
def ping(req, body) -> Tuple[int, Dict]:
    """健康检查"""
    return 200, {"ok": True}


@post("/api/stop")
def stop_server(req, body) -> Tuple[int, Dict]:
    """停止服务器"""
    from ..http_handler import _stop_event, _server
    if _stop_event:
        _stop_event.set()
    if _server:
        import threading
        threading.Thread(target=_server.shutdown, daemon=True).start()
    return 200, {"ok": True}


@put("/api/sessions/{session_id}")
def update_session(req, body) -> Tuple[int, Dict]:
    """更新会话记录"""
    session_id = body.get("session_id")
    patch = body.get("patch", {})
    
    if not session_id:
        return 400, {"error": "missing session_id"}
    
    data = load_data()
    session = None
    for s in data["sessions"]:
        if s["id"] == session_id:
            session = s
            break
    
    if session is None:
        return 404, {"error": "session not found"}
    
    if "bootTime" in patch:
        session["bootTime"] = patch["bootTime"]
    if "shutdownTime" in patch:
        session["shutdownTime"] = patch["shutdownTime"]
        if patch["shutdownTime"] is None:
            session["duration"] = None
    
    if session["bootTime"] and session["shutdownTime"]:
        session["duration"] = _calculate_duration(session["bootTime"], session["shutdownTime"])
    
    _update_counts(data)
    save_data(data)
    return 200, {"ok": True}


@delete("/api/sessions/{session_id}")
def delete_session(req, body) -> Tuple[int, Dict]:
    """删除会话记录"""
    session_id = body.get("session_id")
    
    if not session_id:
        return 400, {"error": "missing session_id"}
    
    data = load_data()
    new_sessions = [s for s in data["sessions"] if s["id"] != session_id]
    
    if len(new_sessions) == len(data["sessions"]):
        return 404, {"error": "session not found"}
    
    data["sessions"] = new_sessions
    _update_counts(data)
    save_data(data)
    return 200, {"ok": True}


@post("/api/merge-sessions")
def merge_sessions(req, body) -> Tuple[int, Dict]:
    """合并多条会话记录"""
    ids = body.get("ids", [])
    
    if not ids or len(ids) < 2:
        return 400, {"error": "至少选择两条记录"}
    
    data = load_data()
    selected = [s for s in data["sessions"] if s["id"] in ids]
    
    if len(selected) < 2:
        return 400, {"error": "未找到足够的记录"}
    
    boot_times = [s["bootTime"] for s in selected if s.get("bootTime")]
    if not boot_times:
        return 400, {"error": "没有有效的开机时间"}
    merged_boot = min(boot_times)
    
    shutdown_times = [s["shutdownTime"] for s in selected if s.get("shutdownTime")]
    has_active = any(not s.get("shutdownTime") for s in selected)
    merged_shutdown = None if has_active else (max(shutdown_times) if shutdown_times else None)
    
    merged = {
        "id": uuid.uuid4().hex[:14],
        "bootTime": merged_boot,
        "shutdownTime": merged_shutdown,
        "duration": None,
    }
    
    if merged_boot and merged_shutdown:
        merged["duration"] = _calculate_duration(merged_boot, merged_shutdown)
    
    id_set = set(ids)
    data["sessions"] = [s for s in data["sessions"] if s["id"] not in id_set]
    data["sessions"].append(merged)
    data["sessions"].sort(key=lambda s: s.get("bootTime", ""))
    _update_counts(data)
    save_data(data)
    
    from ..data_store import load_trash, save_trash
    trash = load_trash()
    trash.get("sessions", []).extend([s for s in selected if s["id"] in id_set])
    save_trash(trash)
    
    return 200, {"ok": True, "data": data}


@post("/api/add-session")
def add_session(req, body) -> Tuple[int, Dict]:
    """添加会话记录"""
    boot_time = body.get("bootTime")
    shutdown_time = body.get("shutdownTime")
    
    if not boot_time:
        return 400, {"error": "missing bootTime"}
    
    data = load_data()
    session = {
        "id": uuid.uuid4().hex[:14],
        "bootTime": boot_time,
        "shutdownTime": shutdown_time or None,
        "duration": None,
    }
    
    if shutdown_time:
        session["duration"] = _calculate_duration(boot_time, shutdown_time)
    
    data["sessions"].append(session)
    data["sessions"].sort(key=lambda s: s.get("bootTime", ""))
    _update_counts(data)
    save_data(data)
    
    return 200, {"ok": True, "data": data}
