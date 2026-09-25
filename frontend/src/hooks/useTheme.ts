import { useCallback } from 'react';
import { useThemeStore, applyThemeToDom } from '../stores/themeStore';
import { settingsApi } from '../api/settings';
import { native, BusinessError } from '../bridge';
import type { AppMode, ThemeName } from '../styles/antd-theme';

/**
 * 主题切换 Hook
 * 切换 mode 时同步：
 * 1. 前端 DOM（data-theme + body.light-mode）
 * 2. Ant Design token（由组件订阅 useThemeStore 触发）
 * 3. 后端 settings.appMode（持久化）
 * 4. 原生桥（同步窗口背景色与 DWM 标题栏）
 */
export function useTheme() {
  const { mode, theme, setMode, setTheme } = useThemeStore();

  const applyMode = useCallback(
    async (newMode: AppMode) => {
      // 1. 前端立即生效
      setMode(newMode);
      applyThemeToDom(newMode, theme);

      // 2. 后端持久化（失败不阻塞，仅提示）
      settingsApi.update({ appMode: newMode }).catch((e) => {
        console.warn('[theme] persist appMode failed:', e);
      });

      // 3. 原生桥同步窗口背景色
      try {
        await native('setTheme', { mode: newMode });
      } catch (e) {
        if (e instanceof BusinessError) {
          // 业务错误：桥已就绪但方法失败，提示
          console.warn('[theme] native setTheme business error:', e.code);
        }
        // BridgeError（浏览器/mock/未就绪）：静默，前端已自管主题
      }
    },
    [setMode, theme],
  );

  const applyTheme = useCallback(
    (newTheme: ThemeName) => {
      setTheme(newTheme);
      applyThemeToDom(mode, newTheme);
      // 同步到后端 settings.appTheme：桌面小组件据此跟随主题取色
      settingsApi.update({ appTheme: newTheme }).catch((e) => {
        console.warn('[theme] persist appTheme failed:', e);
      });
    },
    [setTheme, mode],
  );

  const toggleMode = useCallback(() => {
    void applyMode(mode === 'dark' ? 'light' : 'dark');
  }, [applyMode, mode]);

  return { mode, theme, applyMode, applyTheme, toggleMode };
}

/** 启动时同步 DOM 与 store */
export function useThemeBootstrap() {
  const { mode, theme } = useThemeStore();
  return useCallback(() => applyThemeToDom(mode, theme), [mode, theme]);
}


