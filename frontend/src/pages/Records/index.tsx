import { useEffect, useMemo, useState } from 'react';
// useEffect 用于下方定时器
import {
  Badge,
  Button,
  Card,
  DatePicker,
  Layout,
  Space,
  Table,
  Tag,
  Tooltip,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table/interface';
import {
  FileExcelOutlined,
  FileTextOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import { useNavigate } from 'react-router-dom';
import dayjs, { Dayjs } from 'dayjs';

import * as XLSX from 'xlsx';

import { useBootData } from '../../hooks/useBootData';
import { fmtFullTime, fmtDuration, liveDuration } from '../../stores/sessionStore';
import type { BootSession } from '../../api/types';

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

function Records() {
  const navigate = useNavigate();
  const { data, loading, refresh } = useBootData();

  /* ---------- 状态 ---------- */
  const [dateRange, setDateRange] = useState<DateRange>(null);
  const [now, setNow] = useState<number>(Date.now());

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

  /* ---------- 表格列（只读） ---------- */
  const baseColumns: ColumnsType<BootSession> = [
    {
      title: '序号',
      key: 'idx',
      width: 70,
      render: (_v, _r, idx) => idx + 1,
    },
    {
      title: '开机时间',
      dataIndex: 'bootTime',
      key: 'bootTime',
      width: 190,
      render: (v: string) => (
        <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>{fmtFullTime(v)}</span>
      ),
    },
    {
      title: '关机时间',
      dataIndex: 'shutdownTime',
      key: 'shutdownTime',
      width: 190,
      render: (v: string | null) =>
        v ? (
          <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>{fmtFullTime(v)}</span>
        ) : (
          <Badge status="processing" text={<Text type="success">进行中</Text>} />
        ),
    },
    {
      title: '会话时长',
      key: 'duration',
      width: 150,
      render: (_v, record) => {
        if (!record.shutdownTime) {
          const dur = now - new Date(record.bootTime).getTime();
          return (
            <Badge status="processing">
              <span style={{ fontFamily: 'JetBrains Mono, monospace', color: '#1677ff' }}>
                {dur > 0 ? fmtDuration(dur) : liveDuration(record.bootTime)}
              </span>
            </Badge>
          );
        }
        return (
          <span style={{ fontFamily: 'JetBrains Mono, monospace' }}>
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
          <Tag color="green">已关机</Tag>
        ) : (
          <Tag color="blue">进行中</Tag>
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

      {/* 主表格（只读） */}
      <Card
        style={{
          background: 'var(--bg-card)',
          borderColor: 'var(--border-color)',
        }}
        styles={{ body: { padding: 16 } }}
      >
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
        />
      </Card>

      {/* 提示用户管理操作的位置（首次进入时） */}
      {filteredSessions.length === 0 && !loading && (
        <Card
          size="small"
          style={{
            marginTop: 16,
            background: 'var(--bg-card)',
            borderColor: 'var(--border-color)',
          }}
        >
          <Space>
            <Text type="secondary">暂无记录。</Text>
            <Button type="link" size="small" onClick={() => navigate('/admin')}>
              前往管理页面添加
            </Button>
          </Space>
        </Card>
      )}
    </Layout.Content>
  );
}

export default Records;
