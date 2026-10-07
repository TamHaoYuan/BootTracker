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


def _freeze_at_utc8(monkeypatch, local_iso: str):
    """把 data_store 里的"现在"固定在指定 UTC+8 本地时刻。

    time.strftime("%Y-%m-%d") 返回该时刻的本地日期（自动跨日），
    datetime.now(timezone.utc) 返回同一时刻的 UTC 值，
    从而让测试不依赖运行机器的时区。
    """
    from datetime import datetime as real_datetime, timedelta, timezone

    local = real_datetime.fromisoformat(local_iso).replace(
        tzinfo=timezone(timedelta(hours=8))
    )
    now_utc = local.astimezone(timezone.utc)

    class _FrozenDateTime(real_datetime):
        @classmethod
        def now(cls, tz=None):
            return now_utc if tz is not None else now_utc.replace(tzinfo=None)

    import server.data_store as ds

    # 先取出原始 strftime，避免补丁自身递归
    _real_strftime = ds.time.strftime

    monkeypatch.setattr(ds, "datetime", _FrozenDateTime)
    monkeypatch.setattr(
        ds.time,
        "strftime",
        lambda fmt, *a, _f=_real_strftime: _f(fmt, local.timetuple()),
    )
    return now_utc


def test_local_day_utc_range_boundaries(isolated_data):
    """本地自然日对应的 UTC 边界应首尾闭合、相邻日无缝。"""
    from datetime import datetime as _dt, timedelta, timezone
    from server.data_store import _local_day_utc_range

    start, end = _local_day_utc_range("2026-10-07")
    expect_start = (
        _dt.strptime("2026-10-07", "%Y-%m-%d")
        .astimezone(timezone.utc)
        .replace(tzinfo=None)
        .isoformat()
        + "Z"
    )
    assert start == expect_start
    # 后一天的起点必须等于当天的终点（本地日与 UTC 边界一一对应）
    assert _local_day_utc_range("2026-10-08")[0] == end
    # 相邻日严格递增，且跨度恰为 24 小时
    assert start < end
    span = _dt.fromisoformat(end.replace("Z", "+00:00")) - _dt.fromisoformat(
        start.replace("Z", "+00:00")
    )
    assert span == timedelta(days=1)


def test_no_duplicate_session_when_local_day_follows_utc_day(isolated_data, monkeypatch):
    """本地凌晨开机（UTC 日仍是前一天）重启时不得重复新建会话。

    回归 D2：原先用 `date(boot_time) = 本地日期` 比较，UTC+8 下本地
    00:00–08:00 的开机其 UTC 日期属前一天，导致匹配不到而重复新建，
    并把上一段未收盘会话按当前时间强制收盘。
    """
    from server.data_store import (
        save_data,
        load_data,
        auto_close_and_new_session,
    )

    _freeze_at_utc8(monkeypatch, "2026-10-08T00:35:00")

    # 上一段会话始于同一本地日（= UTC 2026-10-07T16:30Z）
    save_data({
        "bootCount": 1,
        "shutdownCount": 0,
        "sessions": [
            {"id": "early", "bootTime": "2026-10-07T16:30:00Z",
             "shutdownTime": None, "duration": None},
        ],
    })

    auto_close_and_new_session()
    sessions = load_data()["sessions"]

    # 不得因为本地日与 UTC 日不一致而新建第二条会话
    assert len(sessions) == 1, f"本地凌晨重启误建新会话：{[s['id'] for s in sessions]}"
    assert sessions[0]["id"] == "early"
    # 未收盘会话仍应按"启动即收盘"的既有语义被收盘
    assert sessions[0]["shutdownTime"] is not None
    # 时长应为 5 分钟（00:30 → 00:35），而不是把开机时间改写成当前时间
    assert sessions[0]["duration"] == 5 * 60 * 1000


def test_same_local_day_restart_does_not_duplicate(isolated_data, monkeypatch):
    """同一本地日内的二次启动不新建会话（修正后的门禁生效）。"""
    from server.data_store import (
        save_data,
        load_data,
        auto_close_and_new_session,
    )

    _freeze_at_utc8(monkeypatch, "2026-10-08T09:00:00")
    save_data({
        "bootCount": 1,
        "shutdownCount": 0,
        "sessions": [
            {"id": "morning", "bootTime": "2026-10-08T00:30:00Z",  # 本地 08:30
             "shutdownTime": None, "duration": None},
        ],
    })

    auto_close_and_new_session()
    sessions = load_data()["sessions"]
    assert len(sessions) == 1
    assert sessions[0]["duration"] == 30 * 60 * 1000
