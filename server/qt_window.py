#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""基于 PyQt5 的桌面窗口（使用 Chromium 引擎）"""
import os
import sys
import threading
import logging

logger = logging.getLogger(__name__)

def _open_window_qt(html_file, port):
    """使用 PyQt5 打开窗口"""
    try:
        from PyQt5.QtWidgets import (
            QApplication, QMainWindow, QWidget, QVBoxLayout, QPushButton,
            QMessageBox, QSystemTrayIcon, QMenu, QAction
        )
        from PyQt5.QtWebEngineWidgets import QWebEngineView
        from PyQt5.QtCore import QUrl, Qt, QTimer, QSize
        from PyQt5.QtGui import QIcon, QPixmap, QColor

        logger.info(f"Opening Qt window: {html_file}, port={port}")

        url = f"file:///{html_file}#port={port}"
        logger.info(f"URL: {url}")

        app = QApplication(sys.argv)
        app.setApplicationName("开机记录")
        app.setWindowIcon(QIcon())

        class MainWindow(QMainWindow):
            def __init__(self):
                super().__init__()
                self.setWindowTitle("开机记录")
                self.setGeometry(100, 100, 900, 720)
                self.setMinimumSize(700, 500)

                self.web_view = QWebEngineView()
                self.web_view.load(QUrl(url))
                self.web_view.settings().setAttribute(
                    self.web_view.settings().LocalContentCanAccessRemoteUrls, True
                )
                self.web_view.settings().setAttribute(
                    self.web_view.settings().LocalContentCanAccessFileUrls, True
                )

                self.setCentralWidget(self.web_view)

            def closeEvent(self, event):
                event.ignore()
                self.hide()

        window = MainWindow()
        window.show()

        def create_tray_icon():
            try:
                icon_path = os.path.join(
                    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                    "icon.ico"
                )
                if os.path.exists(icon_path):
                    icon = QIcon(icon_path)
                else:
                    pixmap = QPixmap(64, 64)
                    pixmap.fill(QColor(59, 130, 246))
                    icon = QIcon(pixmap)

                tray_icon = QSystemTrayIcon(icon, app)
                tray_icon.setToolTip("开机记录")

                menu = QMenu()
                show_action = QAction("显示窗口", app)
                show_action.triggered.connect(window.show)
                quit_action = QAction("退出", app)
                quit_action.triggered.connect(app.quit)

                menu.addAction(show_action)
                menu.addAction(quit_action)
                tray_icon.setContextMenu(menu)

                def on_tray_activated(reason):
                    if reason == QSystemTrayIcon.DoubleClick:
                        window.show()

                tray_icon.activated.connect(on_tray_activated)
                tray_icon.show()
                return tray_icon
            except Exception as e:
                logger.error(f"Failed to create tray icon: {e}")
                return None

        tray = create_tray_icon()

        sys.exit(app.exec_())

    except ImportError as e:
        logger.error(f"PyQt5 import failed: {e}")
        raise
    except Exception as e:
        logger.error(f"Qt window error: {e}")
        raise