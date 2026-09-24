#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""HTTP 请求处理模块 - HTTP 服务器和路由分发"""

import os
import sys
import json
import socket
import threading
import urllib.parse
import shutil
import re
import subprocess
from http.server import HTTPServer, BaseHTTPRequestHandler
from typing import Dict, Any, Optional

from .config import HTML_FILE, APP_DIR, UPLOAD_DIR, PORT
from .logging_config import logger
from .routes import registry


_server = None
_stop_event = None
_lock_socket = None


def _parse_url_params(path: str) -> Dict[str, str]:
    """解析 URL 参数"""
    parsed = urllib.parse.urlparse(path)
    return urllib.parse.parse_qs(parsed.query)


def _extract_path(path: str) -> str:
    """提取路径（去除查询参数）"""
    if '?' in path:
        return path.split('?', 1)[0]
    return path


def _extract_path_params(path: str, route_pattern: str) -> Dict[str, str]:
    """提取路径参数"""
    path_parts = path.split('/')
    pattern_parts = route_pattern.split('/')
    params = {}
    
    for p, r in zip(path_parts, pattern_parts):
        if r.startswith('{') and r.endswith('}'):
            params[r[1:-1]] = p
    
    return params


class RequestHandler(BaseHTTPRequestHandler):
    """HTTP 请求处理器"""
    
    def do_GET(self):
        """处理 GET 请求"""
        path = _extract_path(self.path)
        
        if path in ("/", "/index.html"):
            self._serve_index()
            return
        
        if path.startswith("/api/"):
            self._handle_api("GET", path)
            return
        
        self._serve_file(self.path)
    
    def do_POST(self):
        """处理 POST 请求"""
        path = _extract_path(self.path)
        
        if path == "/api/upload-bg":
            self._upload_bg_image()
            return
        
        if path == "/api/restart-server":
            self._restart_server()
            return
        
        if path.startswith("/api/"):
            self._handle_api("POST", path)
            return
        
        self._json_response({"error": "not found"}, 404)
    
    def do_OPTIONS(self):
        """处理 OPTIONS 请求（CORS）"""
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()
    
    def do_PUT(self):
        """处理 PUT 请求"""
        path = _extract_path(self.path)
        
        if path.startswith("/api/"):
            self._handle_api("PUT", path)
            return
        
        self._json_response({"error": "not found"}, 404)
    
    def do_DELETE(self):
        """处理 DELETE 请求"""
        path = _extract_path(self.path)
        
        if path.startswith("/api/"):
            self._handle_api("DELETE", path)
            return
        
        self._json_response({"error": "not found"}, 404)
    
    def _handle_api(self, method: str, path: str) -> None:
        """处理 API 请求"""
        try:
            body = self._parse_body()
            # 合并 URL 查询参数（GET 请求无 body，查询参数需手动解析）
            query_params = _parse_url_params(path)
            for k, v in query_params.items():
                if len(v) == 1:
                    body.setdefault(k, v[0])
                else:
                    body.setdefault(k, v)
            result = self._find_handler(method, path)

            if result:
                handler, route_pattern = result
                params = _extract_path_params(path, route_pattern)
                body.update(params)
                code, response = handler(self, body)
                self._json_response(response, code)
            else:
                self._json_response({"error": "not found"}, 404)
        except Exception as e:
            logger.error(f"API error: {e}")
            self._json_response({"error": str(e)}, 500)

    def _find_handler(self, method: str, path: str):
        """查找路由处理器，返回 (handler, route_pattern) 或 None"""
        method_routes = registry._routes.get(method, {})

        if path in method_routes:
            return method_routes[path], path

        for route_path in method_routes:
            if '{' in route_path:
                if self._matches_pattern(path, route_path):
                    return method_routes[route_path], route_path

        return None
    
    def _matches_pattern(self, path: str, pattern: str) -> bool:
        """检查路径是否匹配模式"""
        path_parts = path.split('/')
        pattern_parts = pattern.split('/')
        
        if len(path_parts) != len(pattern_parts):
            return False
        
        for p, r in zip(path_parts, pattern_parts):
            if r.startswith('{') and r.endswith('}'):
                continue
            if p != r:
                return False
        
        return True
    
    def _parse_body(self) -> Dict[str, Any]:
        """解析请求体"""
        length = int(self.headers.get("Content-Length", 0))
        if length == 0:
            return {}
        
        body = self.rfile.read(length)
        
        content_type = self.headers.get("Content-Type", "")
        if content_type.startswith("application/json"):
            try:
                return json.loads(body)
            except Exception:
                return {}
        
        return {}
    
    def _upload_bg_image(self):
        """上传背景图片"""
        try:
            content_type = self.headers.get("Content-Type", "")
            if not content_type.startswith("multipart/form-data"):
                self._json_response({"error": "invalid content type"}, 400)
                return
            
            boundary = content_type.split("boundary=")[1].encode('utf-8')
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            
            parts = body.split(boundary)
            for part in parts:
                if b"Content-Disposition" in part and b"filename" in part:
                    filename_match = re.search(rb'filename="([^"]+)"', part)
                    if filename_match:
                        filename = filename_match.group(1).decode('utf-8')
                        ext = os.path.splitext(filename)[1].lower()
                        if ext not in ('.jpg', '.jpeg', '.png', '.gif', '.webp'):
                            self._json_response({"error": "unsupported image format"}, 400)
                            return
                        
                        data_start = part.find(b"\r\n\r\n") + 4
                        data_end = part.rfind(b"\r\n--")
                        image_data = part[data_start:data_end]
                        
                        os.makedirs(UPLOAD_DIR, exist_ok=True)
                        
                        bg_path = os.path.join(UPLOAD_DIR, "bg.jpg")
                        with open(bg_path, "wb") as f:
                            f.write(image_data)
                        
                        from .settings import load_settings, save_settings
                        settings = load_settings()
                        settings["customBgImage"] = "/static/uploads/bg.jpg"
                        save_settings(settings)
                        
                        self._json_response({"ok": True, "path": "/static/uploads/bg.jpg"})
                        return
            
            self._json_response({"error": "no file uploaded"}, 400)
        except Exception as e:
            logger.error(f"upload bg error: {e}")
            self._json_response({"error": str(e)}, 500)
    
    def _restart_server(self):
        """重启服务器"""
        self._json_response({"ok": True, "message": "server restarting"})
        self.wfile.flush()
        
        def _do_restart():
            import time as _time
            _time.sleep(1.0)
            logger.info("[restart] restarting server via subprocess")
            
            if _lock_socket:
                try:
                    _lock_socket.close()
                    logger.info("[restart] instance lock released")
                except Exception:
                    pass
            
            env = dict(os.environ)
            env['_BOOTTRACKER_RESTART'] = '1'
            subprocess.Popen(
                [sys.executable] + sys.argv,
                cwd=APP_DIR,
                env=env,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0,
            )
            _time.sleep(1.0)
            logger.info("[restart] force exiting old process")
            os._exit(0)
        
        threading.Thread(target=_do_restart, daemon=True).start()
    
    def _serve_index(self):
        """提供首页 — 优先 dist-static/ 下的构建产物，否则回退旧版 index.html"""
        # 查找顺序：1) APP_DIR/dist-static/index.html  2) HTML_FILE（根 index.html，旧版）
        base_dir = getattr(sys, '_MEIPASS', APP_DIR) if getattr(sys, 'frozen', False) else APP_DIR
        candidates = [
            os.path.join(base_dir, 'dist-static', 'index.html'),
            os.path.join(APP_DIR, 'dist-static', 'index.html'),
            HTML_FILE,
        ]
        picked = None
        for c in candidates:
            if os.path.isfile(c):
                picked = c
                break
        if picked is None:
            self._json_response({"error": "index not found"}, 404)
            return
        try:
            with open(picked, "r", encoding="utf-8") as f:
                content = f.read()
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(content.encode("utf-8"))
        except Exception as e:
            logger.error(f"serve_index error: {e}")
            self._json_response({"error": str(e)}, 500)

    def _find_static_path(self, rel: str) -> Optional[str]:
        """在多个静态资源根目录里查找文件，返回首个命中的绝对路径，否则 None。

        查找顺序（冻结/开发通用）：
          1. <res_dir>/dist-static/<rel>      — 前端构建产物
          2. <APP_DIR>/dist-static/<rel>
          3. <res_dir>/static/<rel>           — 旧版 static/
          4. <APP_DIR>/static/<rel>
          5. <res_dir>/<rel>                  — 根路径（上传文件等）
          6. <APP_DIR>/<rel>
        """
        res_dir = getattr(sys, '_MEIPASS', APP_DIR) if getattr(sys, 'frozen', False) else APP_DIR
        roots = [
            os.path.join(res_dir, 'dist-static'),
            os.path.join(APP_DIR, 'dist-static'),
            os.path.join(res_dir, 'static'),
            os.path.join(APP_DIR, 'static'),
            res_dir,
            APP_DIR,
        ]
        safe_roots = {os.path.realpath(r) for r in (res_dir, APP_DIR)}
        for root in roots:
            candidate = os.path.realpath(os.path.join(root, rel))
            # 路径穿越防护：candidate 必须位于某允许根目录内
            if not any(candidate.startswith(r) for r in safe_roots):
                continue
            if os.path.isfile(candidate):
                return candidate
        return None

    def _serve_file(self, path: str):
        """提供静态文件 — 从 dist-static/ 优先、static/ 回退"""
        if '?' in path:
            path = path.split('?', 1)[0]

        rel = path.lstrip("/")
        fpath = self._find_static_path(rel)
        if fpath is None:
            self._json_response({"error": "file not found"}, 404)
            return

        ext = rel.rsplit(".", 1)[-1].lower() if "." in rel else ""
        mime_map = {
            "css": "text/css", "js": "application/javascript", "mjs": "application/javascript",
            "html": "text/html", "json": "application/json",
            "png": "image/png", "jpg": "image/jpeg", "jpeg": "image/jpeg",
            "svg": "image/svg+xml", "ico": "image/x-icon",
            "woff": "font/woff", "woff2": "font/woff2", "ttf": "font/ttf",
            "map": "application/json",
            "wasm": "application/wasm",
        }
        ct = mime_map.get(ext, "application/octet-stream")

        try:
            with open(fpath, "rb") as f:
                content = f.read()
            self.send_response(200)
            self.send_header("Content-Type", ct + "; charset=utf-8" if ext in ("css", "js", "mjs", "html", "json") else ct)
            self.send_header("Access-Control-Allow-Origin", "*")
            # 静态资源带 hash（Vite 自动生成），可长缓存；HTML 与不带 hash 的则 no-cache
            if ext in ("html", ""):
                self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            else:
                self.send_header("Cache-Control", "public, max-age=31536000, immutable")
            self.end_headers()
            self.wfile.write(content)
        except Exception:
            self._json_response({"error": "file not found"}, 404)
    
    def _json_response(self, obj: Dict, code: int = 200) -> None:
        """发送 JSON 响应"""
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(obj, ensure_ascii=False).encode("utf-8"))
    
    def log_message(self, format, *args):
        """禁用默认日志"""
        pass


def start_server(bind_addr: str = "127.0.0.1", port: int = PORT) -> HTTPServer:
    """启动 HTTP 服务器"""
    global _server
    _server = HTTPServer((bind_addr, port), RequestHandler)
    server_thread = threading.Thread(target=_server.serve_forever, daemon=False)
    server_thread.start()
    logger.info(f"[server] HTTP server started on {bind_addr}:{port}")
    return _server


def set_stop_event(event: threading.Event) -> None:
    """设置停止事件"""
    global _stop_event
    _stop_event = event


def set_lock_socket(sock: socket.socket) -> None:
    """设置实例锁套接字"""
    global _lock_socket
    _lock_socket = sock


def shutdown_server() -> None:
    """关闭服务器"""
    if _server:
        _server.shutdown()
        logger.info("[server] HTTP server shutdown")
