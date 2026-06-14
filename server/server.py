#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""HTTP 服务器 + 启动入口模块 — 负责路由、窗口管理和进程控制"""
import os
import sys
import json
import socket
import threading
from http.server import HTTPServer, BaseHTTPRequestHandler

from . import data as D


class DataHandler(BaseHTTPRequestHandler):
    """HTTP 接口：JS 通过 fetch 与之通信"""

    _server = None
    _stop_event = None

    def do_GET(self):
        if self.path == "/api/data":
            self._json_response(D.load_data())
        elif self.path == "/api/ping":
            self._json_response({"ok": True})
        elif self.path == "/api/stop":
            self._json_response({"ok": True})
            if self._stop_event:
                self._stop_event.set()
            if self._server:
                threading.Thread(target=self._server.shutdown, daemon=True).start()
        elif self.path == "/api/backups":
            self._list_backups()
        else:
            self._json_response({"error": "not found"}, 404)

    def do_POST(self):
        if self.path == "/api/data":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                data = json.loads(body)
                D.save_data(data)
                self._json_response({"ok": True})
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
        elif self.path == "/api/clear":
            D.save_data({"bootCount": 0, "shutdownCount": 0, "sessions": []})
            self._json_response({"ok": True})
        elif self.path == "/api/stop":
            self._json_response({"ok": True})
            if self._stop_event:
                self._stop_event.set()
            if self._server:
                threading.Thread(target=self._server.shutdown, daemon=True).start()
        else:
            self._json_response({"error": "not found"}, 404)

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def _list_backups(self):
        """列出可用备份"""
        try:
            D._ensure_backup_dir()
            files = [
                f for f in os.listdir(D.BACKUP_DIR)
                if f.startswith("boot-data-") and f.endswith(".json")
            ]
            files.sort(reverse=True)
            self._json_response({"backups": files})
        except Exception:
            self._json_response({"backups": []})

    def _json_response(self, obj, code=200):
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(json.dumps(obj, ensure_ascii=False).encode("utf-8"))

    def log_message(self, format, *args):
        pass


def is_port_in_use(port):
    """检测端口是否已被占用（说明后台服务已在运行）"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        try:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            s.bind(("127.0.0.1", port))
            return False
        except OSError:
            return True


def setup_autostart(enable=True):
    """设置/取消开机自启（Windows 注册表）"""
    try:
        import winreg
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


def open_window():
    """打开 pywebview 窗口（阻塞直到窗口关闭）"""
    import webview

    url = D.HTML_FILE + "#port=" + str(D.PORT)
    window = webview.create_window(
        title="开机记录",
        url=url,
        width=900,
        height=720,
        min_size=(700, 500),
        resizable=True,
        text_select=False,
    )
    webview.start()


def main():
    """主入口函数"""
    if not os.path.exists(D.HTML_FILE):
        sys.exit(1)

    background = "--background" in sys.argv

    if is_port_in_use(D.PORT):
        # 服务器已在运行，直接打开窗口
        if not background:
            open_window()
        return

    # 首次启动（真正的开机）：自动处理上次会话 + 创建本次开机会话
    D.auto_close_and_new_session()

    # 启动 HTTP 服务器
    server = HTTPServer(("127.0.0.1", D.PORT), DataHandler)
    DataHandler._server = server
    stop_event = threading.Event()
    DataHandler._stop_event = stop_event

    server_thread = threading.Thread(target=server.serve_forever, daemon=False)
    server_thread.start()

    # 写入开机自启注册表
    setup_autostart(True)

    if background:
        stop_event.wait()
    else:
        open_window()
        server.shutdown()
