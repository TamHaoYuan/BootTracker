#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""配置常量模块 — 集中管理所有路径、端口和全局常量"""
import os
import sys

if getattr(sys, 'frozen', False):
    APP_DIR = os.path.dirname(sys.executable)
    _RESOURCE_DIR = os.path.join(sys._MEIPASS)
else:
    APP_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    _RESOURCE_DIR = APP_DIR

PORT = 18792
LOCK_PORT = PORT + 1

HTML_FILE = os.path.join(_RESOURCE_DIR, "index.html")
DB_FILE = os.path.join(APP_DIR, "boot-data.db")
SETTINGS_FILE = os.path.join(APP_DIR, "settings.json")
VERSION_FILE = os.path.join(APP_DIR, "version.json")
PID_FILE = os.path.join(APP_DIR, "boot-tracker.pid")
LOG_DIR = os.path.join(APP_DIR, "logs")
LOG_FILE = os.path.join(LOG_DIR, "boot-tracker.log")
BACKUP_DIR = os.path.join(APP_DIR, "backup")
UPLOAD_DIR = os.path.join(APP_DIR, "static", "uploads")
TUNNEL_LOG_FILE = os.path.join(LOG_DIR, "tunnel.log")
VENDOR_DIR = os.path.join(APP_DIR, "vendor")
# 打包模式：cloudflared 由 spec 打进 _MEIPASS；开发模式：位于 vendor/
CLOUDFLARED_PATH = os.path.join(
    _RESOURCE_DIR if getattr(sys, "frozen", False) else VENDOR_DIR,
    "cloudflared.exe",
)

MAX_BACKUPS = 30
APP_VERSION = "2.1.0"
UPDATE_CHECK_URL = ""