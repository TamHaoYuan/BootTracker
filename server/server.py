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
        elif self.path == "/api/trash":
            self._json_response(D.load_trash())
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
        elif self.path == "/api/trash/restore":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                req = json.loads(body)
                self._trash_restore(req.get("id"))
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
        elif self.path == "/api/trash/clear":
            D.save_trash({"sessions": []})
            self._json_response({"ok": True})
        elif self.path == "/api/trash":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                trash = json.loads(body)
                D.save_trash(trash)
                self._json_response({"ok": True})
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
        elif self.path == "/api/add-session":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            try:
                req = json.loads(body)
                self._add_session(req)
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
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

    def _trash_restore(self, session_id):
        """从回收站恢复记录到主数据"""
        if not session_id:
            self._json_response({"error": "missing id"}, 400)
            return
        trash = D.load_trash()
        idx = next((i for i, s in enumerate(trash.get("sessions", [])) if s.get("id") == session_id), None)
        if idx is None:
            self._json_response({"error": "not found"}, 404)
            return
        session = trash["sessions"].pop(idx)
        data = D.load_data()
        data["sessions"].append(session)
        # 按开机时间排序
        data["sessions"].sort(key=lambda s: s.get("bootTime", ""))
        # 重新统计
        data["bootCount"] = len(data["sessions"])
        data["shutdownCount"] = len([s for s in data["sessions"] if s.get("shutdownTime")])
        D.save_trash(trash)
        D.save_data(data)
        self._json_response({"ok": True})

    def _add_session(self, req):
        """手动添加一条开机记录"""
        boot_time = req.get("bootTime")
        shutdown_time = req.get("shutdownTime")
        if not boot_time:
            self._json_response({"error": "missing bootTime"}, 400)
            return
        data = D.load_data()
        import uuid
        session = {
            "id": uuid.uuid4().hex[:14],
            "bootTime": boot_time,
            "shutdownTime": shutdown_time or None,
            "duration": None,
        }
        if shutdown_time:
            try:
                from datetime import datetime
                bt = datetime.fromisoformat(boot_time.replace("Z", "+00:00"))
                st = datetime.fromisoformat(shutdown_time.replace("Z", "+00:00"))
                session["duration"] = int((st - bt).total_seconds() * 1000)
            except Exception:
                pass
        data["sessions"].append(session)
        data["sessions"].sort(key=lambda s: s.get("bootTime", ""))
        data["bootCount"] = len(data["sessions"])
        data["shutdownCount"] = len([s for s in data["sessions"] if s.get("shutdownTime")])
        D.save_data(data)
        self._json_response({"ok": True, "session": session})

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
