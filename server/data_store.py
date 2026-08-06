#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""数据存储模块 — 负责开机记录、回收站和备份功能"""
import os
import sys
import json
import uuid
import shutil
import time
from datetime import datetime, timezone

from .config import DATA_FILE, TRASH_FILE, BACKUP_DIR, MAX_BACKUPS, PID_FILE, PORT

_LOCK_FILE = os.path.join(os.path.dirname(DATA_FILE), ".boot-data.lock")
_last_backup_time = 0


def _lock_data(timeout=10):
    deadline = time.time() + timeout
    while True:
        try:
            with open(_LOCK_FILE, "x", encoding="utf-8") as f:
                f.write(str(os.getpid()))
            return True
        except FileExistsError:
            try:
                with open(_LOCK_FILE) as f:
                    pid = int(f.read().strip())
                if sys.platform == "win32":
                    import ctypes
                    kernel32 = ctypes.windll.kernel32
                    handle = kernel32.OpenProcess(0x400, False, pid)
                    alive = bool(handle)
                    if handle:
                        kernel32.CloseHandle(handle)
                else:
                    os.kill(pid, 0)
                    alive = True
            except (OSError, ValueError, AttributeError):
                alive = False
            if not alive:
                try:
                    os.remove(_LOCK_FILE)
                    continue
                except Exception:
                    pass
            if time.time() > deadline:
                return False
            time.sleep(0.05)


def _unlock_data():
    try:
        if os.path.exists(_LOCK_FILE):
            os.remove(_LOCK_FILE)
    except Exception:
        pass


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
    global _last_backup_time
    try:
        if not os.path.exists(DATA_FILE):
            return
        now = time.time()
        if now - _last_backup_time < 60:
            return
        _ensure_backup_dir()
        ts = datetime.now().strftime("%Y%m%d-%H%M%S")
        dest = os.path.join(BACKUP_DIR, f"boot-data-{ts}.json")
        shutil.copy2(DATA_FILE, dest)
        _last_backup_time = now
        _cleanup_old_backups()
    except Exception:
        pass


def _cleanup_old_backups():
    try:
        files = [
            f for f in os.listdir(BACKUP_DIR)
            if f.startswith("boot-data-") and f.endswith(".json")
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
        if f.startswith("boot-data-") and f.endswith(".json")
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


def load_data():
    try:
        if os.path.exists(DATA_FILE):
            with open(DATA_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return {"bootCount": 0, "shutdownCount": 0, "sessions": []}


def load_trash():
    try:
        if os.path.exists(TRASH_FILE):
            with open(TRASH_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return {"sessions": []}


def save_trash(trash):
    _lock_data()
    try:
        with open(TRASH_FILE, "w", encoding="utf-8") as f:
            json.dump(trash, f, ensure_ascii=False, indent=2)
    finally:
        _unlock_data()


def save_data(data):
    _lock_data()
    try:
        backup_data()
        with open(DATA_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
    finally:
        _unlock_data()


def _utc_iso_to_local_date(iso_str: str) -> str:
    try:
        dt = datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
        return time.strftime("%Y-%m-%d", time.localtime(dt.timestamp()))
    except Exception:
        return ""


def auto_close_and_new_session():
    _lock_data()
    try:
        data = load_data()
        now_utc = datetime.now(timezone.utc)
        today_str = time.strftime("%Y-%m-%d")
        changed = False

        for s in data.get("sessions", []):
            if not s.get("shutdownTime"):
                try:
                    boot = datetime.fromisoformat(
                        s["bootTime"].replace("Z", "+00:00")
                    )
                    s["shutdownTime"] = now_utc.isoformat().replace("+00:00", "Z")
                    s["duration"] = int((now_utc - boot).total_seconds() * 1000)
                    data["shutdownCount"] = data.get("shutdownCount", 0) + 1
                    changed = True
                except Exception:
                    pass

        has_today_unclosed = any(
            _utc_iso_to_local_date(s.get("bootTime", "")) == today_str
            and not s.get("shutdownTime")
            for s in data.get("sessions", [])
        )

        if not has_today_unclosed:
            new_id = uuid.uuid4().hex[:14]
            data["bootCount"] = data.get("bootCount", 0) + 1
            data["sessions"].append({
                "id": new_id,
                "bootTime": now_utc.isoformat().replace("+00:00", "Z"),
                "shutdownTime": None,
                "duration": None,
            })
            changed = True

        if changed:
            backup_data()
            with open(DATA_FILE, "w", encoding="utf-8") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
    finally:
        _unlock_data()


def is_port_in_use(port=PORT):
    import socket
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("0.0.0.0", port))
            return False
        except OSError:
            return True