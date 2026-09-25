import type { ReactNode } from 'react';
import { InboxOutlined } from '@ant-design/icons';

interface EmptyStateProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  /** 可选操作按钮（引导下一步） */
  action?: ReactNode;
}

/**
 * 空状态引导卡（Vercel 模式）：无数据时展示引导而非空白表格。
 * 结构：图标 + 主文案 + 数据来源说明 + 可选动作。
 */
function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <div className="empty-icon">{icon ?? <InboxOutlined />}</div>
      <div className="empty-title">{title}</div>
      {description && <div className="empty-desc">{description}</div>}
      {action && <div className="empty-action">{action}</div>}
    </div>
  );
}

export default EmptyState;
