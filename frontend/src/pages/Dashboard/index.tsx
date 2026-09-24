import { useEffect, useState, useCallback } from 'react';
import {
  Row,
  Col,
  Card,
  Statistic,
  Table,
  Tag,
  Badge,
  Button,
  Space,
  App as AntdApp,
  Typography,
} from 'antd';
import type { TableProps } from 'antd';
import {
  DashboardOutlined,
  ThunderboltOutlined,
  PoweroffOutlined,
  ClockCircleOutlined,
  PlayCircleOutlined,
} from '@ant-design/icons';

import { useBootData } from '../../hooks/useBootData';
import {
  fmtFullTime,
  fmtDuration,
  countTodayBoots,
  liveDuration,
} from '../../stores/sessionStore';
import { dataApi, statsApi } from '../../api';
import type { BootSession, OverviewStats } from '../../api/types';

const { Text } = Typography;

const CARD_STYLE: React.CSSProperties = {
  padding: '16px 18px',
  borderRadius: 12,
  background: 'var(--bg-card)',
  borderColor: 'var(--border-color)',
};

interface DashboardStats {
  totalBoot: number;
  totalShutdown: number;
  avgDuration: number;
  activeCount: number;
  todayBoots: number;
  recentSessions: BootSession[];
}

const EMPTY_STATS: DashboardStats = {
  totalBoot: 0,
  totalShutdown: 0,
  avgDuration: 0,
  activeCount: 0,
  todayBoots: 0,
  recentSessions: [],
};

