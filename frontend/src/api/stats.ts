import { http } from './client';
import type {
  DailyStatsMap,
  WeeklyStatsMap,
  TrendStatsMap,
  OverviewStats,
  AnomalyStats,
} from './types';

export const statsApi = {
  daily: () =>
    http.get<{ data: DailyStatsMap }>('/stats/daily').then((r) => r.data),

  weekly: () =>
    http.get<{ data: WeeklyStatsMap }>('/stats/weekly').then((r) => r.data),

  /**
   * 最近 N 天趋势
   * @param days 最近多少天，默认 30
   */
  trend: (days = 30) =>
    http
      .get<{ data: TrendStatsMap; days: number }>(`/stats/trend?days=${days}`)
      .then((r) => r),

  overview: () => http.get<OverviewStats>('/stats/overview'),

  anomalies: () => http.get<AnomalyStats>('/stats/anomalies'),
};
