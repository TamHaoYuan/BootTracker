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
    "appTheme": "mica",
    # 界面缩放比例（1.0 = 100%），桌面端 egui 与 Web 端各自读取
    "uiScale": 1.0,
    # 界面语言："" = 跟随系统（首次运行时按 OS 语言决定并写回），
    # 否则为 "zh-CN" / "en-US"。桌面端 egui 与 Web 端各自读取同一个值。
    "language": "",
}


def detect_system_language():
    """探测操作系统界面语言，返回 "zh-CN" 或 "en-US"。

    非中文系统一律回落到 en-US（不做第三语言支持）。
    """
    import locale as _locale
    tag = ""
    try:
        if os.name == "nt":
            import ctypes

            buf = ctypes.create_unicode_buffer(85)
            if ctypes.windll.kernel32.GetUserDefaultLocaleName(buf, 85):
                tag = buf.value or ""
        if not tag:
            tag = _locale.getdefaultlocale()[0] or ""
    except Exception:
        tag = ""
    if not tag:
        tag = os.environ.get("LANG", "") or os.environ.get("LC_ALL", "")
    tag = (tag or "").lower()
    if tag.startswith("zh"):
        return "zh-CN"
    return "en-US"


def resolve_language(settings=None):
    """把 settings 里的 language 解析成具体语言代码。

    ""（跟随系统）时返回探测结果，但**不写回**——真正落盘由
    boot-tracker.py 启动阶段调用 `ensure_language()` 完成。
    """
    if settings is None:
        settings = load_settings()
    lang = (settings or {}).get("language", "")
    if lang in ("zh-CN", "en-US"):
        return lang
    return detect_system_language()


def ensure_language():
    """首次运行时把探测到的语言固化进 settings.json，返回最终语言代码。"""
    try:
        data = load_settings()
        lang = (data or {}).get("language", "")
        if lang not in ("zh-CN", "en-US"):
            lang = detect_system_language()
            data["language"] = lang
            save_settings(data)
        return lang
    except Exception:
        return detect_system_language()


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