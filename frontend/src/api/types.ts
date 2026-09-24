/** 后端 API 数据类型（与 Python data_store.py / settings.py 对齐） */

export interface BootSession {
  id: string;
  bootTime: string; // ISO 字符串
  shutdownTime: string | null;
  duration: number | null; // 毫秒
}

export interface BootData {
  bootCount: number;
  shutdownCount: number;
  sessions: BootSession[];
}

/** 设置 schema（与 server/settings.py _DEFAULT_SETTINGS 对齐） */
export interface AppSettings {
  autoStart: boolean;
  autoBackup: boolean;
  backupCount: number;
  autoCloseIdle: boolean;
  idleCloseMinutes: number;
  defaultChartType: 'bar' | 'line';
  timeFormat: '24h' | '12h';
  customBgImage: string;
  lanAccess: boolean;
  tunnelEnabled: boolean;
  tunnelToken: string;
  customDomain: string;
  widgetEnabled: boolean;
  widgetPosition: string;
  appMode: 'dark' | 'light';
  // 后端额外返回的字段
  _autoStartRegistered?: boolean;
}

export interface PingResponse {
  ok: boolean;
}

export interface OkResponse {
  ok: true;
  [k: string]: unknown;
}

export interface UpdatedResponse extends OkResponse {
  updated: string[];
}

export interface VersionInfo {
  version: string;
  updateUrl: string;
}

export interface VersionHistoryEntry {
  version: string;
  from?: string;
  notes: string;
  date: string;
}

export interface BackupFile {
  name: string;
  size?: number;
  date?: string;
}

export interface TunnelInfo {
  enabled: boolean;
  url: string | null;
  customDomain: string | null;
}

/* ================ 统计 ================ */
export interface DailyStat {
  bootCount: number;
  shutdownCount: number;
  totalDuration: number;
  sessionCount: number;
}

/** date(YYYY-MM-DD) -> daily stat */
export type DailyStatsMap = Record<string, DailyStat>;

export interface WeeklyStat {
  bootCount: number;
  shutdownCount: number;
  totalDuration: number;
}

export type WeeklyStatsMap = Record<string, WeeklyStat>;

export interface TrendStat {
  bootCount: number;
  shutdownCount: number;
  totalDuration: number;
  avgDuration: number;
}
export type TrendStatsMap = Record<string, TrendStat>;

export interface OverviewStats {
  totalBoot: number;
  totalShutdown: number;
  totalDuration: number;
  avgDuration: number;
  activeCount: number;
  recentSessions: BootSession[];
  firstDate: string | null;
  lastDate: string | null;
}

export interface AnomalyStats {
  longest: BootSession[];
  shortest: BootSession[];
  active: BootSession[];
  avgDuration: number;
  thresholdHigh: number;
  thresholdLow: number;
}

/* ================ 回收站 ================ */
export interface TrashData {
  sessions: BootSession[];
}

/* ================ 备份 ================ */
export interface BackupListResponse {
  backups: string[]; // 文件名数组（boot-data-*.db）
}
export interface CleanBackupsResponse extends OkResponse {
  deleted: number;
  remaining: number;
}

/* ================ 版本 bump ================ */
export interface BumpVersionResponse extends OkResponse {
  version: string;
  previous: string;
}
export interface CheckUpdateResponse {
  latest?: string;
  hasUpdate?: boolean;
  downloadUrl?: string;
  message?: string;
}

/* ================ 隧道 ================ */
export interface TunnelUrlResponse {
  url: string | null;
}

/* ================ 窗口/外壳 ================ */
export interface RaiseWindowResponse {
  ok: boolean;
}
export interface WindowThemeResponse extends OkResponse {
  mode: 'dark' | 'light';
  applied: boolean;
}

/* ================ 上传 ================ */
export interface UploadBgResponse extends OkResponse {
  path: string; // 图片访问相对路径，例如 /static/uploads/bg.jpg
}
