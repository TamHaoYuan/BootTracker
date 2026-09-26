import { memo, useState } from 'react';
import type { CSSProperties } from 'react';
import { liveDuration } from '../stores/sessionStore';
import { useVisibleInterval } from '../hooks/useVisibleInterval';

interface LiveDurationProps {
  /** 进行中会话的开机时间（ISO）；为 null 时显示 fallback */
  bootTime: string | null;
  /** bootTime 为空时的占位文本 */
  fallback?: string;
  className?: string;
  style?: CSSProperties;
}

/**
 * 实时运行时长（每秒自刷新）叶子组件。
 *
 * 独立 tick，仅重渲染自身，避免父组件（含大表格 / 整页）因每秒 setState 而重渲染。
 * 窗口隐藏时自动暂停（useVisibleInterval）。用 memo 包裹，父级重渲染时若 props 不变
 * 亦不重渲染。
 */
function LiveDuration({ bootTime, fallback = '—', className, style }: LiveDurationProps) {
  const [, setTick] = useState(0);
  useVisibleInterval(() => setTick((t) => t + 1), 1000);
  const text = bootTime ? liveDuration(bootTime) : fallback;
  return (
    <span className={className} style={style}>
      {text}
    </span>
  );
}

export default memo(LiveDuration);
