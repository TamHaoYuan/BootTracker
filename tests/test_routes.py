#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""HTTP 路由测试：直接用 (req, body) 签名调用路由，不启动服务器."""
import os


# --------- data_routes ---------
def test_ping_ok(isolated_data):
    from server.routes.data_routes import ping
    status, body = ping(None, None)
    assert status == 200
    assert body["ok"] is True


def test_get_data_default(isolated_data):
    from server.routes.data_routes import get_data
    status, body = get_data(None, None)
    assert status == 200
    assert body["bootCount"] == 0
    assert body["sessions"] == []


def test_save_and_clear_data(isolated_data):
    from server.routes.data_routes import save_data_handler, clear_data, get_data

    status, body = save_data_handler(None, {"bootCount": 1, "shutdownCount": 1,
                                            "sessions": [{"id": "x", "bootTime": "2024-01-01T00:00:00",
                                                          "shutdownTime": "2024-01-01T01:00:00",
                                                          "duration": 3600_000}]})
    assert status == 200
    assert body["ok"] is True

    _, body = get_data(None, None)
    assert body["bootCount"] == 1

    status, body = clear_data(None, None)
    assert status == 200
    _, body = get_data(None, None)
    assert body["sessions"] == []


def test_add_session_ok(isolated_data):
    from server.routes.data_routes import add_session

    status, body = add_session(None, {"bootTime": "2024-05-01T10:00:00",
                                      "shutdownTime": "2024-05-01T12:00:00"})
    assert status == 200
    assert body["ok"] is True
    assert body["data"]["bootCount"] == 1
    assert body["data"]["sessions"][0]["duration"] == 2 * 3600 * 1000


def test_add_session_missing_boot(isolated_data):
    from server.routes.data_routes import add_session
    status, body = add_session(None, {})
    assert status == 400
    assert "missing bootTime" in body["error"]


def test_update_session_ok(isolated_data, seed_sessions):
    from server.routes.data_routes import update_session

    status, body = update_session(None, {"session_id": "s3",
                                         "patch": {"shutdownTime": "2024-01-03T17:00:00"}})
    assert status == 200
    assert body["ok"] is True
    from server.data_store import load_data
    s3 = next(s for s in load_data()["sessions"] if s["id"] == "s3")
    assert s3["shutdownTime"] is not None
    assert s3["duration"] > 0


def test_update_session_missing_id(isolated_data):
    from server.routes.data_routes import update_session
    status, _ = update_session(None, {})
    assert status == 400


def test_update_session_not_found(isolated_data, seed_sessions):
    from server.routes.data_routes import update_session
    status, _ = update_session(None, {"session_id": "none"})
    assert status == 404


def test_delete_session_ok(isolated_data, seed_sessions):
    from server.routes.data_routes import delete_session
    status, body = delete_session(None, {"session_id": "s1"})
    assert status == 200
    from server.data_store import load_data
    assert not any(s["id"] == "s1" for s in load_data()["sessions"])


def test_delete_session_not_found(isolated_data, seed_sessions):
    from server.routes.data_routes import delete_session
    status, _ = delete_session(None, {"session_id": "nope"})
    assert status == 404


def test_merge_sessions_ok(isolated_data, seed_sessions):
    from server.routes.data_routes import merge_sessions
    status, body = merge_sessions(None, {"ids": ["s1", "s2"]})
    assert status == 200
    sessions = body["data"]["sessions"]
    # s1/s2 被合并，总数 = 1(merged) + 1(s3) = 2
    assert len(sessions) == 2
    merged = [s for s in sessions if s["id"] not in {"s3"}][0]
    assert merged["bootTime"] == "2024-01-01T08:00:00"
    assert merged["shutdownTime"] == "2024-01-02T17:00:00"
    assert merged["duration"] > 0
    # 旧会话进回收站
    from server.data_store import load_trash
    assert any(s["id"] == "s1" for s in load_trash().get("sessions", []))


def test_merge_sessions_too_few(isolated_data, seed_sessions):
    from server.routes.data_routes import merge_sessions
    status, _ = merge_sessions(None, {"ids": ["s1"]})
    assert status == 400


# --------- settings_routes ---------
def test_settings_default(isolated_data, default_settings):
    from server.routes.settings_routes import get_settings
    status, body = get_settings(None, None)
    assert status == 200
    # appMode 目前不在 settings_routes 校验键中，仍能从文件中读取
    assert body.get("appMode") in ("dark", "light")
    assert "backupCount" in body


