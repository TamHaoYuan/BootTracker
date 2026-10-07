#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Cloudflare 隧道模块 —— 一键临时隧道 + 二进制探测/下载 + 可读错误反馈。

设计要点（与旧版的差别）：
* 旧版 `start_tunnel()` 在找不到 `cloudflared.exe` 时**静默 return**，界面上只会
  一直显示「未启用」，用户完全不知道发生了什么。现在所有失败都记录到
  `_state["error"]`（英文短句，前端按 key 本地化），并同步 `logger.error`。
* 旧版没有「下载 cloudflared」的能力，用户得自己去找。现在提供
  `download_cloudflared()`：从 Cloudflare 官方 release 拉 `cloudflared-windows-amd64.exe`
  落到 `__file__` 同级目录，带进度与体积校验（< 5 MB 视为失败）。
* 临时隧道的 URL 从进程 stdout 日志里正则抓取，抓不到就在 45 秒后置超时错误，
  并在「进程已退出」时立刻给出退出码——这样界面能立刻显示原因而不是空等。

错误 key 一览（前端负责翻译）：
    cloudflared_missing / download_failed / download_in_progress /
    spawn_failed / exited / url_timeout / not_running
"""
import os
import re
import subprocess
import threading
import time
import urllib.request

from .config import CLOUDFLARED_PATH, APP_DIR, TUNNEL_LOG_FILE, PORT
from .logging_config import logger

# 官方 release 的稳定跳转地址（永远指向最新版）
DOWNLOAD_URL = (
    "https://github.com/cloudflare/cloudflared/releases/latest/download/"
    "cloudflared-windows-amd64.exe"
)
# 体积下限：真实二进制 ~60 MB，低于 5 MB 基本是错误页/代理拦截
MIN_BINARY_BYTES = 5 * 1024 * 1024
URL_WAIT_SECONDS = 45

_proc = None
_url = None
_lock = threading.Lock()
_state = {
    "error": "",
    "error_detail": "",
    "mode": "",
    "download": {"status": "idle", "error": "", "bytes": 0},
}

# 兼容旧调用点（settings_routes 曾 `from ..tunnel import _tunnel_proc`）
_tunnel_proc = None
_tunnel_url = None

_URL_RE = re.compile(r"https://[a-z0-9-]+\.trycloudflare\.com")


def _sync_legacy():
    """把内部状态同步到旧模块级变量，避免历史调用点读到过期值。"""
    global _tunnel_proc, _tunnel_url
    _tunnel_proc = _proc
    _tunnel_url = _url


def _set_error(key, detail=""):
    with _lock:
        _state["error"] = key
        _state["error_detail"] = detail
    if detail:
        logger.error(f"[tunnel] {key}: {detail}")
    else:
        logger.error(f"[tunnel] {key}")


def _clear_error():
    with _lock:
        _state["error"] = ""
        _state["error_detail"] = ""


def _find_cloudflared():
    if os.path.exists(CLOUDFLARED_PATH):
        return CLOUDFLARED_PATH
    alt_path = os.path.join(APP_DIR, "cloudflared.exe")
    if os.path.exists(alt_path):
        return alt_path
    return None


def binary_version():
    """探测 cloudflared 版本；不可用时返回空串。"""
    path = _find_cloudflared()
    if not path:
        return ""
    try:
        out = subprocess.run(
            [path, "--version"],
            capture_output=True,
            text=True,
            timeout=8,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        text = (out.stdout or out.stderr or "").strip()
        return text.splitlines()[0] if text else ""
    except Exception as e:  # 二进制损坏 / 被杀软拦截
        logger.warning(f"[tunnel] cloudflared --version failed: {e}")
        return ""


def download_cloudflared():
    """下载 cloudflared.exe 到安装目录。同步阻塞，供后台线程调用。

    返回 (ok, error_key)。状态通过 `_state["download"]` 暴露给轮询接口。
    """
    with _lock:
        if _state["download"]["status"] == "downloading":
            return False, "download_in_progress"
        _state["download"] = {"status": "downloading", "error": "", "bytes": 0}

    target = CLOUDFLARED_PATH if os.path.isdir(os.path.dirname(CLOUDFLARED_PATH)) else os.path.join(APP_DIR, "cloudflared.exe")
    tmp = target + ".part"
    try:
        os.makedirs(os.path.dirname(target), exist_ok=True)
        logger.info(f"[tunnel] downloading cloudflared → {target}")
        req = urllib.request.Request(
            DOWNLOAD_URL, headers={"User-Agent": "BootTracker/2.1"}
        )
        got = 0
        with urllib.request.urlopen(req, timeout=30) as resp, open(tmp, "wb") as fh:
            while True:
                chunk = resp.read(256 * 1024)
                if not chunk:
                    break
                fh.write(chunk)
                got += len(chunk)
                with _lock:
                    _state["download"]["bytes"] = got
        if got < MIN_BINARY_BYTES:
            try:
                os.remove(tmp)
            except OSError:
                pass
            with _lock:
                _state["download"] = {
                    "status": "failed",
                    "error": "download_failed",
                    "bytes": got,
                }
            _set_error("download_failed", f"only {got} bytes")
            return False, "download_failed"
        os.replace(tmp, target)
        with _lock:
            _state["download"] = {"status": "done", "error": "", "bytes": got}
        _clear_error()
        logger.info(f"[tunnel] cloudflared downloaded ({got} bytes)")
        return True, ""
    except Exception as e:
        try:
            if os.path.exists(tmp):
                os.remove(tmp)
        except OSError:
            pass
        with _lock:
            _state["download"] = {"status": "failed", "error": "download_failed", "bytes": 0}
        _set_error("download_failed", str(e))
        return False, "download_failed"


def download_cloudflared_async():
    """后台下载，立即返回。"""
    threading.Thread(target=download_cloudflared, daemon=True).start()


def _wait_for_exit(proc, timeout):
    """在 timeout 秒内若进程退出则返回退出码，否则 None。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        code = proc.poll()
        if code is not None:
            return code
        time.sleep(0.5)
    return None


