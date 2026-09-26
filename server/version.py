#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""版本管理模块 — 负责版本号管理和更新检查"""
import os
import json

from .config import VERSION_FILE, APP_VERSION, UPDATE_CHECK_URL


def _default_version_data():
    return {
        "version": APP_VERSION,
        "history": [],
    }


def load_version():
    if os.path.isfile(VERSION_FILE):
        try:
            with open(VERSION_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            pass
    data = _default_version_data()
    save_version(data)
    return data


def save_version(data):
    with open(VERSION_FILE, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def bump_version(bump_type, notes=""):
    from datetime import datetime, timezone
    data = load_version()
    parts = [int(x) for x in data["version"].split(".")]
    old_version = data["version"]

    if bump_type == "major":
        parts[0] += 1
        parts[1] = 0
        parts[2] = 0
    elif bump_type == "minor":
        parts[1] += 1
        parts[2] = 0
    else:
        parts[2] += 1

    new_version = ".".join(str(p) for p in parts)

    entry = {
        "from": old_version,
        "to": new_version,
        "type": bump_type,
        "notes": notes,
        "time": datetime.now(timezone.utc).isoformat(),
    }
    data["history"].insert(0, entry)
    data["history"] = data["history"][:50]
    data["version"] = new_version

    save_version(data)
    return data


def _version_compare(v1, v2):
    def _parts(v):
        return [int(x) for x in v.replace("v", "").split(".")]
    p1, p2 = _parts(v1), _parts(v2)
    for a, b in zip(p1, p2):
        if a > b:
            return 1
        if a < b:
            return -1
    return 0


def check_update():
    import urllib.request
    # 用运行时真实版本（version.json）作为“当前版本”，与 /api/version 显示保持一致；
    # 避免 config.APP_VERSION 常量滞后导致误报更新
    current = load_version().get("version", APP_VERSION)
    if not UPDATE_CHECK_URL:
        return {
            "current": current,
            "latest": None,
            "hasUpdate": False,
            "message": "未配置更新检查地址",
        }
    try:
        req = urllib.request.Request(
            UPDATE_CHECK_URL,
            headers={"User-Agent": f"BootTracker/{current}"},
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
        latest_ver = data.get("version", "")
        has_update = _version_compare(latest_ver, current) > 0
        return {
            "current": current,
            "latest": latest_ver,
            "hasUpdate": has_update,
            "url": data.get("url", ""),
            "notes": data.get("notes", ""),
        }
    except Exception as e:
        return {
            "current": current,
            "latest": None,
            "hasUpdate": False,
            "message": f"检查失败: {e}",
        }