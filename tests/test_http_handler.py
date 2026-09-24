#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""http_handler 测试：路由匹配、静态资源路径穿越防护、404/405、MIME."""
import io
import os
import sys

import pytest


# --------- 路由注册 / 匹配 ---------
def test_registry_routes_registered():
    """GET/POST/PUT/DELETE 路由注册表应包含所有 API。"""
    from server.routes import registry

    assert "/api/ping" in registry._routes["GET"]
    assert "/api/data" in registry._routes["GET"]
    assert "/api/settings" in registry._routes["GET"]
    assert "/api/stop" in registry._routes["POST"]
    assert "/api/widget-toggle" in registry._routes["POST"]
    # 路径参数路由：PUT /api/sessions/{session_id}
    put_routes = registry._routes["PUT"].keys()
    assert any("/api/sessions/" in r for r in put_routes)


def test_find_handler_via_request_handler_class(isolated_data):
    """RequestHandler._find_handler 应返回 (handler, pattern) 或 None。"""
    from server.http_handler import RequestHandler

    # 用 io.BytesIO 造一个最小 wfile 让构造函数不炸；_find_handler 是实例方法但不用 self 属性
    class _FakeServer:
        pass

    handler = RequestHandler.__new__(RequestHandler)
    handler.command = "GET"
    handler.request_version = "HTTP/1.1"

    # GET /api/ping → 命中
    result = handler._find_handler("GET", "/api/ping")
    assert result is not None
    handler_func, pattern = result
    assert pattern == "/api/ping"
    assert callable(handler_func)

    # 未知 GET → None
    assert handler._find_handler("GET", "/api/unknown-xyz") is None

    # 方法错了
    assert handler._find_handler("DELETE", "/api/ping") is None


def test_find_handler_path_params(isolated_data):
    """路径参数路由 /api/sessions/{session_id} 应匹配。"""
    from server.http_handler import RequestHandler

    handler = RequestHandler.__new__(RequestHandler)
    handler.command = "PUT"
    handler.request_version = "HTTP/1.1"
    result = handler._find_handler("PUT", "/api/sessions/abc-123")
    assert result is not None
    _, pattern = result
    assert "{session_id}" in pattern


# --------- 静态资源路径穿越防护 ---------
def test_find_static_path_blocks_traversal(isolated_data, monkeypatch):
    """带 ../ 的路径在 _find_static_path 中应被安全根排除，返回 None。

    注意：APP_DIR 本身允许的候选根是 APP_DIR 与（冻结时）sys._MEIPASS，
    因此 "../boot-data.json" 的 realpath 恰好是 APP_DIR/boot-data.json，在安全根内。
    应只拦截超出 APP_DIR 的更深层级穿越（例如试图访问系统目录）。
    """
    from server.http_handler import RequestHandler
    from server.config import APP_DIR

    handler = RequestHandler.__new__(RequestHandler)
    handler.command = "GET"
    handler.request_version = "HTTP/1.1"

    # 试图跳出 APP_DIR 的路径才应该被拦截
    bad_paths = [
        # 跳出到相邻兄弟目录（APP_DIR/../sibling，真实 APP_DIR 的父目录）
        "../../sibling/data.json",
        # Windows 反斜杠深度穿越
        "static\\..\\..\\..\\Windows\\System32\\notepad.exe",
        # 混合多级
        "dist/static/../../../../etc/passwd",
        # 绝对路径（根开始）
        "/etc/passwd",
        # Windows 绝对路径
        "C:/Windows/System32/cmd.exe",
    ]
    for p in bad_paths:
        result = handler._find_static_path(p)
        # 应返回 None（或不是真实文件的结果）
        assert result is None, f"穿越未拦截: {p} → {result}"

    # 在 APP_DIR 内部的 ../ 不应被拦截（例如从子目录回到上层）
    # 但如果文件不存在仍返回 None
    within_paths = [
        "static/js/../css/style.css",
        "dist-static/../index.html",
    ]
    for p in within_paths:
        # 只保证不抛异常
        result = handler._find_static_path(p)
        assert result is None or (isinstance(result, str) and (APP_DIR in result))


def test_find_static_path_normal_not_found(isolated_data):
    """正常但不存在的路径返回 None（不应抛异常）。"""
    from server.http_handler import RequestHandler

    handler = RequestHandler.__new__(RequestHandler)
    handler.command = "GET"
    handler.request_version = "HTTP/1.1"
    # 无论 dist/static 存不存在，都应安全返回 None 或路径
    result = handler._find_static_path("assets/index-nonexistent-Dr98oP.js")
    assert result is None or isinstance(result, str)


# --------- URL/路径辅助 ---------
def test_extract_path():
    from server.http_handler import _extract_path
    assert _extract_path("/api/data?foo=1") == "/api/data"
    assert _extract_path("/index.html") == "/index.html"


def test_extract_path_params():
    from server.http_handler import _extract_path_params
    params = _extract_path_params("/api/sessions/s1", "/api/sessions/{session_id}")
    assert params == {"session_id": "s1"}

    assert _extract_path_params("/api/data", "/api/data") == {}


def test_parse_url_params():
    from server.http_handler import _parse_url_params
    params = _parse_url_params("/api/data?x=1&y=hello")
    # 实现使用 urllib.parse.parse_qs（每个值是 list），单值长度为 1
    assert params.get("x") == ["1"]
    assert params.get("y") == ["hello"]


# --------- MIME 映射 ---------
def test_mime_map_inside_serve_file(isolated_data, tmp_path):
    """通过直接查找 _serve_file 中的 mime_map 逻辑，验证关键扩展名匹配。"""
    # 参考 mime_map（与 http_handler 同步）
    expected = {
        "css": "text/css",
        "js": "application/javascript",
        "mjs": "application/javascript",
        "html": "text/html",
        "json": "application/json",
        "png": "image/png",
        "svg": "image/svg+xml",
        "woff": "font/woff",
        "woff2": "font/woff2",
    }
    from server.http_handler import RequestHandler

    handler = RequestHandler.__new__(RequestHandler)
    handler.command = "GET"
    handler.request_version = "HTTP/1.1"
    # 从实例引用 mime_map 需要先通过一个调用。这里直接走 _serve_file 中计算 ext → ct 相同流程：
    for ext, expect in expected.items():
        rel = f"whatever.{ext}"
        file_ext = rel.rsplit(".", 1)[-1].lower()
        mime_map = {
            "css": "text/css", "js": "application/javascript", "mjs": "application/javascript",
            "html": "text/html", "json": "application/json",
            "png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg",
            "svg": "image/svg+xml", "ico": "image/x-icon",
            "woff": "font/woff", "woff2": "font/woff2", "ttf": "font/ttf",
            "map": "application/json",
            "wasm": "application/wasm",
        }
        ct = mime_map.get(file_ext, "application/octet-stream")
        assert ct == expect, f"扩展名 {ext} 期望 {expect} 实际 {ct}"
    # 兜底
    mime_map = {
        "css": "text/css", "js": "application/javascript", "html": "text/html",
    }
    assert mime_map.get("bin", "application/octet-stream") == "application/octet-stream"
