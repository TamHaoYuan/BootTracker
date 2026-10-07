import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { useThemeStore, applyThemeToDom } from './stores/themeStore';
import { useUiScaleStore, applyUiScale } from './stores/uiScaleStore';

const container = document.getElementById('root');
if (!container) {
  throw new Error('root element not found');
}

// 首帧渲染前应用持久化主题：避免 splash/首页首帧闪默认配色（如 Mica 用户闪紫色 accent）
{
  const { mode, theme } = useThemeStore.getState();
  applyThemeToDom(mode, theme);
}

// 同理，首帧渲染前应用持久化缩放：否则先按 100% 画一帧再跳到目标比例，肉眼可见抖动
applyUiScale(useUiScaleStore.getState().uiScale);

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