function Dashboard() {
  const { message } = AntdApp.useApp();
  const { data: bootData, loading: bootLoading, refresh: refreshBoot } = useBootData();

  const [stats, setStats] = useState<DashboardStats>(EMPTY_STATS);
  const [statsLoading, setStatsLoading] = useState(false);
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(t);
  }, []);

  const loadStats = useCallback(async (): Promise<void> => {
    setStatsLoading(true);
    try {
      const overview = await statsApi.overview();
      const o: OverviewStats = overview;
      const sessions = bootData.sessions.length > 0 ? bootData.sessions : o.recentSessions ?? [];
      setStats({
        totalBoot: o.totalBoot ?? bootData.bootCount ?? 0,
        totalShutdown: o.totalShutdown ?? bootData.shutdownCount ?? 0,
        avgDuration: o.avgDuration ?? 0,
        activeCount: o.activeCount ?? sessions.filter((s) => !s.shutdownTime).length,
        todayBoots: countTodayBoots(sessions),
        recentSessions: o.recentSessions ?? sessions.slice(-5).reverse(),
      });
    } catch (e) {
      message.error(`加载概览数据失败：${(e as Error).message}`);
      const sessions = bootData.sessions;
      const finished = sessions.filter((s) => s.shutdownTime);
      const totalDur = finished.reduce((acc, s) => acc + (s.duration ?? 0), 0);
      setStats({
        totalBoot: bootData.bootCount ?? 0,
        totalShutdown: bootData.shutdownCount ?? 0,
        avgDuration: finished.length > 0 ? totalDur / finished.length : 0,
        activeCount: sessions.filter((s) => !s.shutdownTime).length,
        todayBoots: countTodayBoots(sessions),
        recentSessions: sessions.slice(-5).reverse(),
      });
    } finally {
      setStatsLoading(false);
    }
  }, [bootData, message]);

  useEffect(() => {
    void loadStats();
  }, [loadStats]);

  const refreshAll = useCallback(async (): Promise<void> => {
    const b = await refreshBoot();
    if (b !== null) {
      await loadStats();
    }
  }, [refreshBoot, loadStats]);

  const activeSession = bootData.sessions.find((s) => !s.shutdownTime) ?? null;

  const handleRecordShutdown = async (): Promise<void> => {
    if (!activeSession) return;
    try {
      await dataApi.updateSession(activeSession.id, {
        shutdownTime: new Date().toISOString(),
      });
      message.success('已记录关机');
      await refreshAll();
    } catch (e) {
      message.error(`记录关机失败：${(e as Error).message}`);
    }
  };

  const liveDurationText = activeSession
    ? liveDuration(activeSession.bootTime)
    : '—';

  const tableColumns: TableProps<BootSession>['columns'] = [
    {
      title: '开机时间',
      dataIndex: 'bootTime',
      key: 'bootTime',
      render: (val: string) => (
        <span style={{ color: 'var(--text-primary)' }}>{fmtFullTime(val)}</span>
      ),
      width: 200,
    },
    {
      title: '关机时间',
      dataIndex: 'shutdownTime',
      key: 'shutdownTime',
      render: (val: string | null) => (
        <span style={{ color: val ? 'var(--text-primary)' : 'var(--text-muted)' }}>
          {fmtFullTime(val)}
        </span>
      ),
      width: 200,
    },
    {
      title: '时长',
      key: 'duration',
      render: (_: unknown, record: BootSession) => {
        const d = record.shutdownTime
          ? (record.duration ??
              (new Date(record.shutdownTime).getTime() - new Date(record.bootTime).getTime()))
          : now - new Date(record.bootTime).getTime();
        return (
          <span
            style={{
              color: 'var(--accent)',
              fontFamily: "'JetBrains Mono', monospace",
            }}
          >
            {fmtDuration(d)}
          </span>
        );
      },
      width: 140,
    },
    {
      title: '状态',
      key: 'status',
      render: (_: unknown, record: BootSession) =>
        record.shutdownTime ? (
          <Tag color="default">已结束</Tag>
        ) : (
          <Badge status="processing" text={<Tag color="success">进行中</Tag>} />
        ),
      width: 120,
    },
  ];

  const recentForTable = stats.recentSessions.slice(0, 5);

  return (
    <div style={{ position: 'relative' }}>
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12} md={12} lg={8}>
          <Card
            className="today-boot-card"
            style={{
              ...CARD_STYLE,
              background: 'linear-gradient(135deg, rgba(var(--accent-rgb), 0.18), rgba(var(--accent-rgb), 0.04))',
              borderColor: 'rgba(var(--accent-rgb), 0.35)',
            }}
            loading={statsLoading || bootLoading}
          >
            <Statistic
              title={
                <span style={{ color: 'var(--text-secondary)', fontSize: 13, letterSpacing: 0.5 }}>
                  今日开机
                </span>
              }
              value={stats.todayBoots}
              prefix={<ThunderboltOutlined style={{ color: 'var(--accent)' }} />}
              valueStyle={{
                color: 'var(--accent)',
                fontWeight: 700,
                fontSize: 40,
                fontFamily: "'JetBrains Mono', monospace",
              }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6} lg={4}>
          <Card style={CARD_STYLE} loading={statsLoading || bootLoading}>
            <Statistic
              title={<span style={{ color: 'var(--text-muted)' }}>累计开机</span>}
              value={stats.totalBoot}
              prefix={<DashboardOutlined style={{ color: 'var(--text-success-dark)' }} />}
              valueStyle={{ color: 'var(--text-primary)', fontWeight: 700 }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6} lg={4}>
          <Card style={CARD_STYLE} loading={statsLoading || bootLoading}>
            <Statistic
              title={<span style={{ color: 'var(--text-muted)' }}>累计关机</span>}
              value={stats.totalShutdown}
              prefix={<PoweroffOutlined style={{ color: 'var(--text-danger-dark)' }} />}
              valueStyle={{ color: 'var(--text-primary)', fontWeight: 700 }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6} lg={4}>
          <Card style={CARD_STYLE} loading={statsLoading || bootLoading}>
            <Statistic
              title={<span style={{ color: 'var(--text-muted)' }}>平均会话时长</span>}
              value={fmtDuration(stats.avgDuration)}
              prefix={<ClockCircleOutlined style={{ color: 'var(--accent2)' }} />}
              valueStyle={{
                color: 'var(--text-primary)',
                fontWeight: 700,
                fontFamily: "'JetBrains Mono', monospace",
                fontSize: 20,
              }}
            />
          </Card>
        </Col>
        <Col xs={24} sm={12} md={6} lg={4}>
          <Card style={CARD_STYLE} loading={statsLoading || bootLoading}>
            <Statistic
              title={<span style={{ color: 'var(--text-muted)' }}>进行中会话</span>}
              value={stats.activeCount}
              prefix={<PlayCircleOutlined style={{ color: 'var(--text-success-dark)' }} />}
              valueStyle={{ color: 'var(--text-success)', fontWeight: 700 }}
            />
          </Card>
        </Col>
      </Row>

      <Card style={{ ...CARD_STYLE, marginBottom: 16 }} title="本次会话">
        <Row gutter={[24, 16]}>
          <Col xs={24} md={12}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '8px 0',
                borderBottom: '1px solid var(--border-light)',
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>本次开机时间</Text>
              <Text strong style={{ color: 'var(--text-primary)' }}>
                {fmtFullTime(activeSession?.bootTime ?? null)}
              </Text>
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '8px 0',
                borderBottom: '1px solid var(--border-light)',
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>当前状态</Text>
              {activeSession ? (
                <Badge status="processing" text={<Tag color="success">进行中</Tag>} />
              ) : (
                <Tag color="default">已结束</Tag>
              )}
            </div>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                padding: '8px 0',
              }}
            >
              <Text style={{ color: 'var(--text-muted)' }}>已运行时长</Text>
              <Text
                strong
                style={{
                  color: 'var(--accent)',
                  fontFamily: "'JetBrains Mono', monospace",
                  fontSize: 18,
                }}
              >
                {liveDurationText}
              </Text>
            </div>
          </Col>
          <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'center' }}>
            <Button
              type="primary"
              danger
              icon={<PoweroffOutlined />}
              block
              size="large"
              disabled={!activeSession}
              onClick={() => void handleRecordShutdown()}
              style={{ height: 64, fontSize: 16, fontWeight: 600 }}
            >
              记录关机
            </Button>
          </Col>
        </Row>
      </Card>

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
          <Button size="small" onClick={() => void refreshAll()} loading={bootLoading || statsLoading}>
            刷新
          </Button>
        }
      >
        <Table<BootSession>
          rowKey="id"
          columns={tableColumns}
          dataSource={recentForTable}
          pagination={false}
          size="middle"
          loading={bootLoading || statsLoading}
        />
      </Card>
    </div>
  );
}

export default Dashboard;
