#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""数据存储模块 — 基于 SQLite 的开机记录、回收站和备份功能

数据模型：
- sessions 表：存储所有开机会话记录
- trash 表：存储已删除的会话记录（回收站）

接口说明：
- load_data() / save_data() 保持与旧版相同的接口，路由层无需修改
"""
import os
import sys
import uuid
import shutil
import sqlite3
import time
import threading
from datetime import datetime, timedelta, timezone

from .config import DB_FILE, BACKUP_DIR, MAX_BACKUPS, PID_FILE, PORT

_last_backup_time = 0
_local = threading.local()


def _get_conn() -> sqlite3.Connection:
    """获取当前线程的数据库连接（线程安全）"""
    conn = getattr(_local, "conn", None)
    if conn is None:
        conn = sqlite3.connect(DB_FILE, timeout=10, check_same_thread=False)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA foreign_keys=ON")
        _local.conn = conn
    return conn


def _init_db() -> None:
    """初始化数据库表结构"""
    conn = _get_conn()
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            boot_time TEXT NOT NULL,
            shutdown_time TEXT,
            duration INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_sessions_boot ON sessions(boot_time);

        CREATE TABLE IF NOT EXISTS trash (
            id TEXT PRIMARY KEY,
            boot_time TEXT,
            shutdown_time TEXT,
            duration INTEGER,
            deleted_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    """)
    conn.commit()


# 模块加载时初始化数据库
_init_db()


def is_already_running():
    if not os.path.exists(PID_FILE):
        return False
    try:
        with open(PID_FILE, "r") as f:
            pid = int(f.read().strip())
        if sys.platform == "win32":
            import ctypes
            kernel32 = ctypes.windll.kernel32
            handle = kernel32.OpenProcess(0x400, False, pid)
            if handle:
                kernel32.CloseHandle(handle)
                return True
            return False
        else:
            os.kill(pid, 0)
            return True
    except (OSError, ValueError):
        try:
            os.remove(PID_FILE)
        except Exception:
            pass
        return False


def write_pid_file():
    try:
        with open(PID_FILE, "w") as f:
            f.write(str(os.getpid()))
    except Exception:
        pass


def remove_pid_file():
    try:
        os.remove(PID_FILE)
    except Exception:
        pass


def _ensure_backup_dir():
    if not os.path.isdir(BACKUP_DIR):
        os.makedirs(BACKUP_DIR, exist_ok=True)


def backup_data():
    """备份数据库文件"""
    global _last_backup_time
    try:
        if not os.path.exists(DB_FILE):
            return None
        now = time.time()
        if now - _last_backup_time < 60:
            return None
        _ensure_backup_dir()
        ts = datetime.now().strftime("%Y%m%d-%H%M%S")
        dest = os.path.join(BACKUP_DIR, f"boot-data-{ts}.db")
        # 使用 SQLite 的 backup API 确保一致性
        src_conn = _get_conn()
        dst_conn = sqlite3.connect(dest)
        src_conn.backup(dst_conn)
        dst_conn.close()
        _last_backup_time = now
        _cleanup_old_backups()
        return dest
    except Exception:
        return None


def _cleanup_old_backups():
    try:
        files = [
            f for f in os.listdir(BACKUP_DIR)
            if f.startswith("boot-data-") and f.endswith(".db")
        ]
        if len(files) <= MAX_BACKUPS:
            return
        files.sort(reverse=True)
        for f in files[MAX_BACKUPS:]:
            os.remove(os.path.join(BACKUP_DIR, f))
    except Exception:
        pass


def clean_backups(keep=10):
    _ensure_backup_dir()
    files = [
        f for f in os.listdir(BACKUP_DIR)
        if f.startswith("boot-data-") and f.endswith(".db")
    ]
    files.sort(reverse=True)
    deleted = 0
    for f in files[keep:]:
        try:
            os.remove(os.path.join(BACKUP_DIR, f))
            deleted += 1
        except Exception:
            pass
    return deleted, max(0, min(keep, len(files)))


def _row_to_session(row) -> dict:
    """将数据库行转换为旧版格式的字典"""
    return {
        "id": row["id"],
        "bootTime": row["boot_time"],
        "shutdownTime": row["shutdown_time"],
        "duration": row["duration"],
    }


