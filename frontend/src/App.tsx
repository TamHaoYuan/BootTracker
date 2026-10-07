import { lazy, useEffect, useMemo, useState } from 'react';
import { ConfigProvider, App as AntdApp } from 'antd';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import zhCN from 'antd/locale/zh_CN';

import { useThemeStore } from './stores/themeStore';
import { applyThemeToDom } from './stores/themeStore';
import { useUiScaleStore, applyUiScale, clampUiScale } from './stores/uiScaleStore';
import { buildAntdTheme } from './styles/antd-theme';
import { setupBridge, native } from './bridge';
import { settingsApi } from './api/settings';
import { useSettingsStore } from './stores/settingsStore';

import AppLayout from './layouts/AppLayout';
import Dashboard from './pages/Dashboard';
import NotFound from './pages/NotFound';
import BootSplash from './components/BootSplash';
import './styles/tokens.css';
import './styles/global.css';

// 路由级懒加载：Records/Charts/Settings/Admin 拆为按需 chunk，降低首屏 JS 解析量与内存
const Records = lazy(() => import('./pages/Records'));
const Charts = lazy(() => import('./pages/Charts'));
const Settings = lazy(() => import('./pages/Settings'));
const Admin = lazy(() => import('./pages/Admin'));

function App() {
  const { mode, theme } = useThemeStore();
  const [showSplash, setShowSplash] = useState(true);

  useEffect(() => {
    applyThemeToDom(mode, theme);
    // 启动即同步原生标题栏主题（DWM 沉浸式深色），失败静默（浏览器/桥未就绪）
    native('setTheme', { mode }).catch(() => {});
    void setupBridge();
    void (async () => {
      try {
        const s = await settingsApi.get();
        // mode/theme 均以前端（localStorage）为事实源：启动时回写后端
        // （供小组件同步取色），不再反向拉取翻转——避免 splash 播放中途
        // 背景/文字颜色突变闪色
        if (s.appMode !== mode) {
          settingsApi.update({ appMode: mode }).catch(() => {});
        }
        // 拉到 settings 后写入 settingsStore，便于 Layout 使用 customBgImage、defaultChartType 等
        useSettingsStore.getState().setSettings(s);
        if (s.appTheme !== theme) {
          settingsApi.update({ appTheme: theme }).catch(() => {});
        }
      } catch {
        // 后端不可用时保留前端持久化值
      }
    })();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    applyThemeToDom(mode, theme);
  }, [mode, theme]);

  // 页面缩放：初始化 + 跟随 store 变化落到 DOM
  useEffect(() => {
    applyUiScale(useUiScaleStore.getState().uiScale);
    return useUiScaleStore.subscribe((s) => applyUiScale(s.uiScale));
  }, []);

  // 缩放快捷键：Ctrl + 加号/减号/0；Ctrl + 滚轮（含触控板捏合）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      const st = useUiScaleStore.getState();
      if (e.key === '+' || e.key === '=') {
        st.nudgeUiScale(0.1);
      } else if (e.key === '-' || e.key === '_') {
        st.nudgeUiScale(-0.1);
      } else if (e.key === '0') {
        st.resetUiScale();
      } else {
        return;
      }
      // 阻止浏览器自身的页面缩放（原生缩放会把 fixed 侧栏与浮层一起缩放，
      // 且不会同步到 settings，导致两端表现不一致）
      e.preventDefault();
    };
    const onWheel = (e: WheelEvent) => {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      const st = useUiScaleStore.getState();
      st.setUiScale(clampUiScale(st.uiScale + (e.deltaY < 0 ? 0.05 : -0.05)));
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('wheel', onWheel, { passive: false });
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('wheel', onWheel);
    };
  }, []);

  const themeConfig = useMemo(() => buildAntdTheme(mode, theme), [mode, theme]);

  return (
    <ConfigProvider theme={themeConfig} locale={zhCN}>
      <AntdApp>
        {showSplash && <BootSplash onDone={() => setShowSplash(false)} />}
        <HashRouter>
          <Routes>
            <Route path="/" element={<AppLayout />}>
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="records" element={<Records />} />
              <Route path="charts" element={<Charts />} />
              <Route path="admin" element={<Admin />} />
              <Route path="settings" element={<Settings />} />
              <Route path="*" element={<NotFound />} />
            </Route>
          </Routes>
        </HashRouter>
      </AntdApp>
    </ConfigProvider>
  );
}

export default App;
