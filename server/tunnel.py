#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Cloudflare Tunnel 模块 — 负责隧道的启动、停止和 URL 解析"""
import os
import subprocess
import threading
import time
import re

from .config import CLOUDFLARED_PATH, APP_DIR, TUNNEL_LOG_FILE, PORT

_tunnel_proc = None
_tunnel_url = None


def _find_cloudflared():
    if os.path.exists(CLOUDFLARED_PATH):
        return CLOUDFLARED_PATH
    alt_path = os.path.join(APP_DIR, "cloudflared.exe")
    if os.path.exists(alt_path):
        return alt_path
    return None


def _parse_tunnel_url(log_path):
    url_pattern = re.compile(r'https://[a-z0-9-]+\.trycloudflare\.com')
    for _ in range(60):
        time.sleep(0.5)
        try:
            with open(log_path, "r", encoding="utf-8") as f:
                content = f.read()
            m = url_pattern.search(content)
            if m:
                global _tunnel_url
                _tunnel_url = m.group(0)
                return
        except Exception:
            pass


def start_tunnel(tunnel_token="", custom_domain=""):
    global _tunnel_proc, _tunnel_url
    cf_path = _find_cloudflared()
    if not cf_path:
        return

    try:
        log_path = TUNNEL_LOG_FILE

        if tunnel_token:
            proc = subprocess.Popen(
                [cf_path, "tunnel", "run", "--token", tunnel_token],
                stdout=open(log_path, "w", encoding="utf-8"),
                stderr=subprocess.STDOUT,
                cwd=APP_DIR,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0,
            )
            _tunnel_proc = proc
            if custom_domain:
                _tunnel_url = "https://" + custom_domain
            else:
                _tunnel_url = None
                threading.Thread(target=_parse_tunnel_url, args=(log_path,), daemon=True).start()
        else:
            proc = subprocess.Popen(
                [cf_path, "tunnel", "--url", "http://localhost:" + str(PORT)],
                stdout=open(log_path, "w", encoding="utf-8"),
                stderr=subprocess.STDOUT,
                cwd=APP_DIR,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0,
            )
            _tunnel_proc = proc
            _tunnel_url = None
            threading.Thread(target=_parse_tunnel_url, args=(log_path,), daemon=True).start()
    except Exception:
        pass


def stop_tunnel():
    global _tunnel_proc, _tunnel_url
    proc = _tunnel_proc
    if proc is None:
        return
    try:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
        _tunnel_proc = None
        _tunnel_url = None
    except Exception:
        pass


def get_tunnel_url():
    return _tunnel_url