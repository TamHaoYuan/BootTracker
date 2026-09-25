import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { useThemeStore, applyThemeToDom } from './stores/themeStore';

const container = document.getElementById('root');
if (!container) {
  throw new Error('root element not found');
}

// 首帧渲染前应用持久化主题：避免 splash/首页首帧闪默认配色（如 Mica 用户闪紫色 accent）
{
  const { mode, theme } = useThemeStore.getState();
  applyThemeToDom(mode, theme);
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
