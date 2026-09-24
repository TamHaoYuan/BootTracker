/**
 * 统一原生桥入口
 *
 * 检测优先级：Tauri > Mock (浏览器)
 *
 * 调用方始终用 `native(method, args)`，无需关心环境差异。
 */
import { isTauri, callNative as callNativeTauri } from './tauri';
import { callNativeMock } from './mock';
import { BridgeError, BusinessError } from './errors';

export type { BridgeResult } from './tauri';

const _tauri = isTauri();

/** 同步判断是否在原生环境（Tauri） */
export function isNativeAvailable(): boolean {
  return _tauri;
}

/** 同步判断是否在 Tauri 环境 */
export function isTauriEnvironment(): boolean {
  return _tauri;
}

/** 启动时调用一次，初始化桥（Tauri 无需初始化，保留接口兼容） */
export async function setupBridge(): Promise<void> {
  // Tauri 环境无需初始化（invoke 是即时的）
}

/**
 * 统一原生调用入口
 * - Tauri 环境 → Tauri IPC (invoke)
 * - 其他 → mock（开发/浏览器模式）
 */
export async function native<T>(
  method: string,
  args: Record<string, unknown> = {},
  timeoutMs = 5000,
): Promise<T> {
  if (_tauri) {
    return callNativeTauri<T>(method, args, timeoutMs);
  }
  return callNativeMock<T>(method, args);
}

export { BridgeError, BusinessError };
