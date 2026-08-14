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
DATA_FILE = os.path.join(APP_DIR, "boot-data.json")
TRASH_FILE = os.path.join(APP_DIR, "trash-data.json")
SETTINGS_FILE = os.path.join(APP_DIR, "settings.json")
VERSION_FILE = os.path.join(APP_DIR, "version.json")
PID_FILE = os.path.join(APP_DIR, "boot-tracker.pid")
LOG_FILE = os.path.join(APP_DIR, "boot-tracker.log")
BACKUP_DIR = os.path.join(APP_DIR, "backup")
UPLOAD_DIR = os.path.join(APP_DIR, "static", "uploads")
TUNNEL_LOG_FILE = os.path.join(APP_DIR, "tunnel.log")
CLOUDFLARED_PATH = os.path.join(_RESOURCE_DIR, "cloudflared.exe")

MAX_BACKUPS = 30
APP_VERSION = "1.1.0"
UPDATE_CHECK_URL = ""