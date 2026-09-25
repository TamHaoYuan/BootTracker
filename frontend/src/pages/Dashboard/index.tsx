import { useEffect, useMemo, useState } from 'react';
import {
  Row,
  Col,
  Card,
  Table,
  Button,
  Space,
  App as AntdApp,
  Typography,
} from 'antd';
import type { TableProps } from 'antd';
import { PoweroffOutlined, ThunderboltOutlined } from '@ant-design/icons';

import { useBootData } from '../../hooks/useBootData';
import {
  fmtFullTime,
  fmtDuration,
  isoToLocalDate,
  getLocalToday,
  liveDuration,
} from '../../stores/sessionStore';
import { dataApi } from '../../api';
import type { BootSession } from '../../api/types';
import KpiCard from '../../components/KpiCard';
import TrendBars from '../../components/TrendBars';
import StatusPill from '../../components/StatusPill';
import EmptyState from '../../components/EmptyState';

const { Text } = Typography;

const CARD_STYLE: React.CSSProperties = {
  padding: '16px 18px',
  borderRadius: 12,
  background: 'var(--bg-card)',
  borderColor: 'var(--border-color)',
};

const DAY_MS = 86400000;

/** 会话时长：已结束用记录值，进行中算到当前时刻 */
function sessionDuration(s: BootSession, now: number): number {
  if (s.shutdownTime) {
    return (
      s.duration ??
      new Date(s.shutdownTime).getTime() - new Date(s.bootTime).getTime()
    );
  }
  return Math.max(0, now - new Date(s.bootTime).getTime());
}

/** 日期字符串偏移：yyyy-mm-dd ± n 天 */
function shiftDate(date: string, days: number): string {
  const t = new Date(`${date}T00:00:00`).getTime() + days * DAY_MS;
  return isoToLocalDate(new Date(t).toISOString());
}

