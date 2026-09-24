import { useCallback, useEffect, useRef } from 'react';
import { useSessionStore } from '../stores/sessionStore';
import { dataApi } from '../api/data';
import type { BootData } from '../api/types';

/** 数据加载与刷新 */
export function useBootData() {
  const { data, loading, error, setData, setLoading, setError } = useSessionStore();

  // 静默刷新：不触发 loading，避免轮询时界面闪烁
  const silentRefresh = useCallback(async (): Promise<BootData | null> => {
    try {
      const d = await dataApi.getData();
      setData(d);
      return d;
    } catch (e) {
      setError((e as Error).message);
      return null;
    }
  }, [setData, setError]);

  // 手动刷新：触发 loading（用户点击刷新按钮时）
  const refresh = useCallback(async (): Promise<BootData | null> => {
    setLoading(true);
    try {
      const d = await dataApi.getData();
      setData(d);
      return d;
    } catch (e) {
      setError((e as Error).message);
      return null;
    } finally {
      setLoading(false);
    }
  }, [setData, setLoading, setError]);

  // 首次挂载加载
  useEffect(() => {
    void refresh();
  }, [refresh]);

  // 自动轮询：每 5 秒静默同步一次，与小组件（3秒轮询）保持短延迟同步
  // 用 ref 持有最新 silentRefresh，避免 effect 频繁重建定时器
  const refreshRef = useRef(silentRefresh);
  refreshRef.current = silentRefresh;
  useEffect(() => {
    const timer = setInterval(() => {
      void refreshRef.current();
    }, 5000);
    return () => clearInterval(timer);
  }, []);

  return { data, loading, error, refresh };
}
