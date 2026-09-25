interface TrendPoint {
  /** yyyy-mm-dd */
  date: string;
  count: number;
}

interface TrendBarsProps {
  data: TrendPoint[];
  height?: number;
}

/**
 * 主趋势图（Plausible 模式）：一张克制的柱状图回答"最近开机情况正常吗"。
 * 纯 div 实现零依赖；仅"今天"用 accent 实底，其余低饱和——accent 只强调当前。
 */
function TrendBars({ data, height = 120 }: TrendBarsProps) {
  const max = Math.max(1, ...data.map((d) => d.count));
  const lastIdx = data.length - 1;

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'flex-end', gap: 6, height }}>
        {data.map((d, i) => {
          const pct = (d.count / max) * 100;
          const isToday = i === lastIdx;
          return (
            <div
              key={d.date}
              title={`${d.date}：开机 ${d.count} 次`}
              style={{
                flex: 1,
                height: '100%',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'flex-end',
              }}
            >
              <div
                style={{
                  height: d.count > 0 ? `${Math.max(pct, 4)}%` : 2,
                  borderRadius: 4,
                  background: isToday
                    ? 'var(--accent)'
                    : 'rgba(var(--accent-rgb), 0.28)',
                  transition: 'height 0.5s var(--ease-out-expo), background-color 0.3s ease',
                }}
              />
            </div>
          );
        })}
      </div>
      {/* 日期刻度：首 / 中 / 尾 */}
      <div
        className="tnum"
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          marginTop: 8,
          fontSize: 11,
          color: 'var(--text-muted)',
        }}
      >
        <span>{data[0]?.date.slice(5)}</span>
        <span>{data[Math.floor(lastIdx / 2)]?.date.slice(5)}</span>
        <span style={{ color: 'var(--accent-light)' }}>今天</span>
      </div>
    </div>
  );
}

export default TrendBars;
