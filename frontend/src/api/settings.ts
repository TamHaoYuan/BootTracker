import { http } from './client';
import type { AppSettings, UpdatedResponse, WidgetSyncResponse } from './types';

export const settingsApi = {
  get: () => http.get<AppSettings>('/settings'),

  update: (updates: Partial<AppSettings>) =>
    http.put<UpdatedResponse>('/settings', updates),

  /**
   * 立即同步最新设置/数据到桌面小组件。
   * 小组件是独立进程，靠轮询拉取（30s 数据 / 60s 设置），改完设置不会立刻生效；
   * 调用后会重启浮窗进程，启动时立即拉一次最新值。
   */
  widgetSync: () => http.post<WidgetSyncResponse>('/widget-sync'),
};
