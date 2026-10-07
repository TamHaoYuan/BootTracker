#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Cloudflare 隧道的错误反馈与一键下载行为测试。

历史问题：`start_tunnel()` 在找不到 `cloudflared.exe` 时静默 return，界面上只显示
「未启用」，用户无法判断是没装、被杀软拦了还是网络问题。这些用例锁死「任何失败
都必须留下可读的 error key + errorDetail」这一契约。

用例全部避免真的启动 cloudflared / 真的下载：用 monkeypatch 把二进制路径指到
临时目录里不存在的文件，并用假的 `urllib.request` 代替网络。
"""
import time

import pytest

import server.tunnel as tunnel


@pytest.fixture()
def no_binary(tmp_path, monkeypatch):
    """让 `_find_cloudflared()` 一定找不到二进制。"""
    missing = tmp_path / "cloudflared.exe"
    monkeypatch.setattr(tunnel, "CLOUDFLARED_PATH", str(missing))
    monkeypatch.setattr(tunnel, "APP_DIR", str(tmp_path))
    tunnel.stop_tunnel()
    tunnel._state["error"] = ""
    tunnel._state["error_detail"] = ""
    tunnel._state["download"] = {"status": "idle", "error": "", "bytes": 0}
    yield missing
    tunnel.stop_tunnel()


def test_missing_binary_reports_error_instead_of_silent_return(no_binary):
    """没有 cloudflared 时必须给出 cloudflared_missing，而不是静默返回。"""
    ok, error = tunnel.start_tunnel()
    assert ok is False
    assert error == "cloudflared_missing"

    state = tunnel.tunnel_state()
    assert state["error"] == "cloudflared_missing"
    assert state["installed"] is False
    assert state["running"] is False
    assert state["url"] == ""


def test_state_shape_is_stable(no_binary):
    """界面依赖这些 key，缺一个就会让 UI 崩或显示 undefined。"""
    state = tunnel.tunnel_state()
    for key in (
        "running",
        "url",
        "mode",
        "error",
        "errorDetail",
        "installed",
        "binaryVersion",
        "binaryPath",
        "download",
    ):
        assert key in state, f"tunnel_state() 缺少 {key}"
    assert set(state["download"]) >= {"status", "error", "bytes"}


def test_binary_version_is_empty_when_missing(no_binary):
    assert tunnel.binary_version() == ""


def test_download_rejects_truncated_payload(no_binary, monkeypatch):
    """下载到明显偏小的文件（代理拦截 / 404 页面）必须判失败，不能留下坏二进制。"""

    class FakeResp:
        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

        def read(self, _n):
            if getattr(self, "_done", False):
                return b""
            self._done = True
            return b"x" * 1024  # 只有 1 KB

    monkeypatch.setattr(tunnel.urllib.request, "urlopen", lambda *a, **k: FakeResp())
    monkeypatch.setattr(tunnel.urllib.request, "Request", lambda *a, **k: None)

    ok, error = tunnel.download_cloudflared()
    assert ok is False
    assert error == "download_failed"
    assert tunnel.tunnel_state()["download"]["status"] == "failed"
    # 不能留下 .part 或半截文件
    assert not no_binary.exists()
    assert not (no_binary.parent / "cloudflared.exe.part").exists()


def test_download_reports_progress_and_installs(no_binary, monkeypatch):
    """体积达标时应落盘，并把状态置成 done。"""
    payload = b"y" * (tunnel.MIN_BINARY_BYTES + 10)

    class FakeResp:
        def __init__(self):
            self._sent = 0

        def __enter__(self):
            return self

        def __exit__(self, *exc):
            return False

        def read(self, n):
            if self._sent >= len(payload):
                return b""
            chunk = payload[self._sent : self._sent + n]
            self._sent += len(chunk)
            return chunk

    monkeypatch.setattr(tunnel.urllib.request, "urlopen", lambda *a, **k: FakeResp())
    monkeypatch.setattr(tunnel.urllib.request, "Request", lambda *a, **k: None)

    ok, error = tunnel.download_cloudflared()
    assert (ok, error) == (True, "")
    assert no_binary.exists()
    assert no_binary.stat().st_size == len(payload)
    state = tunnel.tunnel_state()
    assert state["download"]["status"] == "done"
    assert state["download"]["bytes"] == len(payload)
    assert state["installed"] is True
    assert state["error"] == ""


def test_url_timeout_is_reported(no_binary, monkeypatch):
    """cloudflared 起得来但一直不给 URL 时，必须超时报错而不是永远空等。"""
    monkeypatch.setattr(tunnel, "URL_WAIT_SECONDS", 1)

    class LogOnlyProc:
        """立刻退出，让 `_parse_tunnel_url` 走 exited 分支。"""

        def poll(self):
            return 1

        def terminate(self):
            pass

        def wait(self, timeout=None):
            return 0

        def kill(self):
            pass

    tunnel._parse_tunnel_url(str(no_binary.parent / "nope.log"), LogOnlyProc())
    state = tunnel.tunnel_state()
    assert state["error"] == "exited"
    assert "code=1" in state["errorDetail"]


def test_stop_and_restart_are_idempotent(no_binary):
    """反复 stop 不应抛异常；restart 在没有二进制时给出明确错误。"""
    tunnel.stop_tunnel()
    tunnel.stop_tunnel()
    ok, error = tunnel.restart_tunnel()
    assert ok is False
    assert error == "cloudflared_missing"


def test_remove_without_binary(no_binary):
    ok, error = tunnel.remove_cloudflared()
    assert ok is False
    assert error == "not_found"
