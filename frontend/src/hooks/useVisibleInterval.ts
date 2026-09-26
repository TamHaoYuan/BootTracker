import { useEffect, useRef } from 'react';

/* ------------------------------------------------------------------ *
 * 应用可见性单例
 *
 * 合并两个信号，任一为“不可见”即视为整体不可见：
 *   1. document.visibilityState —— 浏览器标准；WebView2 最小化 / 被遮挡时通常触发
 *   2. Tauri 原生 window-visibility 事件 —— 窗口 hide()/show() 时由 Rust 侧 emit
 *
 * 为何需要原生信号兜底：Tauri window.hide() 隐藏到托盘时，WebView2 并不保证触发
 * visibilitychange（取决于 Chromium 原生窗口遮挡检测是否生效）。若仅靠 document 信号，
 * 后台定时器可能不暂停，导致优化失效。两信号取交集，任一生效即可正确暂停 / 恢复。
 * ------------------------------------------------------------------ */
type VisibilityListener = (visible: boolean) => void;

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

let docVisible =
  typeof document === 'undefined' ? true : document.visibilityState === 'visible';
let nativeVisible = true; // 非 Tauri 环境恒为 true；Tauri 下由 window-visibility 更新
let lastVisible = docVisible && nativeVisible;
let initialized = false;
const listeners = new Set<VisibilityListener>();

function computeVisible(): boolean {
  return docVisible && nativeVisible;
}

function notify(): void {
  const v = computeVisible();
  if (v === lastVisible) return; // 仅在整体可见性真正翻转时通知，避免重复触发
  lastVisible = v;
  listeners.forEach((l) => l(v));
}

function ensureInitialized(): void {
  if (initialized) return;
  initialized = true;

  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', () => {
      docVisible = document.visibilityState === 'visible';
      notify();
    });
  }

  if (inTauri()) {
    // 动态导入：非 Tauri 环境（含单测 / 纯浏览器）不会触及 @tauri-apps/api
    void import('@tauri-apps/api/event')
      .then(({ listen }) =>
        listen<boolean>('window-visibility', (e) => {
          nativeVisible = e.payload !== false;
          notify();
        }),
      )
      .catch(() => {
        /* 忽略：退回仅用 document 信号 */
      });
  }
}

function subscribeVisibility(listener: VisibilityListener): () => void {
  ensureInitialized();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * 仅在应用可见时运行的定时器。
 *
 * 窗口最小化 / 隐藏到托盘时暂停回调，消除后台空转（轮询请求、每秒重渲染）；
 * 恢复可见时立即补触发一次再继续，避免数据 / 时钟陈旧。回调经 ref 持有最新引用，
 * ms 不变则不重建定时器。
 */
export function useVisibleInterval(callback: () => void, ms: number): void {
  const cbRef = useRef(callback);
  cbRef.current = callback;

  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | null = null;

    const start = () => {
      if (timer === null) timer = setInterval(() => cbRef.current(), ms);
    };
    const stop = () => {
      if (timer !== null) {
        clearInterval(timer);
        timer = null;
      }
    };

    const unsubscribe = subscribeVisibility((visible) => {
      if (visible) {
        cbRef.current(); // 恢复可见时立即补一次
        start();
      } else {
        stop();
      }
    });

    // 初始：仅在可见时启动；不可见则等可见事件再拉起
    if (computeVisible()) start();

    return () => {
      unsubscribe();
      stop();
    };
  }, [ms]);
}
