#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""设置管理模块 — 负责应用设置的读写和开机自启配置"""
import os
import json
import copy

from .config import SETTINGS_FILE

_DEFAULT_SETTINGS = {
    "autoStart": True,
    "autoBackup": True,
    "backupCount": 30,
    "autoCloseIdle": False,
    "idleCloseMinutes": 5,
    "defaultChartType": "bar",
    "timeFormat": "24h",
    "customBgImage": "",
    "lanAccess": True,
    "tunnelEnabled": False,
    "tunnelToken": "",
    "customDomain": "",
    "widgetEnabled": False,
    "widgetPosition": "",
    "appMode": "dark",
}


def _default_settings():
    return copy.deepcopy(_DEFAULT_SETTINGS)


def load_settings():
    if not os.path.isfile(SETTINGS_FILE):
        return _default_settings()
    try:
        with open(SETTINGS_FILE, "r", encoding="utf-8") as f:
            data = json.load(f)
        defaults = _default_settings()
        for k, v in defaults.items():
            if k not in data:
                data[k] = v
        return data
    except Exception:
        return _default_settings()


def save_settings(data):
    with open(SETTINGS_FILE, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def setup_autostart(enable=True):
    try:
        import winreg
        import sys
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
            cmd = '"' + pythonw + '" "' + os.path.abspath(sys.argv[0]) + '" --background'
            winreg.SetValueEx(key, "BootTracker", 0, winreg.REG_SZ, cmd)
        else:
            try:
                winreg.DeleteValue(key, "BootTracker")
            except FileNotFoundError:
                pass
        winreg.CloseKey(key)
    except Exception:
        pass


def is_autostart_registered():
    try:
        import winreg
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER,
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            0,
            winreg.KEY_READ,
        )
        val, _ = winreg.QueryValueEx(key, "BootTracker")
        winreg.CloseKey(key)
        return bool(val)
    except Exception:
        return False