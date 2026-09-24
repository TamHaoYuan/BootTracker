#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""统计分析路由模块 - 数据统计分析 API

提供数据统计和趋势分析功能：
- 每日统计：开机次数、关机次数、总时长
- 每周统计：按周汇总数据
- 趋势分析：最近 N 天的变化趋势
- 异常检测：异常长/短的开机时间
"""

from datetime import datetime, timedelta
from typing import Tuple, Dict, List, Any
from collections import defaultdict

from ..data_store import load_data
from ..logging_config import logger
from . import get


def _parse_datetime(dt_str: str) -> datetime | None:
    """解析 ISO 格式日期时间"""
    try:
        return datetime.fromisoformat(dt_str.replace("Z", "+00:00"))
    except Exception:
        return None


def _calculate_duration_ms(start_str: str, end_str: str) -> int | None:
    """计算持续时间（毫秒）"""
    start = _parse_datetime(start_str)
    end = _parse_datetime(end_str)
    if start and end:
        return int((end - start).total_seconds() * 1000)
    return None


def _get_session_date(session: Dict) -> str | None:
    """获取会话日期（YYYY-MM-DD）"""
    boot_time = _parse_datetime(session.get("bootTime", ""))
    if boot_time:
        return boot_time.strftime("%Y-%m-%d")
    return None


@get("/api/stats/daily")
def get_daily_stats(req, body) -> Tuple[int, Dict]:
    """获取每日统计数据"""
    data = load_data()
    sessions = data.get("sessions", [])
    
    daily_stats = defaultdict(lambda: {
        "bootCount": 0,
        "shutdownCount": 0,
        "totalDuration": 0,
        "sessions": [],
    })
    
    for session in sessions:
        date = _get_session_date(session)
        if not date:
            continue
        
        daily_stats[date]["bootCount"] += 1
        daily_stats[date]["sessions"].append(session)
        
        if session.get("shutdownTime"):
            daily_stats[date]["shutdownCount"] += 1
            duration = session.get("duration") or _calculate_duration_ms(session["bootTime"], session["shutdownTime"])
            if duration:
                daily_stats[date]["totalDuration"] += duration
    
    result = {
        date: {
            "bootCount": stats["bootCount"],
            "shutdownCount": stats["shutdownCount"],
            "totalDuration": stats["totalDuration"],
            "sessionCount": len(stats["sessions"]),
        }
        for date, stats in sorted(daily_stats.items())
    }
    
    return 200, {"data": result}


@get("/api/stats/weekly")
def get_weekly_stats(req, body) -> Tuple[int, Dict]:
    """获取每周统计数据"""
    data = load_data()
    sessions = data.get("sessions", [])
    
    weekly_stats = defaultdict(lambda: {
        "bootCount": 0,
        "shutdownCount": 0,
        "totalDuration": 0,
    })
    
    for session in sessions:
        boot_time = _parse_datetime(session.get("bootTime", ""))
        if not boot_time:
            continue
        
        year, week = boot_time.isocalendar()[:2]
        week_key = f"{year}-W{week:02d}"
        
        weekly_stats[week_key]["bootCount"] += 1
        
        if session.get("shutdownTime"):
            weekly_stats[week_key]["shutdownCount"] += 1
            duration = session.get("duration") or _calculate_duration_ms(session["bootTime"], session["shutdownTime"])
            if duration:
                weekly_stats[week_key]["totalDuration"] += duration
    
    result = dict(sorted(weekly_stats.items()))
    return 200, {"data": result}


@get("/api/stats/trend")
def get_trend_stats(req, body) -> Tuple[int, Dict]:
    """获取趋势分析数据（最近 N 天）"""
    days = int(body.get("days", 30))
    
    data = load_data()
    sessions = data.get("sessions", [])
    
    trend_data: Dict[str, Dict] = {}
    
    today = datetime.now().replace(hour=0, minute=0, second=0, microsecond=0)
    for i in range(days):
        date = today - timedelta(days=days - 1 - i)
        date_str = date.strftime("%Y-%m-%d")
        trend_data[date_str] = {
            "bootCount": 0,
            "shutdownCount": 0,
            "totalDuration": 0,
            "avgDuration": 0,
        }
    
    for session in sessions:
        date = _get_session_date(session)
        if date not in trend_data:
            continue
        
        trend_data[date]["bootCount"] += 1
        
        if session.get("shutdownTime"):
            trend_data[date]["shutdownCount"] += 1
            duration = session.get("duration") or _calculate_duration_ms(session["bootTime"], session["shutdownTime"])
            if duration:
                trend_data[date]["totalDuration"] += duration
    
    for date, stats in trend_data.items():
        if stats["shutdownCount"] > 0:
            stats["avgDuration"] = stats["totalDuration"] // stats["shutdownCount"]
        else:
            stats["avgDuration"] = 0
    
    return 200, {"data": trend_data, "days": days}


@get("/api/stats/overview")
def get_overview_stats(req, body) -> Tuple[int, Dict]:
    """获取概览统计数据"""
    data = load_data()
    sessions = data.get("sessions", [])
    
    total_boot = data.get("bootCount", 0)
    total_shutdown = data.get("shutdownCount", 0)
    total_duration = sum(s.get("duration", 0) for s in sessions if s.get("duration"))
    
    active_sessions = [s for s in sessions if not s.get("shutdownTime")]
    active_count = len(active_sessions)
    
    if total_shutdown > 0:
        avg_duration = total_duration // total_shutdown
    else:
        avg_duration = 0
    
    recent_sessions = sorted(sessions, key=lambda s: s.get("bootTime", ""), reverse=True)[:5]
    
    first_session = min(sessions, key=lambda s: s.get("bootTime", "")) if sessions else None
    first_date = first_session.get("bootTime", "")[:10] if first_session else None
    
    last_session = max(sessions, key=lambda s: s.get("bootTime", "")) if sessions else None
    last_date = last_session.get("bootTime", "")[:10] if last_session else None
    
    result = {
        "totalBoot": total_boot,
        "totalShutdown": total_shutdown,
        "totalDuration": total_duration,
        "avgDuration": avg_duration,
        "activeCount": active_count,
        "recentSessions": recent_sessions,
        "firstDate": first_date,
        "lastDate": last_date,
    }
    
    return 200, result


@get("/api/stats/anomalies")
def get_anomaly_stats(req, body) -> Tuple[int, Dict]:
    """获取异常数据（异常长/短的会话）"""
    data = load_data()
    sessions = data.get("sessions", [])
    
    completed_sessions = [s for s in sessions if s.get("shutdownTime") and s.get("duration")]
    
    if not completed_sessions:
        return 200, {"longest": [], "shortest": [], "active": []}
    
    durations = [s["duration"] for s in completed_sessions]
    avg_duration = sum(durations) // len(durations)
    std_dev = (sum((d - avg_duration) ** 2 for d in durations) // len(durations)) ** 0.5
    
    threshold_high = avg_duration + 2 * std_dev if std_dev else avg_duration * 2
    threshold_low = max(avg_duration - 2 * std_dev, 60000) if std_dev else 30000
    
    long_sessions = sorted(
        [s for s in completed_sessions if s["duration"] > threshold_high],
        key=lambda s: s["duration"],
        reverse=True,
    )[:5]
    
    short_sessions = sorted(
        [s for s in completed_sessions if s["duration"] < threshold_low],
        key=lambda s: s["duration"],
    )[:5]
    
    active_sessions = [s for s in sessions if not s.get("shutdownTime")]
    
    result = {
        "longest": long_sessions,
        "shortest": short_sessions,
        "active": active_sessions,
        "avgDuration": avg_duration,
        "thresholdHigh": int(threshold_high),
        "thresholdLow": int(threshold_low),
    }
    
    return 200, result
