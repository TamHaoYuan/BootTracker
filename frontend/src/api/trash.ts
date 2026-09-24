import { http } from './client';
import type { TrashData, BootData, OkResponse } from './types';

export const trashApi = {
  get: () => http.get<TrashData>('/trash'),

  save: (trash: TrashData) => http.post<OkResponse>('/trash', trash),

  restore: (id: string) =>
    http.post<OkResponse>('/trash/restore', { id }),

  clear: () => http.post<OkResponse>('/trash/clear'),

  permanentDelete: (id: string) =>
    http.post<OkResponse>('/trash/delete', { id }),
};

// 给现有 deleteSession 做兜底：旧行为"删除"其实是丢回收站，需要我们在业务层做。
// 后端原生 delete-sessions/:id 是真删除。这里加一个"软删"函数（把 session 移入回收站）：
export async function softDeleteSession(
  sessionId: string,
  opts: {
    fetchData: () => Promise<BootData>;
    saveData: (d: BootData) => Promise<unknown>;
    fetchTrash: () => Promise<TrashData>;
    saveTrash: (t: TrashData) => Promise<unknown>;
  },
): Promise<void> {
  const data = await opts.fetchData();
  const target = data.sessions.find((s) => s.id === sessionId);
  if (!target) return;
  data.sessions = data.sessions.filter((s) => s.id !== sessionId);
  data.bootCount = data.sessions.length;
  data.shutdownCount = data.sessions.filter((s) => s.shutdownTime).length;
  await opts.saveData(data);

  const trash = await opts.fetchTrash();
  trash.sessions.unshift(target);
  await opts.saveTrash(trash);
}
