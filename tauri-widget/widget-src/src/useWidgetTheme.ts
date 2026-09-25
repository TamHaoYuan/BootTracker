import { useEffect, useState } from 'react';

export interface WidgetTheme {
  mode: 'dark' | 'light';
  /** 主题名（purple/blue/green/orange/gray/mica/material-you），决定强调色 */
  theme: string;
}

/**
 * 从后端 /api/settings 拉取主题设置，与主应用保持一致。
 * 30s 轮询：主页面切换主题/深浅色后，小组件无需重启即可跟随。
 */
export function useWidgetTheme(): WidgetTheme {
  const [widgetTheme, setWidgetTheme] = useState<WidgetTheme>({
    mode: 'dark',
    theme: 'mica',
  });

  useEffect(() => {
    let port = '18792';
    try {
      const stored = localStorage.getItem('bt_port');
      if (stored) port = stored;
    } catch {
      // localStorage 不可用时保持默认
    }

    const pull = () => {
      fetch(`http://127.0.0.1:${port}/api/settings`)
        .then((r) => r.json())
        .then((s: { appMode?: string; appTheme?: string }) => {
          setWidgetTheme({
            mode: s?.appMode === 'light' ? 'light' : 'dark',
            theme: s?.appTheme || 'mica',
          });
        })
        .catch(() => {
          // 后端不可用时保持当前值
        });
    };

    pull();
    const timer = setInterval(pull, 30_000);
    return () => clearInterval(timer);
  }, []);

  return widgetTheme;
}
