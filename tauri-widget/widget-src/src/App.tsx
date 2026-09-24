import { useEffect, useState } from 'react';
import { useWidgetData } from './useWidgetData';
import { useWidgetTheme } from './useWidgetTheme';
import { useWidgetInteraction } from './useWidgetInteraction';
import { fmtDuration } from './format';

function App() {
  const { todayCount, bootTime, online, countChanged } = useWidgetData();
  const mode = useWidgetTheme();
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
    <div className={`card ${online ? '' : 'offline'} ${mode === 'light' ? 'light-mode' : ''}`}>
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
      <div className="time">运行时长 {bootTime ? fmtDuration(elapsed) : '—'}</div>
      <div className="foot">
        <span className="offline-msg">● 连接中…</span>
        <span className="brand">BootTracker</span>
      </div>
    </div>
  );
}

export default App;
