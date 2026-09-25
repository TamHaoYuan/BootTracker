import { useEffect, useState } from 'react';
import { useWidgetData } from './useWidgetData';
import { useWidgetTheme } from './useWidgetTheme';
import { useWidgetInteraction } from './useWidgetInteraction';
import { fmtDuration } from './format';

/** 开机时刻 HH:mm */
function fmtClock(iso: string): string {
  return new Date(iso).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });
}

/**
 * Mercury 单指标卡公式：hero 数字 + 一个辅助行，无图表无滚动。
 * 只回答一个问题："今天开机了吗？第几次、跑了多久？"
 */
function App() {
  const { todayCount, bootTime, online, countChanged } = useWidgetData();
  const { mode, theme } = useWidgetTheme();
  useWidgetInteraction();

  // 实时运行时长：每秒刷新
  const [elapsed, setElapsed] = useState<number>(0);
  useEffect(() => {
    if (!bootTime) {
      setElapsed(0);
      return;
    }
    const parsed = new Date(bootTime).getTime();
    if (Number.isNaN(parsed)) return;
    setElapsed(Date.now() - parsed);
    const t = setInterval(() => setElapsed(Date.now() - parsed), 1000);
    return () => clearInterval(t);
  }, [bootTime]);

  return (
    <div
      className={`card ${online ? '' : 'offline'} ${mode === 'light' ? 'light-mode' : ''}`}
      data-theme={theme}
    >
      <div className="accent-bar" />
      <div className="head">
        <div className={`dot ${online && bootTime ? 'on' : ''}`} />
        <div className="title">开机记录</div>
      </div>
      <div className="main">
        <div
          key={`${todayCount}-${countChanged}`}
          className={`count ${countChanged ? 'pop' : ''}`}
        >
          {todayCount}
        </div>
        <div className="label">今日开机</div>
      </div>
      <div className="time">
        {bootTime
          ? `${fmtClock(bootTime)} 开机 · 已运行 ${fmtDuration(elapsed)}`
          : online
            ? '今天还没有开机记录'
            : '—'}
      </div>
      <div className="foot">
        <span className="offline-msg">无法连接本地服务(18792)</span>
        <span className="brand">BootTracker</span>
      </div>
    </div>
  );
}

export default App;
