import { http } from './client';
import type { AppSettings, UpdatedResponse } from './types';

export const settingsApi = {
  get: () => http.get<AppSettings>('/settings'),

  update: (updates: Partial<AppSettings>) =>
    http.put<UpdatedResponse>('/settings', updates),
};
