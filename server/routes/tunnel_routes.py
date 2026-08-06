#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""隧道路由模块 - Cloudflare Tunnel 相关 API"""

from typing import Tuple, Dict

from ..tunnel import get_tunnel_url, start_tunnel, stop_tunnel
from ..logging_config import logger
from . import get


@get("/api/tunnel-url")
def get_tunnel_url_handler(req, body) -> Tuple[int, Dict]:
    """获取隧道 URL"""
    return 200, {"url": get_tunnel_url()}