def test_settings_update_and_persist(isolated_data, default_settings):
    from server.routes.settings_routes import update_settings, get_settings

    # backupCount 在 _VALID_KEYS 中；appMode 不在，因此只会更新 backupCount
    status, body = update_settings(None, {"backupCount": 7})
    assert status == 200
    assert body["ok"] is True
    assert "backupCount" in body.get("updated", [])

    _, got = get_settings(None, None)
    assert got["backupCount"] == 7


def test_settings_update_no_changes(isolated_data, default_settings):
    """无实际更新（值一致且没有跳过键）时应 200 + updated=[] 而非报错。"""
    from server.routes.settings_routes import update_settings
    # 传不在 _VALID_KEYS 的键会被跳过，返回 updated=[]
    status, body = update_settings(None, {"doesNotExist": True})
    assert status == 200
    assert body["ok"] is True
    assert body["updated"] == []


def test_settings_update_invalid_enum(isolated_data, default_settings):
    """非法枚举值（defaultChartType/timeFormat）应返回 400。"""
    from server.routes.settings_routes import update_settings
    status, body = update_settings(None, {"defaultChartType": "pie"})
    assert status == 400
    assert "defaultChartType" in body["error"]


def test_settings_update_invalid_backup_count(isolated_data, default_settings):
    from server.routes.settings_routes import update_settings
    # 101 超过上限 100
    status, body = update_settings(None, {"backupCount": 101})
    assert status == 400
    assert "backupCount" in body["error"]


def test_widget_toggle(isolated_data, default_settings, monkeypatch):
    """POST /api/widget-toggle 应翻转开关并持久化，启停组件进程被桩替换。"""
    import time
    from server.routes import settings_routes
    from server.settings import load_settings

    started, stopped = [], []
    monkeypatch.setattr(settings_routes, "start_widget",
                        lambda port, ev: started.append(port))
    monkeypatch.setattr(settings_routes, "stop_widget",
                        lambda: stopped.append(True))
    monkeypatch.setattr(settings_routes, "is_widget_running", lambda: False)

    # off -> on：应触发 start_widget（后台线程）并持久化 widgetEnabled=true
    status, body = settings_routes.toggle_widget(None, {})
    assert status == 200
    assert body["widgetEnabled"] is True
    assert load_settings()["widgetEnabled"] is True
    for _ in range(50):
        if started:
            break
        time.sleep(0.02)
    assert started, "start_widget should be called when toggling on"

    # on -> off：应触发 stop_widget 并持久化 widgetEnabled=false
    status, body = settings_routes.toggle_widget(None, {})
    assert status == 200
    assert body["widgetEnabled"] is False
    assert load_settings()["widgetEnabled"] is False
    assert stopped


# --------- trash_routes ---------
def test_trash_move_get_restore(isolated_data, seed_sessions, default_settings):
    from server.routes.trash_routes import save_trash_handler, get_trash, restore_from_trash

    # 1. 构造写入：save_trash_handler（body 格式必须带 sessions 列表项 + deletedAt）
    trash_body = {
        "sessions": [dict(s, deletedAt="2024-06-01T00:00:00") for s in seed_sessions["sessions"] if s["id"] == "s1"]
    }
    status, body = save_trash_handler(None, trash_body)
    assert status == 200
    assert body["ok"] is True

    # 2. 读出
    _, trash = get_trash(None, None)
    ids = [s["id"] for s in trash["sessions"]]
    assert "s1" in ids

    # 3. 恢复：restore_from_trash 用 {"id": "s1"}
    status, body = restore_from_trash(None, {"id": "s1"})
    assert status == 200
    _, trash = get_trash(None, None)
    assert "s1" not in [s["id"] for s in trash["sessions"]]
    from server.data_store import load_data
    assert any(s["id"] == "s1" for s in load_data()["sessions"])


def test_trash_clear(isolated_data, seed_sessions, default_settings):
    from server.routes.trash_routes import save_trash_handler, clear_trash, get_trash
    trash_body = {
        "sessions": [
            {"id": "s1", "bootTime": "2024-01-01T08:00:00", "shutdownTime": "2024-01-01T12:00:00", "deletedAt": "2024-01-04"},
            {"id": "s2", "bootTime": "2024-01-02T09:00:00", "shutdownTime": "2024-01-02T17:00:00", "deletedAt": "2024-01-04"},
        ]
    }
    save_trash_handler(None, trash_body)
    _, before = get_trash(None, None)
    assert len(before["sessions"]) == 2

    status, body = clear_trash(None, None)
    assert status == 200
    _, after = get_trash(None, None)
    assert after["sessions"] == []


