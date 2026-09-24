import { useEffect, useState } from 'react';

/** 从后端 /api/settings 拉取主题模式，与主应用同步 */
export function useWidgetTheme(): 'dark' | 'light' {
  const [mode, setMode] = useState<'dark' | 'light'>('dark');

  useEffect(() => {
    let port = '18792';
    try {
      const stored = localStorage.getItem('bt_port');
      if (stored) port = stored;
    } catch {
      // localStorage 不可用时保持默认
    }

    fetch(`http://127.0.0.1:${port}/api/settings`)
      .then((r) => r.json())
      .then((s: { appMode?: string }) => {
        if (s?.appMode === 'light') setMode('light');
      })
      .catch(() => {
        // 后端不可用时保持暗色默认
      });
  }, []);

  return mode;
}
