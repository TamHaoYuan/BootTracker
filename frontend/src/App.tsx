import { useEffect, useMemo, useState } from 'react';
import { ConfigProvider, App as AntdApp } from 'antd';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import zhCN from 'antd/locale/zh_CN';

import { useThemeStore } from './stores/themeStore';
import { applyThemeToDom } from './stores/themeStore';
import { buildAntdTheme } from './styles/antd-theme';
import { useTheme } from './hooks/useTheme';
import { setupBridge } from './bridge';
import { settingsApi } from './api/settings';
import { useSettingsStore } from './stores/settingsStore';

import AppLayout from './layouts/AppLayout';
import Dashboard from './pages/Dashboard';
import Records from './pages/Records';
import Charts from './pages/Charts';
import Settings from './pages/Settings';
import Admin from './pages/Admin';
import NotFound from './pages/NotFound';
import BootSplash from './components/BootSplash';
import './styles/tokens.css';
import './styles/global.css';

function App() {
  const { mode, theme } = useThemeStore();
  const { applyMode } = useTheme();
  const [showSplash, setShowSplash] = useState(true);

  useEffect(() => {
    applyThemeToDom(mode, theme);
    void setupBridge();
    void (async () => {
      try {
        const s = await settingsApi.get();
        if (s.appMode && s.appMode !== mode) {
          void applyMode(s.appMode);
        }
        // 拉到 settings 后写入 settingsStore，便于 Layout 使用 customBgImage、defaultChartType 等
        useSettingsStore.getState().setSettings(s);
      } catch {
        // 后端不可用时保留前端持久化值
      }
    })();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    applyThemeToDom(mode, theme);
  }, [mode, theme]);

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
