import type { ReactNode } from 'react';

export type PillTone = 'success' | 'accent' | 'muted' | 'danger';

interface StatusPillProps {
  tone: PillTone;
  children: ReactNode;
  /** 隐藏圆点（纯文字胶囊） */
  noDot?: boolean;
}

/**
 * 状态胶囊（Stripe 表格模式）：低饱和底 + 高饱和字 + 状态圆点。
 * tone 语义：success=已关机/正常，accent=进行中（呼吸动画），muted=中性，danger=警示。
 */
function StatusPill({ tone, children, noDot }: StatusPillProps) {
  return (
    <span className={`status-pill pill-${tone}`}>
      {!noDot && <span className="pill-dot" />}
      {children}
    </span>
  );
}

export default StatusPill;
