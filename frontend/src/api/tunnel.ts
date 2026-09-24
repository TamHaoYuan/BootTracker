import { http } from './client';
import type { TunnelUrlResponse } from './types';

export const tunnelApi = {
  getUrl: () => http.get<TunnelUrlResponse>('/tunnel-url'),
};
