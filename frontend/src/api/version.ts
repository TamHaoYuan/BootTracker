import { http } from './client';
import type {
  VersionInfo,
  VersionHistoryEntry,
  BumpVersionResponse,
  CheckUpdateResponse,
} from './types';

export const versionApi = {
  get: () => http.get<VersionInfo>('/version'),

  history: () =>
    http
      .get<{ history: VersionHistoryEntry[] }>('/version/history')
      .then((r) => r.history),

  bump: (type: 'major' | 'minor' | 'patch', notes = '') =>
    http.post<BumpVersionResponse>('/version/bump', { type, notes }),

  checkUpdate: () => http.post<CheckUpdateResponse>('/check-update'),
};
