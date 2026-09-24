import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { AppMode, ThemeName } from '../styles/antd-theme';
import { isTauriEnvironment } from '../bridge';

interface ThemeState {
  mode: AppMode;
  theme: ThemeName;
  setMode: (mode: AppMode) => void;
  setTheme: (theme: ThemeName) => void;
  toggleMode: () => void;
}

/**
 * 主题单一事实源
 * - mode（dark/light）：需同步到后端 settings.appMode，并触发原生桥同步窗口背景色
 * - theme（purple/blue/...）：仅前端持久化
 */
export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      mode: 'dark',
      theme: 'mica',
      setMode: (mode) => set({ mode }),
      setTheme: (theme) => set({ theme }),
      toggleMode: () => set({ mode: get().mode === 'dark' ? 'light' : 'dark' }),
    }),
    {
      name: 'boottracker-theme',
      // 仅持久化主题选择，不持久化函数
      partialize: (s) => ({ mode: s.mode, theme: s.theme }),
      // 旧版本默认主题为 purple，新版本默认 Mica：仍停留在 purple 的
      // 存量用户视为未主动选择，随新默认迁移到 mica
      storage: createJSONStorage(() => ({
        getItem: (k) => {
          const raw = localStorage.getItem(k);
          if (raw) {
            try {
              const p = JSON.parse(raw);
              if (p?.state?.theme === 'purple') {
                p.state.theme = 'mica';
                return JSON.stringify(p);
              }
            } catch {
              /* 损坏数据交给 persist 默认回退 */
            }
          }
          return raw;
        },
        setItem: (k, v) => localStorage.setItem(k, v),
        removeItem: (k) => localStorage.removeItem(k),
      })),
    },
  ),
);

/** 将主题应用到 DOM：data-theme 属性 + body.light-mode 类 + 原生环境标记 */
export function applyThemeToDom(mode: AppMode, theme: ThemeName): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = theme;
  document.body.classList.toggle('light-mode', mode === 'light');
  // Tauri 原生窗口内标记 data-native：Mica 主题据此启用半透明玻璃底
  if (isTauriEnvironment()) {
    document.documentElement.dataset.native = 'tauri';
  }
}
