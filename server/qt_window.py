#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""基于 PyQt5 WebEngine 的主界面窗口模块

提供主界面窗口的创建与控制 helper：
- is_qt_available() / open_main_window(port) / run_qt_event_loop()
- show_main_window() / quit_app()（线程安全，跨线程投递到 Qt 主线程）

设计要点：
- QApplication 必须在主线程创建并运行 exec_()；本模块由调用方（boot-tracker.py）
  在主线程调用 open_main_window → run_qt_event_loop。
- 跨线程的“显示/退出”请求通过 pyqtSignal 跨线程 emit 实现：信号接收对象（_WindowController）
  位于主线程，emit 时 Qt 自动用 Qt.QueuedConnection 把槽调用排队到主线程事件循环执行。
  注意：不能从非 Qt 线程调用 QTimer.singleShot——QTimer 亲和于创建它的线程，后台线程
  没有事件循环会导致回调永不触发。
- 不再内嵌 QSystemTrayIcon（pystray 已负责托盘，避免双托盘）。
- 加载 http://127.0.0.1:{port}/ 而非 file://，确保前端 fetch 走相对路径正常工作。
"""
import os
import sys
import threading
import logging

# WebEngine 在 PyInstaller frozen 环境下，Chromium sandbox 初始化失败会导致窗口无法显示；
# 部分显卡的 GPU 合成也可能失败。仅 frozen 模式下设置：
#   --no-sandbox            sandbox 在 frozen 进程结构下无法初始化
#   --disable-gpu-compositing 仅禁用 GPU 合成路径（保留 GPU 渲染，比 --disable-gpu 流畅）
# 不设置 --enable-logging（会产生 stderr/控制台输出）。开发模式不禁用 GPU 以保持流畅。
if sys.platform == "win32" and getattr(sys, "frozen", False):
    os.environ.setdefault(
        "QTWEBENGINE_CHROMIUM_FLAGS",
        "--no-sandbox --disable-gpu-compositing",
    )
    os.environ.setdefault("QTWEBENGINE_DISABLE_SANDBOX", "1")

logger = logging.getLogger(__name__)

# 模块级单例：QApplication、主窗口、跨线程控制器
_qt_app = None
_main_window = None
_controller = None
_init_lock = threading.Lock()


class _WindowController:
    """跨线程窗口控制器：通过 pyqtSignal 把请求排队到主线程执行。

    延迟到 open_main_window（主线程）内部才实例化与连接信号，
    以保证 QObject 的线程亲和性为主线程。
    """

    def __init__(self, app, window):
        from PyQt5.QtCore import QObject, pyqtSignal

        # 动态定义带信号的 QObject 子类（在主线程）
        class _Signals(QObject):
            raise_requested = pyqtSignal()
            quit_requested = pyqtSignal()
            theme_requested = pyqtSignal(str)

        self._app = app
        self._window = window
        self._signals = _Signals()
        # 槽属于 _signals（主线程 QObject），跨线程 emit 自动用 QueuedConnection
        self._signals.raise_requested.connect(self._do_raise)
        self._signals.quit_requested.connect(self._app.quit)
        self._signals.theme_requested.connect(self._apply_theme)

    def request_raise(self):
        self._signals.raise_requested.emit()

    def request_quit(self):
        self._signals.quit_requested.emit()

    def request_theme(self, mode):
        self._signals.theme_requested.emit(mode)

    def apply_theme_now(self, mode):
        """主线程同步应用主题（供 open_main_window 初始化时直接调用）"""
        self._apply_theme(mode)

    def _apply_theme(self, mode):
        """主线程：同步 WebEngine 背景色与 Windows 标题栏到当前模式"""
        dark = mode != "light"
        try:
            from PyQt5.QtGui import QColor
            bg = QColor("#f8fafc") if not dark else QColor("#0a0a0c")
            page = self._window.centralWidget().page()
            page.setBackgroundColor(bg)
        except Exception as e:
            logger.error(f"[qt] set page background failed: {e}")
        self._apply_dwm_titlebar(dark)

    def _apply_dwm_titlebar(self, dark):
        """Windows：设置沉浸式暗色/浅色标题栏（DWMWA_USE_IMMERSIVE_DARK_MODE）"""
        import sys
        if sys.platform != "win32":
            return
        try:
            import ctypes
            hwnd = int(self._window.winId())
            val = ctypes.c_int(1 if dark else 0)
            # 属性 20（Win10 2004+/Win11）；旧版本为 19
            for attr in (20, 19):
                ok = ctypes.windll.dwmapi.DwmSetWindowAttribute(
                    hwnd, attr, ctypes.byref(val), ctypes.sizeof(val)
                )
                if ok == 0:
                    break
        except Exception as e:
            logger.error(f"[qt] dwm titlebar failed: {e}")

    def _do_raise(self):
        try:
            w = self._window
            if w.isMinimized():
                w.showNormal()
            w.show()
            w.raise_()
            w.activateWindow()
            try:
                from PyQt5.QtWidgets import QApplication
                QApplication.setActiveWindow(w)
            except Exception:
                pass
        except Exception as e:
            logger.error(f"[qt] raise window failed: {e}")


def is_qt_available() -> bool:
    """检测 PyQt5 + QtWebEngine 是否可用（不影响导入本模块）"""
    try:
        import PyQt5.QtWidgets  # noqa: F401
        import PyQt5.QtWebEngineWidgets  # noqa: F401
        return True
    except Exception as e:
        logger.info(f"[qt] PyQt5 unavailable: {e}")
        return False


def open_main_window(port: int) -> bool:
    """在主线程初始化 QApplication + 主窗口。成功返回 True。

    若 PyQt5 不可用返回 False（调用方应回退到系统浏览器）。
    重复调用时直接唤起已有窗口。
    """
    global _qt_app, _main_window, _controller
    with _init_lock:
        if _qt_app is not None:
            if _controller is not None:
                _controller.request_raise()
            return True

        try:
            from PyQt5.QtWidgets import QApplication, QMainWindow
            from PyQt5.QtWebEngineWidgets import QWebEngineView
            from PyQt5.QtCore import QUrl
        except Exception as e:
            logger.error(f"[qt] PyQt5 import failed: {e}")
            return False

        # 设置 AppUserModelID，使任务栏将本应用识别为独立进程（而非 python.exe），
        # 从而正确显示自定义图标
        try:
            import ctypes
            ctypes.windll.shell32.SetCurrentProcessExplicitAppUserModelID(
                "boottracker.app"
            )
        except Exception:
            pass

        _qt_app = QApplication.instance() or QApplication(sys.argv)
        _qt_app.setApplicationName("开机记录")

        # 应用图标（与托盘一致：显示器 + 紫色进度弧，避免默认 Python 图标）
        try:
            from io import BytesIO
            from PyQt5.QtGui import QIcon, QPixmap
            from PyQt5.QtCore import QByteArray
            from .tray import draw_icon_image
            pil_img = draw_icon_image()
            buf = BytesIO()
            pil_img.save(buf, format="PNG")
            pix = QPixmap()
            pix.loadFromData(QByteArray(buf.getvalue()), "PNG")
            _qt_app.setWindowIcon(QIcon(pix))
        except Exception as e:
            logger.error(f"[qt] set window icon failed: {e}")

        class MainWindow(QMainWindow):
            def closeEvent(self, event):
                # 关闭即最小化到托盘，不退出应用
                event.ignore()
                self.hide()

        w = MainWindow()
        w.setWindowTitle("开机记录")
        w.setGeometry(100, 100, 960, 760)
        w.setMinimumSize(700, 500)

        view = QWebEngineView()
        url = f"http://127.0.0.1:{port}/"
        logger.info(f"[qt] loading {url}")
        view.load(QUrl(url))
        # 允许本地内容访问远程 URL（兼容性保留）
        try:
            settings = view.settings()
            settings.setAttribute(settings.LocalContentCanAccessFileUrls, True)
            settings.setAttribute(settings.LocalContentCanAccessRemoteUrls, True)
        except Exception:
            pass
        w.setCentralWidget(view)

        _main_window = w
        # 在主线程创建控制器（保证信号接收对象亲和主线程）
        _controller = _WindowController(_qt_app, w)
        w.show()
        # 读取已保存的模式，初始化 WebEngine 背景色与标题栏（避免白屏闪烁与标题栏不匹配）
        mode = "dark"
        try:
            from .settings import load_settings
            mode = load_settings().get("appMode", "dark")
        except Exception:
            pass
        _controller.apply_theme_now(mode)
        return True


def show_main_window() -> bool:
    """线程安全地唤起已存在的主窗口。无 Qt 应用时返回 False（调用方回退到浏览器）。"""
    if _qt_app is None or _main_window is None or _controller is None:
        return False
    _controller.request_raise()
    return True


def quit_app() -> bool:
    """线程安全地触发 app.quit()，使主线程 exec_() 返回。无 Qt 应用时返回 False。"""
    if _qt_app is None or _controller is None:
        return False
    _controller.request_quit()
    return True


def set_window_theme(mode: str) -> bool:
    """线程安全地切换窗口主题（'dark'/'light'）：同步 WebEngine 背景色与标题栏。

    无 Qt 应用时返回 False（如系统浏览器模式，前端自管主题，无需处理）。
    """
    if _qt_app is None or _controller is None:
        return False
    _controller.request_theme(mode)
    return True


def run_qt_event_loop() -> None:
    """阻塞主线程直到 app.quit()。须在 open_main_window 之后由主线程调用。"""
    if _qt_app is None:
        raise RuntimeError("Qt app not initialized; call open_main_window first")
    _qt_app.exec_()
