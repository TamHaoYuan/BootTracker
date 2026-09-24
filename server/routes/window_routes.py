#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""窗口控制路由 - 主窗口显隐与主题相关 API

窗口模式：Tauri（新版本默认），无原生窗口时由调用方回退浏览器。
"""

from typing import Tuple, Dict

from ..logging_config import logger
from . import post, put


@post("/api/raise-window")
def raise_window(req, body) -> Tuple[int, Dict]:
    """唤起已运行实例的 Tauri 主窗口。

    无原生窗口时返回 ok=false，由调用方（二次启动进程）回退到浏览器。
    """
    try:
        from ..tauri_window import raise_tauri_window
        ok = raise_tauri_window()
        return 200, {"ok": ok}
    except Exception as e:
        logger.error(f"raise_window error: {e}")
        return 200, {"ok": False}


@put("/api/window-theme")
def set_window_theme_route(req, body) -> Tuple[int, Dict]:
    """同步主窗口外壳主题与持久化。

    前端在切换/加载明暗模式时调用，使窗口外壳与页面模式一致。
    """
    mode = body.get("mode", "dark")
    if mode not in ("dark", "light"):
        return 400, {"error": "mode must be dark or light"}
    
    # 持久化设置
    try:
        from ..settings import load_settings, save_settings
        settings = load_settings()
        settings["appMode"] = mode
        save_settings(settings)
    except Exception as e:
        logger.error(f"[window-theme] save appMode failed: {e}")
    
    applied = False
    try:
        from ..tauri_window import set_tauri_theme
        applied = set_tauri_theme(mode)
    except Exception as e:
        logger.error(f"[window-theme] set_tauri_theme failed: {e}")
    
    return 200, {"ok": True, "mode": mode, "applied": applied}
