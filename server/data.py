#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""数据管理模块 — 负责数据的读写、备份和开机会话处理"""
import os
import sys
import json
import uuid
import shutil
import socket
from datetime import datetime, timezone

# 路径常量：基于项目根目录（boot-tracker.py 所在目录的上一级）
APP_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HTML_FILE = os.path.join(APP_DIR, "index.html")
DATA_FILE = os.path.join(APP_DIR, "boot-data.json")
TRASH_FILE = os.path.join(APP_DIR, "trash-data.json")
BACKUP_DIR = os.path.join(APP_DIR, "backup")
MAX_BACKUPS = 30
PORT = 18792


def _ensure_backup_dir():
    """确保备份目录存在"""
    if not os.path.isdir(BACKUP_DIR):
        os.makedirs(BACKUP_DIR, exist_ok=True)


def backup_data():
    """备份当前数据文件到 backup/ 目录，保留最近 MAX_BACKUPS 份"""
    try:
        if not os.path.exists(DATA_FILE):
            return
        _ensure_backup_dir()
        ts = datetime.now().strftime("%Y%m%d-%H%M%S")
        dest = os.path.join(BACKUP_DIR, f"boot-data-{ts}.json")
        shutil.copy2(DATA_FILE, dest)
        _cleanup_old_backups()
    except Exception:
        pass


def _cleanup_old_backups():
    """删除最旧的备份，只保留最近 MAX_BACKUPS 份"""
    try:
        files = [
            f for f in os.listdir(BACKUP_DIR)
            if f.startswith("boot-data-") and f.endswith(".json")
        ]
        if len(files) <= MAX_BACKUPS:
            return
        files.sort(reverse=True)  # 最新的在前
        for f in files[MAX_BACKUPS:]:
            os.remove(os.path.join(BACKUP_DIR, f))
    except Exception:
        pass


def load_data():
    """读取数据文件"""
    try:
        if os.path.exists(DATA_FILE):
            with open(DATA_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return {"bootCount": 0, "shutdownCount": 0, "sessions": []}


def load_trash():
    """读取回收站数据"""
    try:
        if os.path.exists(TRASH_FILE):
            with open(TRASH_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return {"sessions": []}


def save_trash(trash):
    """写入回收站数据"""
    try:
        with open(TRASH_FILE, "w", encoding="utf-8") as f:
            json.dump(trash, f, ensure_ascii=False, indent=2)
    except Exception:
        pass


def save_data(data):
    """写入数据文件（写入前自动备份）"""
    try:
        backup_data()
        with open(DATA_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
    except Exception:
        pass


def auto_close_and_new_session():
    """开机时自动关闭上一次的活跃会话，创建本次开机会话。

    此函数仅在服务器首次启动时调用（即真正的开机时刻），
    因此遇到未关闭的会话必定是上一次关机前遗漏的。
    """
    data = load_data()
    changed = False
    now = datetime.now(timezone.utc)

    # 关闭所有未结束的会话（上一次开机遗留）
    for s in data.get("sessions", []):
        if not s.get("shutdownTime"):
            try:
                boot = datetime.fromisoformat(
                    s["bootTime"].replace("Z", "+00:00")
                )
                s["shutdownTime"] = now.isoformat().replace("+00:00", "Z")
                s["duration"] = int((now - boot).total_seconds() * 1000)
                data["shutdownCount"] = data.get("shutdownCount", 0) + 1
                changed = True
            except Exception:
                pass

    # 创建本次开机会话
    new_id = uuid.uuid4().hex[:14]
    data["bootCount"] = data.get("bootCount", 0) + 1
    data["sessions"].append({
        "id": new_id,
        "bootTime": now.isoformat().replace("+00:00", "Z"),
        "shutdownTime": None,
        "duration": None,
    })
    changed = True

    if changed:
        save_data(data)


def is_port_in_use(port):
    """检测端口是否已被占用（说明后台服务已在运行）"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("127.0.0.1", port))
            return False
        except OSError:
            return True


def setup_autostart(enable=True):
    """设置/取消开机自启（Windows 注册表）"""
    try:
        import winreg
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0,
            winreg.KEY_SET_VALUE,
        )
        if enable:
            pythonw = sys.executable.replace("python.exe", "pythonw.exe")
            if not os.path.exists(pythonw):
                pythonw = os.path.join(os.path.dirname(sys.executable), "pythonw.exe")
            cmd = '"' + pythonw + '" "' + os.path.abspath(
                os.path.join(APP_DIR, "boot-tracker.py")
            ) + '" --background'
            winreg.SetValueEx(key, "BootTracker", 0, winreg.REG_SZ, cmd)
        else:
            try:
                winreg.DeleteValue(key, "BootTracker")
            except FileNotFoundError:
                pass
        winreg.CloseKey(key)
    except Exception:
        pass