def _tail_log(path, limit=600):
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return f.read()[-limit:].strip()
    except OSError:
        return ""


def _parse_tunnel_url(log_path, proc):
    """轮询日志抓 trycloudflare 地址；抓不到就写明确原因。"""
    global _url
    for _ in range(int(URL_WAIT_SECONDS / 0.5)):
        time.sleep(0.5)
        code = proc.poll()
        if code is not None:
            # 进程已经死了：退出码 + 日志尾巴，直接告诉用户为什么
            with _lock:
                if not _state.get("error"):
                    _state["error"] = "exited"
                    _state["error_detail"] = f"code={code} {_tail_log(log_path, 300)}"
            logger.error(f"[tunnel] cloudflared exited early code={code}")
            return
        try:
            with open(log_path, "r", encoding="utf-8", errors="replace") as f:
                content = f.read()
        except OSError:
            continue
        m = _URL_RE.search(content)
        if m:
            with _lock:
                _url = m.group(0)
                _state["error"] = ""
                _state["error_detail"] = ""
            _sync_legacy()
            logger.info(f"[tunnel] url = {_url}")
            return
    # 超时：把日志尾巴留给用户排查
    with _lock:
        if not _state.get("error"):
            _state["error"] = "url_timeout"
            _state["error_detail"] = _tail_log(log_path, 300)
    logger.error("[tunnel] url not found in log within timeout")


def start_tunnel(tunnel_token="", custom_domain=""):
    """启动隧道。返回 (ok, error_key)。

    调用方可以忽略返回值（旧调用点就是忽略的），错误同时记录在
    `tunnel_state()` 里供界面读取。
    """
    global _proc, _url
    _clear_error()
    stop_tunnel()

    cf_path = _find_cloudflared()
    if not cf_path:
        _set_error("cloudflared_missing")
        return False, "cloudflared_missing"

    try:
        log_path = TUNNEL_LOG_FILE
        os.makedirs(os.path.dirname(log_path), exist_ok=True)
        if tunnel_token:
            args = [cf_path, "tunnel", "run", "--token", tunnel_token]
        else:
            args = [cf_path, "tunnel", "--url", "http://localhost:" + str(PORT)]
        # 覆盖写日志：这样 URL 解析只会命中本次运行的输出
        log_fh = open(log_path, "w", encoding="utf-8")
        proc = subprocess.Popen(
            args,
            stdout=log_fh,
            stderr=subprocess.STDOUT,
            cwd=APP_DIR,
            creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
        )
        log_fh.close()
    except Exception as e:
        _set_error("spawn_failed", str(e))
        return False, "spawn_failed"

    with _lock:
        _proc = proc
        _url = None
        _state["mode"] = "token" if tunnel_token else "quick"
        if tunnel_token and custom_domain:
            _url = "https://" + custom_domain
    _sync_legacy()

    if not (_url):
        threading.Thread(
            target=_parse_tunnel_url, args=(TUNNEL_LOG_FILE, proc), daemon=True
        ).start()
    return True, ""


def stop_tunnel():
    global _proc, _url
    with _lock:
        proc = _proc
        _proc = None
        _url = None
        _state["mode"] = ""
    _sync_legacy()
    if proc is None:
        return
    try:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
    except Exception as e:
        logger.warning(f"[tunnel] stop failed: {e}")


def restart_tunnel(tunnel_token="", custom_domain=""):
    stop_tunnel()
    return start_tunnel(tunnel_token=tunnel_token, custom_domain=custom_domain)


def get_tunnel_url():
    with _lock:
        return _url


def tunnel_state():
    """供界面轮询的完整状态快照。"""
    with _lock:
        proc = _proc
        return {
            "running": proc is not None and proc.poll() is None,
            "url": _url or "",
            "mode": _state.get("mode", ""),
            "error": _state.get("error", ""),
            "errorDetail": _state.get("error_detail", ""),
            "installed": _find_cloudflared() is not None,
            "binaryVersion": binary_version(),
            "binaryPath": _find_cloudflared() or "",
            "download": dict(_state["download"]),
        }


def remove_cloudflared():
    """删除已下载的二进制（界面上的「移除」按钮）。"""
    stop_tunnel()
    path = _find_cloudflared()
    if not path:
        return False, "not_found"
    try:
        os.remove(path)
        logger.info(f"[tunnel] removed {path}")
        return True, ""
    except Exception as e:
        _set_error("remove_failed", str(e))
        return False, "remove_failed"