def load_data():
    """加载所有开机记录（返回与旧版相同的格式）"""
    try:
        conn = _get_conn()
        rows = conn.execute("SELECT * FROM sessions ORDER BY boot_time").fetchall()
        sessions = [_row_to_session(r) for r in rows]
        boot_count = len(sessions)
        shutdown_count = sum(1 for s in sessions if s["shutdownTime"])
        return {
            "bootCount": boot_count,
            "shutdownCount": shutdown_count,
            "sessions": sessions,
        }
    except Exception:
        return {"bootCount": 0, "shutdownCount": 0, "sessions": []}


def load_trash():
    """加载回收站数据"""
    try:
        conn = _get_conn()
        rows = conn.execute("SELECT * FROM trash ORDER BY boot_time").fetchall()
        sessions = [_row_to_session(r) for r in rows]
        return {"sessions": sessions}
    except Exception:
        return {"sessions": []}


def save_trash(trash):
    """保存回收站数据（全量替换）"""
    conn = _get_conn()
    conn.execute("DELETE FROM trash")
    for s in trash.get("sessions", []):
        conn.execute(
            "INSERT INTO trash (id, boot_time, shutdown_time, duration) VALUES (?, ?, ?, ?)",
            (s["id"], s.get("bootTime"), s.get("shutdownTime"), s.get("duration"))
        )
    conn.commit()


def save_data(data):
    """保存数据（全量替换）"""
    backup_data()
    conn = _get_conn()
    conn.execute("DELETE FROM sessions")
    for s in data.get("sessions", []):
        conn.execute(
            "INSERT OR REPLACE INTO sessions (id, boot_time, shutdown_time, duration) VALUES (?, ?, ?, ?)",
            (s["id"], s.get("bootTime"), s.get("shutdownTime"), s.get("duration"))
        )
    conn.commit()


def _utc_iso_to_local_date(iso_str: str) -> str:
    try:
        dt = datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
        return time.strftime("%Y-%m-%d", time.localtime(dt.timestamp()))
    except Exception:
        return ""


def _local_day_utc_range(local_date: str):
    """本地自然日 [00:00, 次日 00:00) 对应的 UTC ISO 边界

    boot_time 一律存 UTC（见 auto_close_and_new_session 的 now_iso），
    而"今天"是本地概念。直接用 `date(boot_time) = 本地日期` 比较会漏判：
    在 UTC+8 下，本地 00:00–08:00 的开机其 UTC 日期仍属前一天，
    于是下次启动会误判为"今天还没有会话"而重复新建。
    """
    day = datetime.strptime(local_date, "%Y-%m-%d")
    start = day.astimezone(timezone.utc)
    end = start + timedelta(days=1)
    return (
        start.replace(tzinfo=None).isoformat() + "Z",
        end.replace(tzinfo=None).isoformat() + "Z",
    )


def auto_close_and_new_session():
    """自动关闭未结束会话并创建新会话"""
    conn = _get_conn()
    now_utc = datetime.now(timezone.utc)
    today_str = time.strftime("%Y-%m-%d")
    now_iso = now_utc.isoformat().replace("+00:00", "Z")

    # 关闭所有未结束的会话
    unclosed = conn.execute(
        "SELECT id, boot_time FROM sessions WHERE shutdown_time IS NULL"
    ).fetchall()

    for row in unclosed:
        try:
            boot = datetime.fromisoformat(row["boot_time"].replace("Z", "+00:00"))
            if boot.tzinfo is None:
                # 兼容无时区的历史数据：按 UTC 解释，避免与 now_utc 相减时报错
                boot = boot.replace(tzinfo=timezone.utc)
            shutdown_iso = now_iso
            duration_ms = max(0, int((now_utc - boot).total_seconds() * 1000))
            conn.execute(
                "UPDATE sessions SET shutdown_time = ?, duration = ? WHERE id = ?",
                (shutdown_iso, duration_ms, row["id"])
            )
        except Exception:
            pass

    # 检查"今天"（本地自然日）是否已有会话：用 UTC 边界比较，勿用 date(boot_time)
    day_start_utc, day_end_utc = _local_day_utc_range(today_str)
    today_row = conn.execute(
        "SELECT COUNT(*) as cnt FROM sessions WHERE boot_time >= ? AND boot_time < ?",
        (day_start_utc, day_end_utc)
    ).fetchone()

    if today_row["cnt"] == 0:
        new_id = uuid.uuid4().hex[:14]
        conn.execute(
            "INSERT INTO sessions (id, boot_time, shutdown_time, duration) VALUES (?, ?, NULL, NULL)",
            (new_id, now_iso)
        )

    conn.commit()
    backup_data()


def is_port_in_use(port=PORT):
    import socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("0.0.0.0", port))
            return False
        except OSError:
            return True
