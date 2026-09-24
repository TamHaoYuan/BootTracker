/**
 * 浏览器独立开发时的原生桥 mock
 * 在纯浏览器环境（vite dev server）替代 Tauri IPC，使前端无需启动后端也能开发
 */
import { BridgeError, BusinessError } from './errors';
import type { BridgeResult } from './tauri';

type MockHandler = (
  args: Record<string, unknown>,
) => BridgeResult<unknown> | Promise<BridgeResult<unknown>>;

const handlers: Record<string, MockHandler> = {
  setTheme: (args) => {
    const mode = args.mode as string;
    if (mode !== 'dark' && mode !== 'light') {
      return { ok: false, code: 'BRIDGE_BAD_ARGS', message: `invalid mode: ${mode}` };
    }
    return { ok: true, data: null };
  },
  raiseWindow: () => ({ ok: true, data: null }),
  quitApp: () => ({ ok: true, data: null }),
  pickFile: () => ({ ok: true, data: 'C:/mock/file.txt' }),
  getAppInfo: () => ({ ok: true, data: { version: '0.1.0-mock', mode: 'dev' } }),
};

export async function callNativeMock<T>(
  method: string,
  _args: Record<string, unknown> = {},
  timeoutMs = 200,
): Promise<T> {
  // 模拟通信延迟
  await new Promise((r) => setTimeout(r, timeoutMs));

  const handler = handlers[method];
  if (!handler) {
    throw new BusinessError('BRIDGE_UNKNOWN_METHOD', `mock: unknown method ${method}`);
  }
  const res = await handler(_args);
  if (!res.ok) {
    throw new BusinessError(res.code, res.message);
  }
  return res.data as T;
}

/** 注册自定义 mock 处理器（测试用） */
export function registerMockHandler(method: string, handler: MockHandler): void {
  handlers[method] = handler;
}

/** 重置为默认 handlers（测试隔离用） */
export function resetMockHandlers(): void {
  // 重新加载模块等效 — 直接清空非内置项由测试自行注册
  for (const key of Object.keys(handlers)) {
    if (!['setTheme', 'raiseWindow', 'quitApp', 'pickFile', 'getAppInfo'].includes(key)) {
      delete handlers[key];
    }
  }
}

export { BridgeError, BusinessError };
