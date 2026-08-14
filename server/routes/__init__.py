#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""路由模块 - API 路由注册和分发"""

from typing import Dict, Callable, Any, Tuple
from http.server import BaseHTTPRequestHandler

RouteHandler = Callable[[BaseHTTPRequestHandler, Dict[str, Any]], Tuple[int, Dict[str, Any]]]


class RouteRegistry:
    """路由注册表"""
    
    def __init__(self):
        self._routes: Dict[str, Dict[str, RouteHandler]] = {
            "GET": {},
            "POST": {},
            "PUT": {},
            "DELETE": {},
            "PATCH": {},
        }
    
    def register(self, method: str, path: str, handler: RouteHandler) -> None:
        """注册路由"""
        method = method.upper()
        if method not in self._routes:
            raise ValueError(f"Unsupported HTTP method: {method}")
        self._routes[method][path] = handler
    
    def get(self, path: str) -> RouteHandler | None:
        """获取 GET 路由处理器"""
        return self._routes["GET"].get(path)
    
    def post(self, path: str) -> RouteHandler | None:
        """获取 POST 路由处理器"""
        return self._routes["POST"].get(path)
    
    def put(self, path: str) -> RouteHandler | None:
        """获取 PUT 路由处理器"""
        return self._routes["PUT"].get(path)
    
    def delete(self, path: str) -> RouteHandler | None:
        """获取 DELETE 路由处理器"""
        return self._routes["DELETE"].get(path)


# 全局路由注册表
registry = RouteRegistry()


def route(method: str, path: str) -> Callable[[RouteHandler], RouteHandler]:
    """装饰器：注册路由"""
    def decorator(handler: RouteHandler) -> RouteHandler:
        registry.register(method, path, handler)
        return handler
    return decorator


def get(path: str) -> Callable[[RouteHandler], RouteHandler]:
    """装饰器：注册 GET 路由"""
    return route("GET", path)


def post(path: str) -> Callable[[RouteHandler], RouteHandler]:
    """装饰器：注册 POST 路由"""
    return route("POST", path)


def put(path: str) -> Callable[[RouteHandler], RouteHandler]:
    """装饰器：注册 PUT 路由"""
    return route("PUT", path)


def delete(path: str) -> Callable[[RouteHandler], RouteHandler]:
    """装饰器：注册 DELETE 路由"""
    return route("DELETE", path)


# 导入所有路由模块
from . import data_routes
from . import trash_routes
from . import settings_routes
from . import backup_routes
from . import version_routes
from . import tunnel_routes
from . import stats_routes
from . import window_routes
