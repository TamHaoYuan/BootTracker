/** 格式化运行时长（ms → 中文短格式） */
export function fmtDuration(ms: number): string {
  if (!ms || ms <= 0) return '—';
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}时${m}分`;
  if (m > 0) return `${m}分${sec}秒`;
  return `${sec}秒`;
}