function Dashboard() {
  const { message } = AntdApp.useApp();
  const { data: bootData, loading: bootLoading, refresh: refreshBoot } = useBootData();

  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);

  /* ---------- KPI 计算（全部由 sessions 前端推导，Stripe 卡条公式） ---------- */
  const kpi = useMemo(() => {
    const sessions = bootData.sessions;
    const today = getLocalToday();
    const yesterday = shiftDate(today, -1);

    // 按日期分桶（一次遍历）
    const byDay = new Map<string, BootSession[]>();
    for (const s of sessions) {
      const d = isoToLocalDate(s.bootTime);
      const list = byDay.get(d);
      if (list) list.push(s);
      else byDay.set(d, [s]);
    }

    const todaySessions = byDay.get(today) ?? [];
    const yesterdaySessions = byDay.get(yesterday) ?? [];

    const sumDuration = (list: BootSession[]): number =>
      list.reduce((acc, s) => acc + sessionDuration(s, now), 0);

    // 近 7 日 / 前 7 日开机次数（日均）
    let last7 = 0;
    let prev7 = 0;
    for (let i = 0; i < 7; i++) {
      last7 += (byDay.get(shiftDate(today, -i)) ?? []).length;
      prev7 += (byDay.get(shiftDate(today, -7 - i)) ?? []).length;
    }

    // 近 14 日趋势（缺日补 0，末位是今天）
    const trend14: { date: string; count: number }[] = [];
    for (let i = 13; i >= 0; i--) {
      const d = shiftDate(today, -i);
      trend14.push({ date: d, count: (byDay.get(d) ?? []).length });
    }

    return {
      todayBoots: todaySessions.length,
      yesterdayBoots: yesterdaySessions.length,
      todayUsage: sumDuration(todaySessions),
      yesterdayUsage: sumDuration(yesterdaySessions),
      avg7: last7 / 7,
      prevAvg7: prev7 / 7,
      trend14,
      trend14Total: trend14.reduce((acc, d) => acc + d.count, 0),
    };
  }, [bootData.sessions, now]);

  /* ---------- 最近会话（倒序取 5） ---------- */
  const recentSessions = useMemo(
    () =>
      [...bootData.sessions]
        .sort((a, b) => new Date(b.bootTime).getTime() - new Date(a.bootTime).getTime())
        .slice(0, 5),
    [bootData.sessions],
  );

  const activeSession = bootData.sessions.find((s) => !s.shutdownTime) ?? null;
  const activeCount = bootData.sessions.filter((s) => !s.shutdownTime).length;

  const handleRecordShutdown = async (): Promise<void> => {
    if (!activeSession) return;
    try {
      await dataApi.updateSession(activeSession.id, {
        shutdownTime: new Date().toISOString(),
      });
      message.success('已记录关机');
      await refreshBoot();
    } catch (e) {
      message.error(`记录关机失败：${(e as Error).message}`);
    }
  };

  const liveDurationText = activeSession ? liveDuration(activeSession.bootTime) : '—';

  /* ---------- 最近会话表格（结构化：数字右对齐 tabular-nums + 状态胶囊） ---------- */
  const tableColumns: TableProps<BootSession>['columns'] = [
    {
      title: '开机时间',
      dataIndex: 'bootTime',
      key: 'bootTime',
      render: (val: string) => (
        <span className="tnum" style={{ fontFamily: "'JetBrains Mono', monospace" }}>
          {fmtFullTime(val)}
        </span>
      ),
      width: 200,
    },
    {
      title: '关机时间',
      dataIndex: 'shutdownTime',
      key: 'shutdownTime',
      render: (val: string | null) => (
        <span
          className="tnum"
          style={{
            fontFamily: "'JetBrains Mono', monospace",
            color: val ? 'var(--text-primary)' : 'var(--text-muted)',
          }}
        >
          {val ? fmtFullTime(val) : '—'}
        </span>
      ),
      width: 200,
    },
    {
      title: '时长',
      key: 'duration',
      align: 'right',
      render: (_: unknown, record: BootSession) => (
        <span
          className="tnum"
          style={{
            color: record.shutdownTime ? 'var(--text-primary)' : 'var(--accent)',
            fontFamily: "'JetBrains Mono', monospace",
          }}
        >
          {fmtDuration(sessionDuration(record, now))}
        </span>
      ),
      width: 140,
    },
    {
      title: '状态',
      key: 'status',
      render: (_: unknown, record: BootSession) =>
        record.shutdownTime ? (
          <StatusPill tone="muted">已结束</StatusPill>
        ) : (
          <StatusPill tone="accent">进行中</StatusPill>
        ),
      width: 110,
    },
  ];

  const bootDiff = kpi.todayBoots - kpi.yesterdayBoots;

  return (
    <div style={{ position: 'relative' }}>
      {/* ===== KPI 卡条（Stripe 公式：大数字 + 标签 + 趋势箭头） ===== */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12} lg={6}>
          <KpiCard
            hero
            accent
            loading={bootLoading}
            label="今日开机"
            value={kpi.todayBoots}
            trend={
              bootDiff > 0
                ? { dir: 'up', text: `较昨日 +${bootDiff} 次` }
                : bootDiff < 0
                  ? { dir: 'down', text: `较昨日 ${bootDiff} 次` }
                  : { dir: 'flat', text: '与昨日持平' }
            }
          />
        </Col>
        <Col xs={24} sm={12} lg={5}>
          <KpiCard
            mono
            loading={bootLoading}
            label="今日使用时长"
            value={fmtDuration(kpi.todayUsage)}
            trend={{ dir: 'flat', text: `昨日 ${fmtDuration(kpi.yesterdayUsage)}` }}
          />
        </Col>
        <Col xs={12} sm={12} lg={4}>
          <KpiCard
            loading={bootLoading}
            label="7 日日均开机"
            value={kpi.avg7.toFixed(1)}
            trend={{
              dir:
                kpi.avg7 > kpi.prevAvg7
                  ? 'up'
                  : kpi.avg7 < kpi.prevAvg7
                    ? 'down'
                    : 'flat',
              text: `前 7 日 ${kpi.prevAvg7.toFixed(1)}`,
            }}
          />
        </Col>
        <Col xs={12} sm={12} lg={4}>
          <KpiCard
            loading={bootLoading}
            label="累计记录"
            value={bootData.bootCount ?? bootData.sessions.length}
          />
        </Col>
        <Col xs={24} sm={12} lg={5}>
          <KpiCard
            loading={bootLoading}
            label="进行中会话"
            value={activeCount}
            extra={
              activeCount > 0 ? (
                <StatusPill tone="accent">使用中</StatusPill>
              ) : (
                <StatusPill tone="muted">无</StatusPill>
              )
            }
          />
        </Col>
      </Row>

      {/* ===== 主趋势图 + 本次会话（一图表一操作，拒绝图表墙） ===== */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} lg={14}>
          <Card
            style={CARD_STYLE}
            title={
              <Space>
                <ThunderboltOutlined style={{ color: 'var(--accent)' }} />
                <span>近 14 日开机趋势</span>
              </Space>
            }
            extra={
              <Text type="secondary" className="tnum" style={{ fontSize: 12 }}>
                共 {kpi.trend14Total} 次
              </Text>
            }
          >
            {kpi.trend14Total === 0 && !bootLoading ? (
              <EmptyState
                title="近 14 日没有开机记录"
                description="应用会在每次启动时自动记录开机时间，数据积累后这里会展示趋势。"
              />
            ) : (
              <TrendBars data={kpi.trend14} height={130} />
            )}
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card style={{ ...CARD_STYLE, height: '100%' }} title="本次会话">
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '7px 0',
                borderBottom: '1px solid var(--border-light)',
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>本次开机时间</Text>
              <Text
                strong
                className="tnum"
                style={{ color: 'var(--text-primary)', fontFamily: "'JetBrains Mono', monospace", fontSize: 13 }}
              >
                {fmtFullTime(activeSession?.bootTime ?? null)}
              </Text>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '7px 0',
                borderBottom: '1px solid var(--border-light)',
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>当前状态</Text>
              {activeSession ? (
                <StatusPill tone="accent">进行中</StatusPill>
              ) : (
                <StatusPill tone="muted">已结束</StatusPill>
              )}
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '7px 0',
                marginBottom: 12,
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>已运行时长</Text>
              <Text
                strong
                className="tnum"
                style={{
                  color: 'var(--accent)',
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: 18,
                }}
              >
                {liveDurationText}
              </Text>
            </div>
            <Button
              type="primary"
              danger
              icon={<PoweroffOutlined />}
              block
              size="large"
              disabled={!activeSession}
              onClick={() => void handleRecordShutdown()}
              style={{ height: 48, fontSize: 15, fontWeight: 600 }}
            >
              记录关机
            </Button>
          </Card>
        </Col>
      </Row>

      {/* ===== 最近会话（表格回归） ===== */}
      <Card
        style={CARD_STYLE}
        title={
          <Space>
            <span>最近会话</span>
            <Text type="secondary" style={{ fontSize: 12 }}>
              （最近 5 条）
            </Text>
          </Space>
        }
        extra={
          <Button size="small" onClick={() => void refreshBoot()} loading={bootLoading}>
            刷新
          </Button>
        }
      >
        <Table<BootSession>
          rowKey="id"
          columns={tableColumns}
          dataSource={recentSessions}
          pagination={false}
          size="middle"
          loading={bootLoading}
          locale={{
            emptyText: (
              <EmptyState
                title="等待第一次开机记录…"
                description="记录来自应用启动时自动采集；也可以到「管理」页面手动补录历史数据。"
              />
            ),
          }}
        />
      </Card>
    </div>
  );
}

export default Dashboard;
