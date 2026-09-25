import { useEffect, useMemo, useState } from 'react';
// useEffect 用于下方定时器
import {
  Button,
  Card,
  DatePicker,
  Layout,
  Segmented,
  Space,
  Table,
  Tooltip,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table/interface';
import {
  FieldTimeOutlined,
  FileExcelOutlined,
  FileTextOutlined,
  ReloadOutlined,
  TableOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import dayjs, { Dayjs } from 'dayjs';

import * as XLSX from 'xlsx';

import { useBootData } from '../../hooks/useBootData';
import { useUiStore, type RecordsView } from '../../stores/uiStore';
import {
  fmtFullTime,
  fmtDuration,
  liveDuration,
  isoToLocalDate,
  getLocalToday,
} from '../../stores/sessionStore';
import type { BootSession } from '../../api/types';
import StatusPill from '../../components/StatusPill';
import EmptyState from '../../components/EmptyState';

const { RangePicker } = DatePicker;
const { Title, Text } = Typography;

/* ================= 导出工具（与 Admin 共享行为，自包含副本） ================= */

function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

function todayFilenameStamp(): string {
  const d = new Date();
  return `${d.getFullYear()}${pad2(d.getMonth() + 1)}${pad2(d.getDate())}`;
}

function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function escapeCSVField(v: string | number | null | undefined): string {
  if (v === null || v === undefined) return '';
  const s = String(v);
  if (s.includes(',') || s.includes('"') || s.includes('\n') || s.includes('\r')) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}

function exportCSV(sessions: BootSession[]): void {
  const header = ['开机时间', '关机时间', '时长(ms)', '时长字符串'];
  const rows = sessions.map((s) => {
    const bootStr = fmtFullTime(s.bootTime);
    const shutdownStr = s.shutdownTime ? fmtFullTime(s.shutdownTime) : '';
    const durationMs = s.duration ?? '';
    const durationStr = s.duration ? fmtDuration(s.duration) : '';
    return [bootStr, shutdownStr, durationMs, durationStr].map(escapeCSVField).join(',');
  });
  const csv = '\ufeff' + [header.join(','), ...rows].join('\r\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  triggerDownload(blob, `boot-records-${todayFilenameStamp()}.csv`);
}

// 迁移旧版 app.js exportXLSX：开机记录 sheet + 统计 sheet
function exportXLSX(sessions: BootSession[]): void {
  if (sessions.length === 0) return;

  // Sheet 1: 开机记录
  const headers = ['序号', '开机时间', '关机时间', '会话时长', '状态'];
  const rows = sessions.map((s, i) => [
    i + 1,
    fmtFullTime(s.bootTime),
    s.shutdownTime ? fmtFullTime(s.shutdownTime) : '未关机',
    s.duration ? fmtDuration(s.duration) : '—',
    s.shutdownTime ? '已关机' : '进行中',
  ]);
  const wsData = [headers, ...rows];
  const ws = XLSX.utils.aoa_to_sheet(wsData);
  ws['!cols'] = [
    { wch: 6 },
    { wch: 20 },
    { wch: 20 },
    { wch: 14 },
    { wch: 10 },
  ];

  // Sheet 2: 统计
  const statsData: (string | number)[][] = [
    ['统计项', '数值'],
    ['导出时间', new Date().toLocaleString('zh-CN')],
    ['总开机次数', sessions.length],
    ['总关机次数', sessions.filter((s) => s.shutdownTime).length],
  ];
  const closedSessions = sessions.filter((s) => s.duration);
  if (closedSessions.length > 0) {
    const avg =
      closedSessions.reduce((acc: number, s) => acc + (s.duration ?? 0), 0) /
      closedSessions.length;
    statsData.push(['平均会话时长', fmtDuration(avg)]);
  }
  const wsStats = XLSX.utils.aoa_to_sheet(statsData);
  wsStats['!cols'] = [{ wch: 16 }, { wch: 20 }];

  const wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, '开机记录');
  XLSX.utils.book_append_sheet(wb, wsStats, '统计');

  const fileName = `开机记录_${todayFilenameStamp()}.xlsx`;
  XLSX.writeFile(wb, fileName);
}

/* ================= 页面主组件（只读视图） ================= */

type DateRange = [Dayjs | null, Dayjs | null] | null;
type ViewMode = RecordsView;

/** 时间轴条目时刻：HH:mm */
function fmtClock(iso: string): string {
  return new Date(iso).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

function Records() {
  const navigate = useNavigate();
  const { data, loading, refresh } = useBootData();

  /* ---------- 状态 ---------- */
  const [dateRange, setDateRange] = useState<DateRange>(null);
  const [now, setNow] = useState<number>(Date.now());
  // 视图模式跨会话记住（uiStore → localStorage）
  const view = useUiStore((s) => s.recordsView);
  const setView = useUiStore((s) => s.setRecordsView);

  /* ---------- 实时时长 tick（用于进行中会话） ---------- */
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);

  /* ---------- 筛选 ---------- */
  const filteredSessions = useMemo(() => {
    let list = data.sessions;
    if (dateRange) {
      const [from, to] = dateRange;
      if (from || to) {
        list = list.filter((s) => {
          const t = dayjs(s.bootTime);
          if (from && t.isBefore(from.startOf('day'))) return false;
          if (to && t.isAfter(to.endOf('day'))) return false;
          return true;
        });
      }
    }
    return [...list].sort(
      (a, b) => new Date(b.bootTime).getTime() - new Date(a.bootTime).getTime(),
    );
  }, [data.sessions, dateRange]);

  /* ---------- 时间轴分组（Structured 模式：按天分组，日期倒序） ---------- */
  const TIMELINE_LIMIT = 100;
  const timelineGroups = useMemo(() => {
    const map = new Map<string, BootSession[]>();
    for (const s of filteredSessions.slice(0, TIMELINE_LIMIT)) {
      const d = isoToLocalDate(s.bootTime);
      const list = map.get(d);
      if (list) list.push(s);
      else map.set(d, [s]);
    }
    const today = getLocalToday();
    const groups: { date: string; label: string; total: number; items: BootSession[] }[] = [];
    for (const [date, items] of map) {
      const total = items.reduce((acc, s) => {
        if (s.shutdownTime) {
          return (
            acc +
            (s.duration ??
              new Date(s.shutdownTime).getTime() - new Date(s.bootTime).getTime())
          );
        }
        return acc + Math.max(0, now - new Date(s.bootTime).getTime());
      }, 0);
      groups.push({
        date,
        label: date === today ? '今天' : date,
        total,
        items,
      });
    }
    return groups;
  }, [filteredSessions, now]);

  /* ---------- 表格列（只读，结构化：数字右对齐 tabular-nums + 状态胶囊） ---------- */
  const baseColumns: ColumnsType<BootSession> = [
    {
      title: '序号',
      key: 'idx',
      width: 70,
      align: 'right',
      render: (_v, _r, idx) => (
        <span className="tnum" style={{ color: 'var(--text-muted)' }}>
          {idx + 1}
        </span>
      ),
    },
    {
      title: '开机时间',
      dataIndex: 'bootTime',
      key: 'bootTime',
      width: 190,
      render: (v: string) => (
        <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
          {fmtFullTime(v)}
        </span>
      ),
    },
    {
      title: '关机时间',
      dataIndex: 'shutdownTime',
      key: 'shutdownTime',
      width: 190,
      render: (v: string | null) =>
        v ? (
          <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
            {fmtFullTime(v)}
          </span>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>—</span>
        ),
    },
    {
      title: '会话时长',
      key: 'duration',
      width: 150,
      align: 'right',
      render: (_v, record) => {
        if (!record.shutdownTime) {
          const dur = now - new Date(record.bootTime).getTime();
          return (
            <span
              className="tnum"
              style={{ fontFamily: 'JetBrains Mono, monospace', color: 'var(--accent)' }}
            >
              {dur > 0 ? fmtDuration(dur) : liveDuration(record.bootTime)}
            </span>
          );
        }
        return (
          <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
            {fmtDuration(record.duration)}
          </span>
        );
      },
    },
    {
      title: '状态',
      key: 'status',
      width: 100,
      render: (_v, record) =>
        record.shutdownTime ? (
          <StatusPill tone="success">已关机</StatusPill>
        ) : (
          <StatusPill tone="accent">进行中</StatusPill>
        ),
    },
  ];

  /* ---------- 渲染 ---------- */
  return (
    <Layout.Content style={{ padding: '24px', maxWidth: 1400, margin: '0 auto', width: '100%' }}>
      {/* 顶部操作栏 */}
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }} wrap>
        <Title level={4} style={{ color: 'var(--text-primary)', margin: 0 }}>
          开机记录
        </Title>
        <Space wrap>
          <Segmented<ViewMode>
            value={view}
            onChange={setView}
            options={[
              { label: '表格', value: 'table', icon: <TableOutlined /> },
              { label: '时间轴', value: 'timeline', icon: <FieldTimeOutlined /> },
            ]}
          />
          <Space.Compact>
            <Button icon={<FileTextOutlined />} onClick={() => exportCSV(filteredSessions)}>
              导出 CSV
            </Button>
            <Button
              icon={<FileExcelOutlined />}
              onClick={() => exportXLSX(filteredSessions)}
              disabled={filteredSessions.length === 0}
            >
              导出 XLSX
            </Button>
          </Space.Compact>
        </Space>
      </Space>

      {/* 筛选栏 */}
      <Space style={{ marginBottom: 16 }} wrap>
        <RangePicker
          value={dateRange as [Dayjs | null, Dayjs | null] | null}
          onChange={(v) => setDateRange(v as DateRange)}
          allowClear
        />
        <Button onClick={() => setDateRange(null)} disabled={!dateRange}>
          清除筛选
        </Button>
        <Button icon={<ReloadOutlined />} onClick={() => void refresh()} loading={loading}>
          刷新
        </Button>
        <Tooltip title="编辑/删除/合并/添加等管理操作请到「管理」页面">
          <Text type="secondary" style={{ fontSize: 12 }}>
            只读视图，管理操作请前往「管理」
          </Text>
        </Tooltip>
      </Space>

      {/* 主内容：表格 / 时间轴双视图 */}
      <Card
        style={{
          background: 'var(--bg-card)',
          borderColor: 'var(--border-color)',
        }}
        styles={{ body: { padding: 16 } }}
      >
        {view === 'table' ? (
          <Table<BootSession>
            rowKey="id"
            loading={loading}
            dataSource={filteredSessions}
            columns={baseColumns}
            scroll={{ x: 1000 }}
            pagination={{
              pageSize: 15,
              showSizeChanger: true,
              showTotal: (t) => `共 ${t} 条`,
              pageSizeOptions: ['10', '15', '30', '50', '100'],
            }}
            locale={{
              emptyText: (
                <EmptyState
                  title={dateRange ? '该时间段没有记录' : '等待第一次开机记录…'}
                  description={
                    dateRange
                      ? '换一个时间范围，或清除筛选查看全部记录。'
                      : '记录来自应用启动时自动采集；也可以手动补录历史数据。'
                  }
                  action={
                    <Button type="primary" ghost onClick={() => navigate('/admin')}>
                      前往管理页面添加
                    </Button>
                  }
                />
              ),
            }}
          />
        ) : filteredSessions.length === 0 ? (
          <EmptyState
            icon={<FieldTimeOutlined />}
            title={dateRange ? '该时间段没有记录' : '等待第一次开机记录…'}
            description={
              dateRange
                ? '换一个时间范围，或清除筛选查看全部记录。'
                : '记录来自应用启动时自动采集；时间轴会按天展示开机事件流。'
            }
            action={
              <Button type="primary" ghost onClick={() => navigate('/admin')}>
                前往管理页面添加
              </Button>
            }
          />
        ) : (
          <div>
            {timelineGroups.map((g) => (
              <div key={g.date}>
                <div className="tl-day-head">
                  <span className="tl-day-date">{g.label}</span>
                  <span className="tl-day-sum">
                    {g.items.length} 次 · 共 {fmtDuration(g.total)}
                  </span>
                </div>
                <div className="tl-list">
                  {g.items.map((s) => {
                    const active = !s.shutdownTime;
                    const dur = active
                      ? Math.max(0, now - new Date(s.bootTime).getTime())
                      : (s.duration ??
                        new Date(s.shutdownTime!).getTime() - new Date(s.bootTime).getTime());
                    return (
                      <div key={s.id} className={`tl-item${active ? ' tl-active' : ''}`}>
                        <span className="tl-dot" />
                        <span className="tl-time">
                          {fmtClock(s.bootTime)} → {s.shutdownTime ? fmtClock(s.shutdownTime) : '现在'}
                        </span>
                        <span className="tl-meta">
                          <span
                            className="tl-dur"
                            style={active ? { color: 'var(--accent)' } : undefined}
                          >
                            {fmtDuration(dur)}
                          </span>
                          {active ? (
                            <StatusPill tone="accent">进行中</StatusPill>
                          ) : (
                            <StatusPill tone="success">已关机</StatusPill>
                          )}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
            {filteredSessions.length > 100 && (
              <Text type="secondary" style={{ display: 'block', textAlign: 'center', padding: '12px 0 4px', fontSize: 12 }}>
                时间轴仅展示最近 100 条，更早记录请切换到表格视图
              </Text>
            )}
          </div>
        )}
      </Card>
    </Layout.Content>
  );
}

export default Records;
