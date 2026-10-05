import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type RecordsView = 'table' | 'timeline';

interface UiState {
  /** 开机记录页视图模式 */
  recordsView: RecordsView;
  setRecordsView: (v: RecordsView) => void;
}

/**
 * 用户 UI 偏好 — localStorage 持久化，跨会话记住。
 * 与 themeStore（主题）、settingsStore（后端 settings.json）分工：
 * 这里只放纯前端、不影响后端的界面状态。
 *
 * 注：侧栏折叠状态已从此处移除——桌面端侧栏改为「常态折叠 + 鼠标悬浮临时
 * 展开」，是瞬时状态，不需要也不应该持久化。
 */
export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      recordsView: 'table',
      setRecordsView: (recordsView) => set({ recordsView }),
    }),
    {
      name: 'boottracker-ui',
      partialize: (s) => ({ recordsView: s.recordsView }),
    },
  ),
);
