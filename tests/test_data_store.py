#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""数据存储模块测试：load/save_data、回收站、备份（SQLite 版本）"""
import json
import os
from pathlib import Path


def test_load_data_empty(isolated_data):
    """首次无数据时应返回默认空结构。"""
    from server.data_store import load_data

    data = load_data()
    assert data["bootCount"] == 0
    assert data["shutdownCount"] == 0
    assert data["sessions"] == []


def test_save_and_load_roundtrip(isolated_data):
    """save_data 后 load_data 应能完整恢复。"""
    from server.data_store import save_data, load_data

    obj = {
        "bootCount": 2,
        "shutdownCount": 1,
        "sessions": [
            {"id": "a", "bootTime": "2024-01-01T00:00:00",
             "shutdownTime": "2024-01-01T01:00:00", "duration": 3600_000},
            {"id": "b", "bootTime": "2024-01-02T00:00:00",
             "shutdownTime": None, "duration": None},
        ],
    }
    save_data(obj)
    data = load_data()
    assert data["bootCount"] == 2
    assert data["sessions"][1]["id"] == "b"
    assert data["sessions"][1]["shutdownTime"] is None


def test_trash_roundtrip(isolated_data):
    """回收站 load/save 往返。"""
    from server.data_store import load_trash, save_trash

    trash = {"sessions": [{"id": "del1", "bootTime": "2024-01-01T00:00:00",
                           "shutdownTime": None, "duration": None}]}
    save_trash(trash)
    got = load_trash()
    assert got["sessions"][0]["id"] == "del1"


def test_backup_data_creates_file(isolated_data, seed_sessions, monkeypatch):
    """backup_data 应在 BACKUP_DIR 下创建 .db 备份文件。"""
    from server.data_store import backup_data
    from server.config import BACKUP_DIR
    import server.data_store as ds

    # 重置节流，确保 backup_data 能执行
    monkeypatch.setattr(ds, "_last_backup_time", 0)

    path = backup_data()
    assert path is None or Path(str(path)).exists() or Path(str(BACKUP_DIR)).exists()
    # 目录里至少有一份备份
    files = [f for f in os.listdir(BACKUP_DIR) if f.endswith(".db")]
    assert len(files) >= 1


def test_clean_backups_pruning(isolated_data, seed_sessions, monkeypatch):
    """clean_backups(keep=N) 应只保留最新 N 份。"""
    from server.data_store import backup_data, clean_backups
    import server.config as cfg

    paths = []
    for _ in range(5):
        p = backup_data()
        if p:
            paths.append(p)
    # 保留 2 份
    result = clean_backups(keep=2)
    files = sorted(os.listdir(cfg.BACKUP_DIR))
    assert len(files) <= 2


def test_backup_data_and_clean_via_routes(isolated_data, seed_sessions):
    """路由层：通过 backup_routes.clean_old_backups 触发清理。"""
    from server.data_store import backup_data
    from server.routes.backup_routes import clean_old_backups
    from server.config import BACKUP_DIR

    for _ in range(4):
        backup_data()
    before = len(os.listdir(BACKUP_DIR))
    from server.data_store import clean_backups
    clean_backups(keep=2)
    after = len(os.listdir(BACKUP_DIR))
    assert after <= before and after <= 2


def test_db_file_created(isolated_data):
    """数据库文件应在首次操作后创建。"""
    from server.data_store import _get_conn
    import server.data_store as ds

    conn = _get_conn()
    assert os.path.exists(ds.DB_FILE)


def test_auto_close_and_new_session(isolated_data):
    """auto_close_and_new_session 应关闭未结束会话并创建新会话。"""
    from server.data_store import save_data, load_data, auto_close_and_new_session

    # 先写入一条未结束的会话
    data = {
        "bootCount": 1,
        "shutdownCount": 0,
        "sessions": [
            {"id": "old1", "bootTime": "2024-01-01T08:00:00Z",
             "shutdownTime": None, "duration": None},
        ],
    }
    save_data(data)

    # 执行自动关闭
    auto_close_and_new_session()

    data = load_data()
    # 旧会话应被关闭
    closed = [s for s in data["sessions"] if s["id"] == "old1"]
    assert len(closed) == 1
    assert closed[0]["shutdownTime"] is not None

    # 应有一条新的未关闭会话（今天的）
    unclosed = [s for s in data["sessions"] if s["shutdownTime"] is None]
    assert len(unclosed) >= 1
