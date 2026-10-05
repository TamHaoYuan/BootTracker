/**
 * 统一原生桥入口（网页端专用）
 *
 * 桌面端已改为纯 Rust 前端（tauri-app/ui），网页端（浏览器）不再运行于 Tauri 环境，
 * 因此所有原生能力走浏览器 mock：调用方仍用 `native(method, args)`，无需关心环境。
 */
import { callNativeMock } from './mock';
import { BridgeError, BusinessError } from './errors';

/** 同步判断是否在原生环境（网页端恒为 false：桌面端已独立为 Rust 前端） */
export function isNativeAvailable(): boolean {
  return false;
}

/** 同步判断是否在 Tauri 环境（网页端恒为 false） */
export function isTauriEnvironment(): boolean {
  return false;
}

/** 兼容保留：网页端无需初始化 */
export async function setupBridge(): Promise<void> {
  // no-op
}

/**
 * 统一原生调用入口（网页端 → mock）
 */
export async function native<T>(
  method: string,
  args: Record<string, unknown> = {},
  timeoutMs = 5000,
): Promise<T> {
  return callNativeMock<T>(method, args, timeoutMs);
}

export { BridgeError, BusinessError };
