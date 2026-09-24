#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""备份路由模块 - 备份相关 API（SQLite 版本）"""

import os
import sqlite3
import shutil
from typing import Tuple, Dict

from ..data_store import backup_data, clean_backups
from ..config import DB_FILE, BACKUP_DIR
from ..logging_config import logger
from . import get, post, delete


def _ensure_backup_dir() -> None:
    """确保备份目录存在"""
    os.makedirs(BACKUP_DIR, exist_ok=True)


@get("/api/backups")
def list_backups(req, body) -> Tuple[int, Dict]:
    """列出所有备份文件"""
    try:
        _ensure_backup_dir()
        files = [
            f for f in os.listdir(BACKUP_DIR)
            if f.startswith("boot-data-") and f.endswith(".db")
        ]
        files.sort(reverse=True)
        return 200, {"backups": files}
    except Exception:
        return 200, {"backups": []}


@post("/api/backup/restore")
def restore_backup(req, body) -> Tuple[int, Dict]:
    """恢复备份"""
    filename = body.get("filename")
    
    if not filename:
        return 400, {"error": "missing filename"}
    
    safe = os.path.basename(filename)
    if safe != filename or not safe.startswith("boot-data-") or not safe.endswith(".db"):
        return 400, {"error": "invalid filename"}
    
    backup_path = os.path.join(BACKUP_DIR, safe)
    if not os.path.isfile(backup_path):
        return 404, {"error": "backup file not found"}
    
    try:
        # 先备份当前数据库
        backup_data()
        # 使用 SQLite backup API 恢复
        from ..data_store import _get_conn, _local
        src_conn = sqlite3.connect(backup_path)
        dst_conn = _get_conn()
        src_conn.backup(dst_conn)
        src_conn.close()
        dst_conn.commit()
        return 200, {"ok": True}
    except Exception as e:
        logger.error(f"restore_backup error: {e}")
        return 400, {"error": str(e)}


@delete("/api/delete-backup")
def delete_backup(req, body) -> Tuple[int, Dict]:
    """删除备份文件"""
    filename = body.get("filename")
    
    if not filename:
        return 400, {"error": "missing filename"}
    
    safe = os.path.basename(filename)
    if safe != filename or not safe.startswith("boot-data-") or not safe.endswith(".db"):
        return 400, {"error": "invalid filename"}
    
    backup_path = os.path.join(BACKUP_DIR, safe)
    if not os.path.isfile(backup_path):
        return 404, {"error": "file not found"}
    
    try:
        os.remove(backup_path)
        return 200, {"ok": True}
    except Exception as e:
        logger.error(f"delete_backup error: {e}")
        return 400, {"error": str(e)}


@post("/api/clean-backups")
def clean_old_backups(req, body) -> Tuple[int, Dict]:
    """清理旧备份"""
    try:
        keep = int(body.get("keep", 10))
        keep = max(1, min(keep, 100))
        deleted, remaining = clean_backups(keep=keep)
        return 200, {"ok": True, "deleted": deleted, "remaining": remaining}
    except Exception as e:
        logger.error(f"clean_backups error: {e}")
        return 400, {"error": str(e)}
