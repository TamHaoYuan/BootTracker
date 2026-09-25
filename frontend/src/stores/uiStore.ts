import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type RecordsView = 'table' | 'timeline';

interface UiState {
  /** 侧栏折叠状态（桌面端） */
  sidebarCollapsed: boolean;
  /** 开机记录页视图模式 */
  recordsView: RecordsView;
  setSidebarCollapsed: (v: boolean) => void;
  setRecordsView: (v: RecordsView) => void;
}

/**
 * 用户 UI 偏好 — localStorage 持久化，跨会话记住。
 * 与 themeStore（主题）、settingsStore（后端 settings.json）分工：
 * 这里只放纯前端、不影响后端的界面状态。
 */
export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      sidebarCollapsed: true,
      recordsView: 'table',
      setSidebarCollapsed: (sidebarCollapsed) => set({ sidebarCollapsed }),
      setRecordsView: (recordsView) => set({ recordsView }),
    }),
    {
      name: 'boottracker-ui',
      partialize: (s) => ({ sidebarCollapsed: s.sidebarCollapsed, recordsView: s.recordsView }),
    },
  ),
);
