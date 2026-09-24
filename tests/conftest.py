#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""pytest fixtures: 临时隔离数据目录，避免污染真实用户数据"""
import os
import sys
import json
from pathlib import Path

import pytest

_APP_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _APP_DIR not in sys.path:
    sys.path.insert(0, _APP_DIR)


@pytest.fixture()
def isolated_data(tmp_path, monkeypatch):
    """隔离的 DB_FILE / BACKUP_DIR / SETTINGS_FILE。

    使用 monkeypatch 同时修补 config 和所有可能已经缓存了常量的模块，
    确保不会读取真实用户数据。
    """
    import server.config as cfg

    data_dir = tmp_path / "data"
    data_dir.mkdir()

    new_db = str(data_dir / "boot-data.db")
    new_backup = str(data_dir / "backups")
    new_pid = str(data_dir / "boot-tracker.pid")
    new_settings = str(data_dir / "settings.json")

    # 1. 先改 config
    monkeypatch.setattr(cfg, "DB_FILE", new_db)
    monkeypatch.setattr(cfg, "BACKUP_DIR", new_backup)
    monkeypatch.setattr(cfg, "PID_FILE", new_pid)
    monkeypatch.setattr(cfg, "SETTINGS_FILE", new_settings)

    os.makedirs(cfg.BACKUP_DIR, exist_ok=True)

    # 2. 如果 data_store / settings / routes 已经导入，替换它们内部引用
    import importlib
    import server.data_store as ds
    import server.settings as st

    # 关闭旧连接
    if hasattr(ds._local, "conn"):
        try:
            ds._local.conn.close()
        except Exception:
            pass
        ds._local.conn = None

    ds.DB_FILE = new_db
    ds.BACKUP_DIR = new_backup
    st.SETTINGS_FILE = new_settings

    # 重新初始化数据库
    ds._init_db()

    # 所有 from data_store import 的模块都需要同步重写
    for _mod_name in ("server.routes.backup_routes",):
        try:
            _mod = sys.modules.get(_mod_name) or importlib.import_module(_mod_name)
            if hasattr(_mod, "DB_FILE"):
                _mod.DB_FILE = new_db
            if hasattr(_mod, "BACKUP_DIR"):
                _mod.BACKUP_DIR = new_backup
        except Exception:
            pass

    yield tmp_path

    # 清理
    if hasattr(ds._local, "conn"):
        try:
            ds._local.conn.close()
        except Exception:
            pass
        ds._local.conn = None


@pytest.fixture()
def seed_sessions(isolated_data):
    """写入 3 条会话：2 条已结束、1 条进行中。返回 data 字典。"""
    from server.data_store import save_data

    data = {
        "bootCount": 3,
        "shutdownCount": 2,
        "sessions": [
            {
                "id": "s1",
                "bootTime": "2024-01-01T08:00:00",
                "shutdownTime": "2024-01-01T12:00:00",
                "duration": 4 * 3600 * 1000,
            },
            {
                "id": "s2",
                "bootTime": "2024-01-02T09:00:00",
                "shutdownTime": "2024-01-02T17:00:00",
                "duration": 8 * 3600 * 1000,
            },
            {
                "id": "s3",
                "bootTime": "2024-01-03T08:30:00",
                "shutdownTime": None,
                "duration": None,
            },
        ],
    }
    save_data(data)
    return data


@pytest.fixture()
def default_settings(isolated_data):
    """写入默认 settings 并返回字典。"""
    from server.settings import save_settings, _default_settings

    defaults = _default_settings()
    save_settings(defaults)
    return dict(defaults)