def test_trash_permanent_delete(isolated_data, seed_sessions, default_settings):
    from server.routes.trash_routes import save_trash_handler, get_trash, delete_from_trash
    save_trash_handler(None, {"sessions": [{"id": "s1", "bootTime": "2024-01-01T08:00:00", "deletedAt": "2024-01-04"}]})
    _, before = get_trash(None, None)
    assert len(before["sessions"]) == 1

    # delete_from_trash 用 {"id": "s1"}
    status, body = delete_from_trash(None, {"id": "s1"})
    assert status == 200
    _, after = get_trash(None, None)
    assert after["sessions"] == []


# --------- backup_routes ---------
def _pin_backup_paths(backup_dir: str, db_file: str):
    """将所有相关模块的 BACKUP_DIR/DB_FILE 全部锁定到指定路径。"""
    import os
    os.makedirs(backup_dir, exist_ok=True)
    # 批量同步所有导入过的模块引用
    for _attr in ("BACKUP_DIR", "DB_FILE"):
        _val = backup_dir if _attr == "BACKUP_DIR" else db_file
        for _mod_name in (
            "server.data_store",
            "server.routes.backup_routes",
        ):
            import sys as _sys
            import importlib
            _mod = _sys.modules.get(_mod_name) or importlib.import_module(_mod_name)
            if hasattr(_mod, _attr):
                setattr(_mod, _attr, _val)


def _reset_backup_throttle():
    """重置 data_store 中 backup_data 的 60 秒节流，避免测试受其他用例污染。"""
    import server.data_store as ds
    ds._last_backup_time = 0


def test_backup_list_and_delete(isolated_data, seed_sessions, default_settings):
    from server.config import BACKUP_DIR as cfg_backup_dir, DB_FILE as cfg_db_file
    from server.data_store import backup_data
    from server.routes.backup_routes import list_backups, delete_backup

    _pin_backup_paths(cfg_backup_dir, cfg_db_file)
    _reset_backup_throttle()

    backup_data()
    _reset_backup_throttle()
    backup_data()

    import os
    files_on_disk = [f for f in os.listdir(cfg_backup_dir) if f.startswith("boot-data-") and f.endswith(".db")]

    status, body = list_backups(None, None)
    assert status == 200
    files = body["backups"]
    assert isinstance(files, list) and len(files) >= 1, (
        f"备份列表为空: files={files}, disk={files_on_disk}, dir={cfg_backup_dir}"
    )

    first_name = files[0]
    # delete_backup 使用 {"filename": name}
    status, _ = delete_backup(None, {"filename": first_name})
    assert status == 200

    status, body = list_backups(None, None)
    assert status == 200
    assert first_name not in body["backups"]


def test_backup_restore(isolated_data, seed_sessions, default_settings):
    from server.config import BACKUP_DIR as cfg_backup_dir, DB_FILE as cfg_db_file
    from server.data_store import backup_data, save_data, load_data
    from server.routes.backup_routes import list_backups, restore_backup

    _pin_backup_paths(cfg_backup_dir, cfg_db_file)
    _reset_backup_throttle()

    backup_data()
    save_data({"bootCount": 0, "shutdownCount": 0, "sessions": []})
    assert load_data()["bootCount"] == 0

    import os
    files_on_disk = [f for f in os.listdir(cfg_backup_dir) if f.startswith("boot-data-") and f.endswith(".db")]

    _, lst = list_backups(None, None)
    assert len(lst["backups"]) >= 1, f"无备份可恢复: disk={files_on_disk}, dir={cfg_backup_dir}"
    name = lst["backups"][0]
    # restore_backup 使用 {"filename": name}
    status, body = restore_backup(None, {"filename": name})
    assert status == 200, f"restore 失败: {body}"
    assert body["ok"] is True
    assert load_data()["bootCount"] == 3


# --------- stats_routes ---------
def test_overview_stats(isolated_data, seed_sessions):
    from server.routes.stats_routes import get_overview_stats
    status, body = get_overview_stats(None, None)
    assert status == 200
    # 字段名是 totalBoot / totalShutdown
    assert body["totalBoot"] == 3
    assert body["totalShutdown"] == 2
    # 平均时长：(4h + 8h) / 2 = 6h = 21600000 ms
    assert body["avgDuration"] == (4 + 8) * 3600 * 1000 // 2
    assert body["totalDuration"] == (4 + 8) * 3600 * 1000


def test_daily_and_weekly_counts(isolated_data, seed_sessions):
    from server.routes.stats_routes import get_daily_stats, get_weekly_stats

    status, daily = get_daily_stats(None, None)
    assert status == 200
    # daily 返回 {"data": {date: {...}}}
    assert isinstance(daily["data"], dict)
    assert len(daily["data"]) >= 1  # 至少有一天

    status, weekly = get_weekly_stats(None, None)
    assert status == 200
    assert isinstance(weekly.get("data"), (dict, list))
