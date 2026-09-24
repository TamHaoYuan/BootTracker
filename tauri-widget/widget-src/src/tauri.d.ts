// Tauri v2 全局 API 类型声明（withGlobalTauri: true 注入到 window.__TAURI__）
interface WidgetDataPayload {
  today_count: number;
  boot_time: string | null;
  online: boolean;
}

interface TauriEvent<T> {
  payload: T;
}

interface TauriCore {
  invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
}

interface TauriWindow {
  getCurrentWindow: () => {
    startDragging: () => Promise<void>;
  };
}

interface TauriEventApi {
  listen: <T = WidgetDataPayload>(
    event: string,
    handler: (e: TauriEvent<T>) => void,
  ) => Promise<() => void>;
}

interface Window {
  __TAURI__?: {
    core?: TauriCore;
    event?: TauriEventApi;
    window?: TauriWindow;
  };
}
