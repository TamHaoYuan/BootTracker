import { http } from './client';
import type { BackupListResponse, CleanBackupsResponse, OkResponse } from './types';

export const backupApi = {
  list: () => http.get<BackupListResponse>('/backups'),

  restore: (filename: string) =>
    http.post<OkResponse>('/backup/restore', { filename }),

  delete: (filename: string) =>
    http.delete<OkResponse>('/delete-backup', { filename }),

  clean: (keep = 10) =>
    http.post<CleanBackupsResponse>('/clean-backups', { keep }),
};
