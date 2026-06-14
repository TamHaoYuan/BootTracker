#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 - 桌面应用 (后台常驻版)

架构：
- HTTP 服务器常驻后台，端口 18792
- 开机时 Python 后端自动关闭上一次未结束的会话、创建新会话
- 每次写入数据前自动备份到 backup/ 目录，保留最近 30 份
- 关闭窗口后进程继续运行，再次打开快捷方式即可恢复
- 支持 --background 参数：无窗口静默启动（用于开机自启）
- 注册表自启项在首次启动时自动写入
"""
import os
import sys
import json
import socket
import uuid
import shutil
import threading
from datetime import datetime, timezone
from http.server import HTTPServer, BaseHTTPRequestHandler

PORT = 18792
APP_DIR = os.path.dirname(os.path.abspath(__file__))
HTML_FILE = os.path.join(APP_DIR, "index.html")
DATA_FILE = os.path.join(APP_DIR, "boot-data.json")
BACKUP_DIR = os.path.join(APP_DIR, "backup")
MAX_BACKUPS = 30


def _ensure_backup_dir():
    if not os.path.isdir(BACKUP_DIR):
        os.makedirs(BACKUP_DIR, exist_ok=True)


def backup_data():
    """备份当前数据文件到 backup/ 目录，保留最近 MAX_BACKUPS 份"""
    try:
        if not os.path.exists(DATA_FILE):
            return
        _ensure_backup_dir()
        ts = datetime.now().strftime("%Y%m%d-%H%M%S")
        dest = os.path.join(BACKUP_DIR, f"boot-data-{ts}.json")
        shutil.copy2(DATA_FILE, dest)
        _cleanup_old_backups()
    except Exception:
        pass


def _cleanup_old_backups():
    """删除最旧的备份，只保留最近 MAX_BACKUPS 份"""
    try:
        files = [f for f in os.listdir(BACKUP_DIR) if f.startswith("boot-data-") and f.endswith(".json")]
        if len(files) <= MAX_BACKUPS:
            return
        files.sort(reverse=True)  # 最新的在前
        for f in files[MAX_BACKUPS:]:
            os.remove(os.path.join(BACKUP_DIR, f))
    except Exception:
        pass


def load_data():
    """读取数据文件"""
    try:
        if os.path.exists(DATA_FILE):
            with open(DATA_FILE, "r", encoding="utf-8") as f:
                return json.load(f)
    except Exception:
        pass
    return {"bootCount": 0, "shutdownCount": 0, "sessions": []}


def save_data(data):
    """写入数据文件（写入前自动备份）"""
    try:
        backup_data()
        with open(DATA_FILE, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
    except Exception:
        pass


def auto_close_and_new_session():
    """开机时自动关闭上一次的活跃会话，创建本次开机会话。

    此函数仅在服务器首次启动时调用（即真正的开机时刻），
    因此遇到未关闭的会话必定是上一次关机前遗漏的。
    """
    data = load_data()
    changed = False
    now = datetime.now(timezone.utc)

    # 关闭所有未结束的会话（上一次开机遗留）
    for s in data.get("sessions", []):
        if not s.get("shutdownTime"):
            try:
                boot = datetime.fromisoformat(
                    s["bootTime"].replace("Z", "+00:00")
                )
                s["shutdownTime"] = now.isoformat().replace("+00:00", "Z")
                s["duration"] = int((now - boot).total_seconds() * 1000)
                data["shutdownCount"] = data.get("shutdownCount", 0) + 1
                changed = True
            except Exception:
                pass

    # 创建本次开机会话
    new_id = uuid.uuid4().hex[:14]
    data["bootCount"] = data.get("bootCount", 0) + 1
    data["sessions"].append({
        "id": new_id,
        "bootTime": now.isoformat().replace("+00:00", "Z"),
        "shutdownTime": None,
        "duration": None,
    })
    changed = True

    if changed:
        save_data(data)


class DataHandler(BaseHTTPRequestHandler):
    """HTTP 接口：JS 通过 fetch 与之通信"""

    data_file = ""
    _server = None
    _stop_event = None

    def do_GET(self):
        if self.path == "/api/data":
            self._json_response(load_data())
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
                save_data(data)
                self._json_response({"ok": True})
            except Exception as e:
                self._json_response({"error": str(e)}, 400)
        elif self.path == "/api/clear":
            save_data({"bootCount": 0, "shutdownCount": 0, "sessions": []})
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
            _ensure_backup_dir()
            files = [f for f in os.listdir(BACKUP_DIR) if f.startswith("boot-data-") and f.endswith(".json")]
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
            cmd = '"' + pythonw + '" "' + os.path.abspath(__file__) + '" --background'
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

    url = HTML_FILE + "#port=" + str(PORT)
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
    if not os.path.exists(HTML_FILE):
        sys.exit(1)

    background = "--background" in sys.argv

    if is_port_in_use(PORT):
        # 服务器已在运行，直接打开窗口
        if not background:
            open_window()
        return

    # 首次启动（真正的开机）：自动处理上次会话 + 创建本次开机会话
    auto_close_and_new_session()

    # 启动 HTTP 服务器
    server = HTTPServer(("127.0.0.1", PORT), DataHandler)
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


if __name__ == "__main__":
    main()
