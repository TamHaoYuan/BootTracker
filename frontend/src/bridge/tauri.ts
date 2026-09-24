/**
 * Tauri IPC 桥接封装
 *
 * 使用 @tauri-apps/api 的 invoke 调用 Rust 后端实现的 IPC 命令。
 *
 * 返回结构：
 * { ok: true, data: T } | { ok: false, code, message }
 */
import { invoke } from '@tauri-apps/api/core';
import { BridgeError, BusinessError } from './errors';

export type BridgeResult<T> =
  | { ok: true; data: T }
  | { ok: false; code: string; message: string };

/** 检测当前是否运行在 Tauri 环境 */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * Tauri IPC 调用
 *
 * @param method Rust 命令名称（snake_case）
 * @param args 参数对象
 */
export async function callTauri<T>(
  method: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  try {
    const result = await invoke<BridgeResult<T>>(method, args);

    if (!result.ok) {
      throw new BusinessError(
        (result as { ok: false; code: string; message: string }).code,
        (result as { ok: false; code: string; message: string }).message,
      );
    }

    return result.data;
  } catch (e) {
    if (e instanceof BusinessError) throw e;
    throw new BridgeError('TAURI_INVOKE_FAILED', String(e));
  }
}

/**
 * 统一方法名映射：将 camelCase 方法名映射到 Tauri 的 snake_case 命令
 */
const METHOD_MAP: Record<string, string> = {
  setTheme: 'set_theme',
  raiseWindow: 'raise_window',
  quitApp: 'quit_app',
  pickFile: 'pick_file',
  getAppInfo: 'get_app_info',
};

/**
 * 通过统一入口调用
 *
 * @param method 方法名（camelCase，内部自动映射为 snake_case）
 * @param args 参数对象
 */
export async function callNative<T>(
  method: string,
  args: Record<string, unknown> = {},
  _timeoutMs?: number,
): Promise<T> {
  const tauriCommand = METHOD_MAP[method] || method;
  return callTauri<T>(tauriCommand, args);
}
