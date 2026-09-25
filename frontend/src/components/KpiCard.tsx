import type { ReactNode } from 'react';
import { Card, Skeleton } from 'antd';
import { ArrowUpOutlined, ArrowDownOutlined, MinusOutlined } from '@ant-design/icons';

export interface KpiTrend {
  /** up=变多/变好，down=变少，flat=持平 */
  dir: 'up' | 'down' | 'flat';
  text: string;
}

interface KpiCardProps {
  label: string;
  value: string | number;
  /** 趋势行：箭头 + 颜色双编码（Guitar Wiz 无障碍） */
  trend?: KpiTrend;
  /** 主卡：更大字号 + 主题渐变底 + 发光 */
  hero?: boolean;
  /** 数值用 accent 色 */
  accent?: boolean;
  /** 等宽字体（时长/数字） */
  mono?: boolean;
  loading?: boolean;
  /** 数值右侧附加内容（如状态胶囊） */
  extra?: ReactNode;
}

const CARD_BASE: React.CSSProperties = {
  padding: '16px 18px',
  borderRadius: 12,
  background: 'var(--bg-card)',
  borderColor: 'var(--border-color)',
  height: '100%',
};

const TREND_ICON = {
  up: <ArrowUpOutlined />,
  down: <ArrowDownOutlined />,
  flat: <MinusOutlined />,
} as const;

/**
 * KPI 卡（Stripe / 925 Studios 公式）：
 * 大数字（tabular-nums） + 标签（muted） + 趋势箭头（语义色）。
 */
function KpiCard({ label, value, trend, hero, accent, mono, loading, extra }: KpiCardProps) {
  const style: React.CSSProperties = hero
    ? {
        ...CARD_BASE,
        background:
          'linear-gradient(135deg, rgba(var(--accent-rgb), 0.18), rgba(var(--accent-rgb), 0.04))',
        borderColor: 'rgba(var(--accent-rgb), 0.35)',
      }
    : CARD_BASE;

  return (
    <Card className={hero ? 'today-boot-card' : undefined} style={style}>
      {loading ? (
        <Skeleton active title={{ width: '50%' }} paragraph={{ rows: 1, width: '70%' }} />
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          <span
            style={{
              color: hero ? 'var(--text-secondary)' : 'var(--text-muted)',
              fontSize: hero ? 13 : 12,
              letterSpacing: hero ? 0.5 : 0,
            }}
          >
            {label}
          </span>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <span
              className="tnum"
              style={{
                color: accent ? 'var(--accent)' : 'var(--text-primary)',
                fontWeight: 700,
                fontSize: hero ? 40 : 26,
                lineHeight: hero ? 1.15 : 1.25,
                ...(mono ? { fontFamily: "'JetBrains Mono', monospace" } : {}),
              }}
            >
              {value}
            </span>
            {extra}
          </div>
          {trend && (
            <span className={`kpi-trend trend-${trend.dir}`}>
              {TREND_ICON[trend.dir]}
              {trend.text}
            </span>
          )}
        </div>
      )}
    </Card>
  );
}

export default KpiCard;
