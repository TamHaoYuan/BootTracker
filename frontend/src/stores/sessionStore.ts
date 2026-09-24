import { create } from 'zustand';
import type { BootData, BootSession } from '../api/types';

interface SessionState {
  data: BootData;
  loading: boolean;
  error: string | null;
  setData: (d: BootData) => void;
  setLoading: (b: boolean) => void;
  setError: (e: string | null) => void;
  /** 获取当前活跃会话（未关机） */
  activeSession: () => BootSession | null;
}

const EMPTY_DATA: BootData = { bootCount: 0, shutdownCount: 0, sessions: [] };

export const useSessionStore = create<SessionState>((set, get) => ({
  data: EMPTY_DATA,
  loading: false,
  error: null,
  setData: (data) => set({ data, error: null }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
  activeSession: () => {
    const sessions = get().data.sessions;
    return sessions.find((s) => !s.shutdownTime) ?? null;
  },
}));

/** 工具：ISO → 本地日期 yyyy-mm-dd */
export function isoToLocalDate(iso: string | null): string {
  if (!iso) return '';
  const d = new Date(iso);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

export function getLocalToday(): string {
  return isoToLocalDate(new Date().toISOString());
}

/** 今日开机次数 */
export function countTodayBoots(sessions: BootSession[]): number {
  const today = getLocalToday();
  return sessions.filter((s) => isoToLocalDate(s.bootTime) === today).length;
}

/** 完整本地时间格式化 */
export function fmtFullTime(iso: string | null): string {
  if (!iso) return '—';
  return new Date(iso).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/** 时长格式化（毫秒 → x时x分x秒） */
export function fmtDuration(ms: number | null | undefined): string {
  if (!ms || ms <= 0) return '—';
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}时${m}分${sec}秒`;
  if (m > 0) return `${m}分${sec}秒`;
  return `${sec}秒`;
}

/** 实时计算运行时长（基于当前时间） */
export function liveDuration(bootTime: string | null): string {
  if (!bootTime) return '—';
  return fmtDuration(Date.now() - new Date(bootTime).getTime());
}
