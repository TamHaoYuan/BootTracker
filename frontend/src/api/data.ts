import { http } from './client';
import type {
  BootData,
  BootSession,
  OkResponse,
  PingResponse,
} from './types';

export const dataApi = {
  ping: () => http.get<PingResponse>('/ping', { timeoutMs: 3000 }),

  getData: () => http.get<BootData>('/data'),

  saveData: (data: BootData) => http.post<OkResponse>('/data', data),

  clearData: () => http.post<OkResponse>('/clear'),

  addSession: (bootTime: string, shutdownTime?: string) =>
    http.post<OkResponse & { data: BootData }>('/add-session', { bootTime, shutdownTime }),

  updateSession: (sessionId: string, patch: Partial<Pick<BootSession, 'bootTime' | 'shutdownTime'>>) =>
    http.put<OkResponse>(`/sessions/${sessionId}`, { session_id: sessionId, patch }),

  deleteSession: (sessionId: string) =>
    http.delete<OkResponse>(`/sessions/${sessionId}`, { session_id: sessionId }),

  mergeSessions: (ids: string[]) =>
    http.post<OkResponse & { data: BootData }>('/merge-sessions', { ids }),

  stopServer: () => http.post<OkResponse>('/stop'),
};
