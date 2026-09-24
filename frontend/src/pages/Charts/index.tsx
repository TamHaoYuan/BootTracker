import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  App as AntdApp,
  Card,
  Empty,
  Select,
  Segmented,
  Space,
  Table,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table/interface';
import { BarChartOutlined, LineChartOutlined, PieChartOutlined } from '@ant-design/icons';

import { statsApi } from '../../api';
import { useSettingsStore } from '../../stores/settingsStore';
import { useThemeStore } from '../../stores/themeStore';
import { fmtDuration, useSessionStore } from '../../stores/sessionStore';
import type { TrendStat, WeeklyStat } from '../../api/types';

const { Title, Text } = Typography;

type ChartType = 'bar' | 'line' | 'pie';
type DayOption = 30 | 60 | 90;

interface TrendRow {
  date: string;
  stat: TrendStat;
}

interface WeeklyRow {
  weekKey: string;
  stat: WeeklyStat;
}

const CHART_WIDTH = 1000;
const CHART_HEIGHT = 360;
const CHART_PADDING_LEFT = 70;
const CHART_PADDING_RIGHT = 30;
const CHART_PADDING_TOP = 30;
const CHART_PADDING_BOTTOM = 50;

const PIE_COLORS = ['#8b5cf6', '#3b82f6', '#10b981', '#f97316', '#ef4444', '#d946ef'];

function msToHours(ms: number): number {
  return ms / 3600000;
}

function fmtHoursAxis(hours: number): string {
  if (hours === 0) return '0h';
  if (hours < 1) return `${Math.round(hours * 60)}m`;
  const h = Math.floor(hours);
  const m = Math.round((hours - h) * 60);
  return m > 0 ? `${h}h${m}m` : `${h}h`;
}

function getDateRange(days: number): { start: string; end: string } {
  const end = new Date();
  const start = new Date();
  start.setDate(end.getDate() - days + 1);
  const fmt = (d: Date) => {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  };
  return { start: fmt(start), end: fmt(end) };
}

