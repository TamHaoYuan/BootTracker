#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""系统托盘模块 — 负责创建和管理系统托盘图标"""
import os
import sys
import math
import threading

from PIL import Image, ImageDraw


def create_tray_icon(stop_event):
    size = 64
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    scr_x1, scr_y1, scr_x2, scr_y2 = 10, 8, 54, 44
    draw.rounded_rectangle(
        [scr_x1, scr_y1, scr_x2, scr_y2], radius=4,
        outline=(230, 230, 230, 255), width=2
    )
    stand_cx = 32
    draw.line([(stand_cx - 8, scr_y2), (stand_cx + 8, scr_y2)],
              fill=(230, 230, 230, 255), width=2)
    draw.line([(stand_cx, scr_y2), (stand_cx, scr_y2 + 5)],
              fill=(230, 230, 230, 255), width=2)
    draw.line([(stand_cx - 6, scr_y2 + 5), (stand_cx + 6, scr_y2 + 5)],
              fill=(230, 230, 230, 255), width=2)

    pcx, pcy = 32, 26
    pr = 10
    draw.line([(pcx, pcy - pr + 3), (pcx, pcy - 3)],
              fill=(139, 92, 246, 255), width=2)
    points = []
    for deg in range(210, 331, 3):
        rad = math.radians(deg)
        px = pcx + pr * math.cos(rad)
        py = pcy - pr * math.sin(rad)
        points.append((px, py))
    if len(points) > 1:
        draw.line(points, fill=(139, 92, 246, 255), width=2)

    def show_window(icon, item=None):
        import subprocess
        exe = sys.executable
        if exe.endswith("python.exe"):
            exe = exe.replace("python.exe", "pythonw.exe")
        subprocess.Popen(
            [exe, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "boot-tracker.py")]
        )

    def exit_app(icon, item=None):
        icon.stop()
        if stop_event:
            stop_event.set()
        os._exit(0)

    import pystray
    menu = pystray.Menu(
        pystray.MenuItem("显示窗口", show_window, default=True),
        pystray.MenuItem("退出", exit_app),
    )
    return pystray.Icon("boottracker", img, "开机记录", menu)