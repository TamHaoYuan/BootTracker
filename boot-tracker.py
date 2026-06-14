#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""开机记录 - 桌面应用入口"""
import os
import sys

# 确保项目根目录在 sys.path 中（用于导入 server 包）
APP_DIR = os.path.dirname(os.path.abspath(__file__))
if APP_DIR not in sys.path:
    sys.path.insert(0, APP_DIR)

from server import main

if __name__ == "__main__":
    main()