function getWeekStart(dateStr: string): string {
  const d = new Date(dateStr);
  const day = d.getDay();
  const diff = day === 0 ? 6 : day - 1;
  d.setDate(d.getDate() - diff);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day2 = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day2}`;
}

function Charts() {
  const { message } = AntdApp.useApp();
  const { settings } = useSettingsStore();
  const { mode, theme } = useThemeStore();
  const { data: sessionData } = useSessionStore();

  const [days, setDays] = useState<DayOption>(30);
  const [chartType, setChartType] = useState<ChartType>(() => {
    const stored = settings?.defaultChartType;
    return (stored === 'bar' || stored === 'line' ? stored : 'line') as ChartType;
  });
  const [trendLoading, setTrendLoading] = useState(false);
  const [weeklyLoading, setWeeklyLoading] = useState(false);
  const [trendRows, setTrendRows] = useState<TrendRow[]>([]);
  const [weeklyRows, setWeeklyRows] = useState<WeeklyRow[]>([]);

  const dateRange = useMemo(() => getDateRange(days), [days]);

  const loadTrend = useCallback(async () => {
    setTrendLoading(true);
    try {
      const resp = await statsApi.trend(days);
      const data = resp.data ?? {};
      const rows: TrendRow[] = Object.keys(data)
        .sort()
        .map((date) => ({ date, stat: data[date] }));
      setTrendRows(rows);
    } catch (e) {
      message.error(`加载趋势数据失败：${(e as Error).message}`);
    } finally {
      setTrendLoading(false);
    }
  }, [days, message]);

  const loadWeekly = useCallback(async () => {
    setWeeklyLoading(true);
    try {
      const data = await statsApi.weekly();
      const rows: WeeklyRow[] = Object.keys(data)
        .sort()
        .map((k) => ({ weekKey: k, stat: data[k] }))
        .slice(-12);
      setWeeklyRows(rows);
    } catch (e) {
      message.error(`加载周统计失败：${(e as Error).message}`);
    } finally {
      setWeeklyLoading(false);
    }
  }, [message]);

  useEffect(() => {
    void loadTrend();
  }, [loadTrend]);

  useEffect(() => {
    if (chartType === 'pie') {
      void loadWeekly();
    }
  }, [chartType, loadWeekly]);

  const hasAnyData = sessionData.sessions.length > 0 || trendRows.length > 0;

  /* ================= 主图渲染 ================= */

  const chartInnerWidth = CHART_WIDTH - CHART_PADDING_LEFT - CHART_PADDING_RIGHT;
  const chartInnerHeight = CHART_HEIGHT - CHART_PADDING_TOP - CHART_PADDING_BOTTOM;

  const renderBarChart = () => {
    if (trendRows.length === 0) return null;
    const values = trendRows.map((r) => msToHours(r.stat.totalDuration ?? 0));
    const maxVal = Math.max(...values, 1);
    const barCount = trendRows.length;
    const barSlot = chartInnerWidth / barCount;
    const barWidth = Math.max(2, barSlot * 0.65);
    const yTicks = 5;

    return (
      <svg viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} style={{ width: '100%', height: 'auto' }}>
        {Array.from({ length: yTicks + 1 }).map((_, i) => {
          const y = CHART_PADDING_TOP + (chartInnerHeight / yTicks) * i;
          const val = maxVal * (1 - i / yTicks);
          return (
            <g key={`tick-${i}`}>
              <line
                x1={CHART_PADDING_LEFT}
                x2={CHART_WIDTH - CHART_PADDING_RIGHT}
                y1={y}
                y2={y}
                stroke="rgba(var(--accent-rgb), 0.12)"
                strokeWidth={1}
                strokeDasharray={i === yTicks ? '' : '4 4'}
              />
              <text
                x={CHART_PADDING_LEFT - 8}
                y={y + 4}
                textAnchor="end"
                fontSize={11}
                fill="var(--text-muted)"
              >
                {fmtHoursAxis(val)}
              </text>
            </g>
          );
        })}
        {trendRows.map((row, i) => {
          const val = msToHours(row.stat.totalDuration ?? 0);
          const barHeight = (val / maxVal) * chartInnerHeight;
          const x = CHART_PADDING_LEFT + barSlot * i + (barSlot - barWidth) / 2;
          const y = CHART_PADDING_TOP + chartInnerHeight - barHeight;
          return (
            <rect
              key={row.date}
              x={x}
              y={y}
              width={barWidth}
              height={barHeight}
              fill="var(--accent)"
              rx={3}
              opacity={0.85}
              style={{ cursor: 'pointer', transition: 'opacity 0.15s' }}
            >
              <title>{`${row.date}\n总时长：${fmtDuration(row.stat.totalDuration)}`}</title>
            </rect>
          );
        })}
        {trendRows
          .filter((_, i) => barCount <= 15 || i % Math.ceil(barCount / 10) === 0)
          .map((row, idx) => {
            const i = trendRows.indexOf(row);
            const x = CHART_PADDING_LEFT + barSlot * i + barSlot / 2;
            const label = row.date.slice(5);
            return (
              <text
                key={`x-${idx}`}
                x={x}
                y={CHART_HEIGHT - CHART_PADDING_BOTTOM + 18}
                textAnchor="middle"
                fontSize={10}
                fill="var(--text-muted)"
                transform={`rotate(-30 ${x} ${CHART_HEIGHT - CHART_PADDING_BOTTOM + 18})`}
              >
                {label}
              </text>
            );
          })}
      </svg>
    );
  };

  const renderLineChart = () => {
    if (trendRows.length === 0) return null;
    const values = trendRows.map((r) => msToHours(r.stat.totalDuration ?? 0));
    const maxVal = Math.max(...values, 1);
    const pointCount = trendRows.length;
    const stepX = pointCount > 1 ? chartInnerWidth / (pointCount - 1) : 0;
    const yTicks = 5;

    const points: string[] = trendRows.map((row, i) => {
      const val = msToHours(row.stat.totalDuration ?? 0);
      const x = CHART_PADDING_LEFT + stepX * i;
      const y = CHART_PADDING_TOP + chartInnerHeight - (val / maxVal) * chartInnerHeight;
      return `${x},${y}`;
    });

    const areaPath =
      points.length > 0
        ? `M ${CHART_PADDING_LEFT},${CHART_PADDING_TOP + chartInnerHeight} L ${points.join(' L ')} L ${
            CHART_PADDING_LEFT + (pointCount > 1 ? chartInnerWidth : 0)
          },${CHART_PADDING_TOP + chartInnerHeight} Z`
        : '';

    return (
      <svg viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} style={{ width: '100%', height: 'auto' }}>
        {Array.from({ length: yTicks + 1 }).map((_, i) => {
          const y = CHART_PADDING_TOP + (chartInnerHeight / yTicks) * i;
          const val = maxVal * (1 - i / yTicks);
          return (
            <g key={`tick-${i}`}>
              <line
                x1={CHART_PADDING_LEFT}
                x2={CHART_WIDTH - CHART_PADDING_RIGHT}
                y1={y}
                y2={y}
                stroke="rgba(var(--accent-rgb), 0.12)"
                strokeWidth={1}
                strokeDasharray={i === yTicks ? '' : '4 4'}
              />
              <text
                x={CHART_PADDING_LEFT - 8}
                y={y + 4}
                textAnchor="end"
                fontSize={11}
                fill="var(--text-muted)"
              >
                {fmtHoursAxis(val)}
              </text>
            </g>
          );
        })}
        {areaPath && (
          <path
            d={areaPath}
            fill="var(--accent)"
            opacity={0.12}
          />
        )}
        {points.length > 0 && (
          <polyline
            points={points.join(' ')}
            fill="none"
            stroke="var(--accent)"
            strokeWidth={2.5}
            strokeLinejoin="round"
            strokeLinecap="round"
          />
        )}
        {trendRows.map((row, i) => {
          const val = msToHours(row.stat.totalDuration ?? 0);
          const x = CHART_PADDING_LEFT + stepX * i;
          const y = CHART_PADDING_TOP + chartInnerHeight - (val / maxVal) * chartInnerHeight;
          return (
            <g key={row.date}>
              <circle
                cx={x}
                cy={y}
                r={4}
                fill="var(--accent)"
                stroke="var(--bg-card)"
                strokeWidth={2}
                style={{ cursor: 'pointer' }}
              >
                <title>{`${row.date}\n总时长：${fmtDuration(row.stat.totalDuration)}`}</title>
              </circle>
            </g>
          );
        })}
        {trendRows
          .filter((_, i) => pointCount <= 15 || i % Math.ceil(pointCount / 10) === 0)
          .map((row, idx) => {
            const i = trendRows.indexOf(row);
            const x = CHART_PADDING_LEFT + stepX * i;
            const label = row.date.slice(5);
            return (
              <text
                key={`x-${idx}`}
                x={x}
                y={CHART_HEIGHT - CHART_PADDING_BOTTOM + 18}
                textAnchor="middle"
                fontSize={10}
                fill="var(--text-muted)"
                transform={`rotate(-30 ${x} ${CHART_HEIGHT - CHART_PADDING_BOTTOM + 18})`}
              >
                {label}
              </text>
            );
          })}
      </svg>
    );
  };

  const renderPieChart = () => {
    if (weeklyRows.length === 0) return null;
    const cx = CHART_WIDTH / 2;
    const cy = CHART_HEIGHT / 2;
    const radius = Math.min(chartInnerWidth, chartInnerHeight) / 2 - 10;
    const total = weeklyRows.reduce((s, r) => s + (r.stat.bootCount ?? 0), 0);
    if (total === 0) return null;

    let cumulative = 0;
    const segments = weeklyRows.map((row, i) => {
      const value = row.stat.bootCount ?? 0;
      const startAngle = (cumulative / total) * Math.PI * 2;
      cumulative += value;
      const endAngle = (cumulative / total) * Math.PI * 2;
      const largeArc = endAngle - startAngle > Math.PI ? 1 : 0;
      const x1 = cx + radius * Math.sin(startAngle);
      const y1 = cy - radius * Math.cos(startAngle);
      const x2 = cx + radius * Math.sin(endAngle);
      const y2 = cy - radius * Math.cos(endAngle);
      const path =
        value === total
          ? `M ${cx - radius},${cy} A ${radius},${radius} 0 1 1 ${cx + radius},${cy} A ${radius},${radius} 0 1 1 ${cx - radius},${cy} Z`
          : `M ${cx},${cy} L ${x1},${y1} A ${radius},${radius} 0 ${largeArc} 1 ${x2},${y2} Z`;
      const midAngle = (startAngle + endAngle) / 2;
      const labelR = radius * 0.65;
      const labelX = cx + labelR * Math.sin(midAngle);
      const labelY = cy - labelR * Math.cos(midAngle);
      const pct = total > 0 ? (value / total) * 100 : 0;
      return {
        path,
        color: PIE_COLORS[i % PIE_COLORS.length],
        row,
        value,
        pct,
        labelX,
        labelY,
      };
    });

    const legendStartX = 20;
    const legendStartY = CHART_PADDING_TOP;
    const legendItemH = 22;

    return (
      <svg viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`} style={{ width: '100%', height: 'auto' }}>
        {segments.map((seg) => (
          <g key={seg.row.weekKey}>
            <path
              d={seg.path}
              fill={seg.color}
              stroke="var(--bg-card)"
              strokeWidth={2}
              opacity={0.9}
              style={{ cursor: 'pointer', transition: 'opacity 0.15s' }}
            >
              <title>{`${seg.row.weekKey}\n开机次数：${seg.value}\n占比：${seg.pct.toFixed(1)}%`}</title>
            </path>
            {seg.pct >= 5 && (
              <text
                x={seg.labelX}
                y={seg.labelY}
                textAnchor="middle"
                dominantBaseline="middle"
                fontSize={11}
                fill="#fff"
                fontWeight={600}
              >
                {seg.pct.toFixed(0)}%
              </text>
            )}
          </g>
        ))}
        {segments.map((seg, i) => (
          <g key={`legend-${seg.row.weekKey}`} transform={`translate(${legendStartX}, ${legendStartY + i * legendItemH})`}>
            <rect x={0} y={2} width={14} height={14} rx={3} fill={seg.color} />
            <text x={20} y={13} fontSize={11} fill="var(--text-secondary)">
              {seg.row.weekKey.slice(5)} · {seg.value}次 ({seg.pct.toFixed(1)}%)
            </text>
          </g>
        ))}
      </svg>
    );
  };

  /* ================= 热度图渲染 ================= */

  const heatmapMatrix = useMemo(() => {
    if (trendRows.length === 0) return { weeks: [] as string[][], weekStarts: [] as string[] };

    const startDate = trendRows[0].date;
    const endDate = trendRows[trendRows.length - 1].date;

    const ws = getWeekStart(startDate);
    const allDates: string[] = [];
    const cur = new Date(ws);
    const end = new Date(endDate);
    while (cur <= end) {
      const y = cur.getFullYear();
      const m = String(cur.getMonth() + 1).padStart(2, '0');
      const d = String(cur.getDate()).padStart(2, '0');
      allDates.push(`${y}-${m}-${d}`);
      cur.setDate(cur.getDate() + 1);
    }

    const weekCount = Math.ceil(allDates.length / 7);
    const weeks: string[][] = [];
    const weekStarts: string[] = [];
    for (let w = 0; w < weekCount; w++) {
      const row: string[] = [];
      weekStarts.push(allDates[w * 7]);
      for (let d = 0; d < 7; d++) {
        const idx = w * 7 + d;
        row.push(idx < allDates.length ? allDates[idx] : '');
      }
      weeks.push(row);
    }

    return { weeks, weekStarts };
  }, [trendRows]);

  // 旧版 getHeatColor 逻辑迁移：mica 灰阶、亮色/暗色不同饱和度
  const isMica = theme === 'mica';
  const isLight = mode === 'light';
  const heatLevels = useMemo(() => {
    if (isMica) {
      // 云母：灰阶热力
      return ['rgba(148,163,184,0.25)', 'rgba(148,163,184,0.4)', 'rgba(148,163,184,0.6)', 'rgba(148,163,184,0.8)'];
    }
    // 亮色用低饱和、暗色用高饱和，accent 通过 CSS 变量随主题变体切换
    if (isLight) {
      return [
        'rgba(var(--accent-rgb),0.12)',
        'rgba(var(--accent-rgb),0.25)',
        'rgba(var(--accent-rgb),0.45)',
        'rgba(var(--accent-rgb),0.7)',
      ];
    }
    return [
      'rgba(var(--accent-rgb),0.5)',
      'rgba(var(--accent-rgb),0.65)',
      'rgba(var(--accent-rgb),0.8)',
      'rgba(var(--accent-rgb),0.95)',
    ];
  }, [isMica, isLight]);

  const heatEmpty = useMemo(() => {
    if (isMica) return 'rgba(255,255,255,0.03)';
    if (isLight) return 'rgba(0,0,0,0.04)';
    return 'rgba(30,41,59,0.5)';
  }, [isMica, isLight]);

  const getHeatColor = (count: number): string => {
    if (count <= 0) return heatEmpty;
    if (count <= 2) return heatLevels[0];
    if (count <= 4) return heatLevels[1];
    if (count <= 6) return heatLevels[2];
    return heatLevels[3];
  };

  const getBootCount = (date: string): number => {
    const found = trendRows.find((r) => r.date === date);
    return found?.stat.bootCount ?? 0;
  };

  const renderHeatmap = () => {
    const { weeks, weekStarts } = heatmapMatrix;
    if (weeks.length === 0) return null;

    const cellSize = 18;
    const cellGap = 4;
    const leftPad = 80;
    const topPad = 30;
    const bottomPad = 50;
    const width = leftPad + 7 * (cellSize + cellGap) + 20;
    const height = topPad + weeks.length * (cellSize + cellGap) + bottomPad;

    const monthLabels: { x: number; label: string }[] = [];
    let lastMonth = -1;
    weeks.forEach((week) => {
      week.forEach((date, di) => {
        if (!date) return;
        const d = new Date(date);
        const month = d.getMonth();
        if (month !== lastMonth && di === 0) {
          const x = leftPad + di * (cellSize + cellGap);
          monthLabels.push({ x, label: `${d.getFullYear()}-${String(month + 1).padStart(2, '0')}` });
          lastMonth = month;
        }
      });
    });

    const weekdayLabels = ['一', '二', '三', '四', '五', '六', '日'];

    return (
      <svg
        viewBox={`0 0 ${width} ${height}`}
        style={{ width: '100%', height: 'auto', maxWidth: 800 }}
      >
        {weekdayLabels.map((lbl, i) => (
          <text
            key={`wd-${i}`}
            x={leftPad - 6}
            y={topPad + i * (cellSize + cellGap) + cellSize - 4}
            textAnchor="end"
            fontSize={10}
            fill="var(--text-muted)"
          >
            {lbl}
          </text>
        ))}
        {weeks.map((week, wi) => (
          <g key={`wk-${wi}`}>
            {wi % 2 === 0 && (
              <text
                x={leftPad - 6}
                y={topPad + wi * (cellSize + cellGap) + cellSize - 4}
                textAnchor="end"
                fontSize={9}
                fill="var(--text-muted)"
              >
                {weekStarts[wi]?.slice(5)}
              </text>
            )}
            {week.map((date, di) => {
              if (!date) return null;
              const count = getBootCount(date);
              const x = leftPad + di * (cellSize + cellGap);
              const y = topPad + wi * (cellSize + cellGap);
              return (
                <g key={date}>
                  <rect
                    x={x}
                    y={y}
                    width={cellSize}
                    height={cellSize}
                    rx={3}
                    fill={getHeatColor(count)}
                    style={{ cursor: 'pointer' }}
                  >
                    <title>{`${date}\n开机次数：${count}`}</title>
                  </rect>
                </g>
              );
            })}
          </g>
        ))}
        {monthLabels.map((m, i) => (
          <text
            key={`ml-${i}`}
            x={m.x}
            y={topPad + weeks.length * (cellSize + cellGap) + 20}
            fontSize={10}
            fill="var(--text-muted)"
          >
            {m.label}
          </text>
        ))}
        <g transform={`translate(${leftPad}, ${topPad + weeks.length * (cellSize + cellGap) + 36})`}>
          <text x={0} y={10} fontSize={10} fill="var(--text-muted)">
            少
          </text>
          {[0, 1, 2, 3, 4].map((i) => (
            <rect
              key={`lg-${i}`}
              x={24 + i * (cellSize + cellGap)}
              y={0}
              width={cellSize}
              height={cellSize}
              rx={3}
              fill={i === 0 ? heatEmpty : heatLevels[i - 1]}
            />
          ))}
          <text x={24 + 5 * (cellSize + cellGap)} y={10} fontSize={10} fill="var(--text-muted)">
            多
          </text>
        </g>
      </svg>
    );
  };

  /* ================= 数据表格 ================= */

  const top10Rows = useMemo(
    () =>
      [...trendRows]
        .sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime())
        .slice(0, 10),
    [trendRows],
  );

  const tableColumns: ColumnsType<TrendRow> = [
    {
      title: '日期',
      dataIndex: 'date',
      key: 'date',
      width: 120,
      render: (v: string) => <Text style={{ fontFamily: 'JetBrains Mono, monospace' }}>{v}</Text>,
    },
    {
      title: '开机次数',
      key: 'bootCount',
      width: 100,
      render: (_: unknown, r: TrendRow) => r.stat.bootCount ?? 0,
    },
    {
      title: '关机次数',
      key: 'shutdownCount',
      width: 100,
      render: (_: unknown, r: TrendRow) => r.stat.shutdownCount ?? 0,
    },
    {
      title: '总时长',
      key: 'totalDuration',
      width: 160,
      render: (_: unknown, r: TrendRow) => (
        <span style={{ color: 'var(--accent)', fontFamily: 'JetBrains Mono, monospace' }}>
          {fmtDuration(r.stat.totalDuration)}
        </span>
      ),
    },
    {
      title: '平均时长',
      key: 'avgDuration',
      width: 160,
      render: (_: unknown, r: TrendRow) => (
        <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>
          {fmtDuration(r.stat.avgDuration)}
        </span>
      ),
    },
  ];

  /* ================= 主渲染 ================= */

  return (
    <div style={{ maxWidth: 1400, margin: '0 auto', padding: '24px', width: '100%' }}>
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }} wrap>
        <Space direction="vertical" size={4}>
          <Title level={4} style={{ color: 'var(--text-primary)', margin: 0 }}>
            数据分析
          </Title>
          <Text style={{ color: 'var(--text-muted)', fontSize: 13 }}>
            数据范围：{days} 天（{dateRange.start} ~ {dateRange.end}）
          </Text>
        </Space>
        <Space wrap>
          <Text style={{ color: 'var(--text-secondary)' }}>最近</Text>
          <Select<DayOption>
            value={days}
            onChange={(v) => setDays(v)}
            style={{ width: 100 }}
            options={[
              { value: 30, label: '30 天' },
              { value: 60, label: '60 天' },
              { value: 90, label: '90 天' },
            ]}
          />
        </Space>
      </Space>

      {!hasAnyData && !trendLoading ? (
        <Card
          style={{
            background: 'var(--bg-card)',
            borderColor: 'var(--border-color)',
          }}
        >
          <Empty description="暂无开机会话数据，记录第一次开机后再来查看分析图表吧" />
        </Card>
      ) : (
        <>
          <Card
            style={{
              background: 'var(--bg-card)',
              borderColor: 'var(--border-color)',
              marginBottom: 16,
            }}
            title={
              <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                <span>开机时长趋势</span>
                <Segmented<ChartType>
                  value={chartType}
                  onChange={setChartType}
                  options={[
                    {
                      value: 'bar',
                      icon: <BarChartOutlined />,
                      label: '柱状图',
                    },
                    {
                      value: 'line',
                      icon: <LineChartOutlined />,
                      label: '折线图',
                    },
                    {
                      value: 'pie',
                      icon: <PieChartOutlined />,
                      label: '饼图',
                    },
                  ]}
                />
              </Space>
            }
            loading={trendLoading || (chartType === 'pie' && weeklyLoading)}
          >
            {chartType === 'bar' && renderBarChart()}
            {chartType === 'line' && renderLineChart()}
            {chartType === 'pie' && renderPieChart()}
          </Card>

          {trendRows.length > 0 && (
            <Card
              style={{
                background: 'var(--bg-card)',
                borderColor: 'var(--border-color)',
                marginBottom: 16,
              }}
              title="详细数据（最近 10 天）"
              size="small"
            >
              <Table<TrendRow>
                rowKey="date"
                columns={tableColumns}
                dataSource={top10Rows}
                pagination={false}
                size="small"
              />
            </Card>
          )}

          <Card
            style={{
              background: 'var(--bg-card)',
              borderColor: 'var(--border-color)',
            }}
            title={
              <Space>
                <span>开机热度图</span>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  （颜色越深表示开机次数越多）
                </Text>
              </Space>
            }
            loading={trendLoading}
          >
            {renderHeatmap()}
          </Card>
        </>
      )}
    </div>
  );
}

export default Charts;
