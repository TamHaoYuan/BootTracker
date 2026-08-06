#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""日志配置模块

使用 Python 标准库 logging 替代自定义 _log() 函数，提供更完善的日志功能。

日志级别：
- DEBUG: 详细的调试信息
- INFO: 一般信息（默认）
- WARNING: 警告信息
- ERROR: 错误信息
- CRITICAL: 严重错误

日志输出：
- 文件日志：按天滚动，保留最近 30 天
- 控制台日志：仅显示 INFO 及以上级别
"""

import os
import logging
from logging.handlers import TimedRotatingFileHandler
from typing import Optional

from .config import LOG_FILE, APP_DIR


def setup_logging(level: int = logging.INFO) -> logging.Logger:
    """配置日志系统

    Args:
        level: 日志级别，默认为 INFO

    Returns:
        配置好的 logger 实例
    """
    # 创建 logger
    logger = logging.getLogger("boot-tracker")
    logger.setLevel(level)
    logger.propagate = False

    # 清除已存在的 handler，避免重复
    if logger.handlers:
        for handler in logger.handlers[:]:
            logger.removeHandler(handler)

    # 创建日志目录
    log_dir = os.path.dirname(LOG_FILE)
    if log_dir and not os.path.exists(log_dir):
        os.makedirs(log_dir, exist_ok=True)

    # 文件日志 handler（按天滚动）
    file_handler = TimedRotatingFileHandler(
        LOG_FILE,
        when="midnight",
        interval=1,
        backupCount=30,
        encoding="utf-8",
    )
    file_handler.setLevel(logging.DEBUG)
    file_formatter = logging.Formatter(
        "%(asctime)s [%(levelname)s] %(name)s:%(lineno)d - %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    file_handler.setFormatter(file_formatter)
    logger.addHandler(file_handler)

    # 控制台日志 handler（仅显示 INFO 及以上）
    console_handler = logging.StreamHandler()
    console_handler.setLevel(logging.INFO)
    console_formatter = logging.Formatter(
        "%(asctime)s [%(levelname)s] %(message)s",
        datefmt="%H:%M:%S",
    )
    console_handler.setFormatter(console_formatter)
    logger.addHandler(console_handler)

    return logger


# 创建全局 logger 实例
logger = setup_logging()


def get_logger(name: Optional[str] = None) -> logging.Logger:
    """获取指定名称的 logger

    Args:
        name: logger 名称，如果为 None 返回全局 logger

    Returns:
        logger 实例
    """
    if name:
        return logging.getLogger(f"boot-tracker.{name}")
    return logger
