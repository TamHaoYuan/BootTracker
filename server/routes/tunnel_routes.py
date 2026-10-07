#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""隧道路由模块 - Cloudflare Tunnel 相关 API

界面上「一键临时隧道」需要的全部动作都在这里：
* `GET  /api/tunnel-status`   —— 轮询状态（是否装好 cloudflared、URL、错误原因、下载进度）
* `POST /api/tunnel-start`    —— 显式启动（先用 `tunnelEnabled=true` 也能启，这里是显式按钮）
* `POST /api/tunnel-stop`     —— 停止
* `POST /api/tunnel-download` —— 一键下载 cloudflared 到安装目录
* `POST /api/tunnel-remove`   —— 移除已下载的 cloudflared
"""

from typing import Tuple, Dict

from ..tunnel import (
    download_cloudflared_async,
    get_tunnel_url,
    remove_cloudflared,
    start_tunnel,
    stop_tunnel,
    tunnel_state,
)
from ..settings import load_settings, save_settings
from ..logging_config import logger
from . import get, post


@get("/api/tunnel-url")
def get_tunnel_url_handler(req, body) -> Tuple[int, Dict]:
    """获取隧道 URL（保留旧接口，独立小组件/脚本可能还在用）"""
    return 200, {"url": get_tunnel_url()}


@get("/api/tunnel-status")
def tunnel_status_handler(req, body) -> Tuple[int, Dict]:
    """完整隧道状态，供界面轮询（建议 2s 一次）"""
    try:
        return 200, tunnel_state()
    except Exception as e:
        logger.error(f"tunnel_status error: {e}")
        return 400, {"error": str(e)}


@post("/api/tunnel-start")
def tunnel_start_handler(req, body) -> Tuple[int, Dict]:
    """启动隧道（无 token 即临时隧道）。

    注意：**即使失败也返回 200**，把 `ok:false` + 错误 key 放在 body 里。
    客户端（Rust ureq）在 4xx 时拿不到 body，只看得到 "http status 400"，
    界面上就没法展示「没装 cloudflared」这种真正有用的原因了。
    """
    try:
        settings = load_settings()
        token = settings.get("tunnelToken", "")
        domain = settings.get("customDomain", "")
        ok, error = start_tunnel(tunnel_token=token, custom_domain=domain)
        return 200, {"ok": ok, "error": error, **tunnel_state()}
    except Exception as e:
        logger.error(f"tunnel_start error: {e}")
        return 200, {"ok": False, "error": "spawn_failed", "errorDetail": str(e)}


@post("/api/tunnel-stop")
def tunnel_stop_handler(req, body) -> Tuple[int, Dict]:
    """停止隧道并同步关闭 `tunnelEnabled`，避免下次启动又被自动拉起。"""
    try:
        stop_tunnel()
        settings = load_settings()
        if settings.get("tunnelEnabled"):
            settings["tunnelEnabled"] = False
            save_settings(settings)
        return 200, {"ok": True, **tunnel_state()}
    except Exception as e:
        logger.error(f"tunnel_stop error: {e}")
        return 400, {"ok": False, "error": str(e)}


@post("/api/tunnel-download")
def tunnel_download_handler(req, body) -> Tuple[int, Dict]:
    """一键下载 cloudflared（后台线程，界面轮询 /api/tunnel-status 看进度）"""
    try:
        download_cloudflared_async()
        return 200, {"ok": True, **tunnel_state()}
    except Exception as e:
        logger.error(f"tunnel_download error: {e}")
        return 400, {"ok": False, "error": str(e)}


@post("/api/tunnel-remove")
def tunnel_remove_handler(req, body) -> Tuple[int, Dict]:
    """移除已下载的 cloudflared（同样 200 + ok:false，理由见 tunnel_start_handler）"""
    try:
        ok, error = remove_cloudflared()
        return 200, {"ok": ok, "error": error, **tunnel_state()}
    except Exception as e:
        logger.error(f"tunnel_remove error: {e}")
        return 200, {"ok": False, "error": "remove_failed", "errorDetail": str(e)}
