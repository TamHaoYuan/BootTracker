import { useEffect, useRef, useState } from 'react';

export interface WidgetData {
  todayCount: number;
  bootTime: string | null;
  online: boolean;
}

const EMPTY: WidgetData = { todayCount: 0, bootTime: null, online: false };

/** 监听 Tauri 后端推送的 widget-data 事件 */
export function useWidgetData(): WidgetData {
  const [data, setData] = useState<WidgetData>(EMPTY);
  const prevCountRef = useRef(0);

  useEffect(() => {
    const ev = window.__TAURI__?.event;
    if (!ev?.listen) return;

    let unlisten: (() => void) | undefined;
    ev.listen<WidgetDataPayload>('widget-data', (e) => {
      const p = e.payload;
      setData({
        todayCount: p.today_count ?? 0,
        bootTime: p.boot_time ?? null,
        online: p.online ?? false,
      });
    }).then((un) => {
      unlisten = un;
    }).catch(() => {});

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // 检测计数变化以触发 pop 动画
  const countChanged = data.todayCount !== prevCountRef.current;
  prevCountRef.current = data.todayCount;

  return { ...data, countChanged } as WidgetData & { countChanged: boolean };
}
