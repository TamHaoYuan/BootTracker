import { create } from 'zustand';
import { persist } from 'zustand/middleware';

/** 界面缩放区间：与桌面端 settings.uiScale 保持一致 */
export const UI_SCALE_MIN = 0.8;
export const UI_SCALE_MAX = 1.5;
export const UI_SCALE_STEP = 0.05;
export const UI_SCALE_DEFAULT = 1.0;

export function clampUiScale(v: number): number {
  if (!Number.isFinite(v)) return UI_SCALE_DEFAULT;
  const clamped = Math.min(UI_SCALE_MAX, Math.max(UI_SCALE_MIN, v));
  // 吸附到 5% 网格，避免滚轮缩放写出一长串浮点尾数
  return Math.round(clamped * 100) / 100;
}

/**
 * 把缩放比例写到 DOM。
 *
 * 为什么用 CSS `zoom` 而不是 `transform: scale` 或 `font-size`：
 * - `transform` 不改变布局尺寸，放大后底部被裁掉、滚动高度不对；
 * - 逐元素改 `font-size` 只能影响用 rem/em 的声明，项目里大量 px 字号的
 *   第三方组件（AntD）不会跟着变。
 * 取浏览器原生「页面缩放」的实现方式：对根容器整体 `zoom`。缩放系数会
 * 沿 DOM 树继承，**必须只写在容器节点上**——早期版本写了 `html body *`，
 * 结果是逐层相乘（1.5× 实际放大到 2.25×），已修正。
 *
 * 另外三类节点渲染在 `#root` 之外（挂在 body 下），不继承根容器的 zoom，
 * 需要单独列出；它们都在 `#root` 之外，所以不会与根容器叠加。
 */
const ZOOM_TARGETS =
  '#root, .ant-select-dropdown, .ant-picker-dropdown, .ant-dropdown, ' +
  '.ant-popover, .ant-tooltip, .ant-notification, .ant-message, .ant-modal-root, ' +
  '.ant-drawer, .ant-image-preview-root, .ant-wave';

export function applyUiScale(scale: number): void {
  if (typeof document === 'undefined') return;
  const s = clampUiScale(scale);
  const root = document.documentElement;
  root.style.setProperty('--ui-scale', String(s));
  // 便于排查：在元素面板里能直接看到当前比例
  root.dataset.uiScale = String(s);
  let el = document.getElementById('ui-scale-style') as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement('style');
    el.id = 'ui-scale-style';
    document.head.appendChild(el);
  }
  el.textContent = `${ZOOM_TARGETS} { zoom: ${s.toFixed(4)}; }`;
}

interface UiScaleState {
  uiScale: number;
  setUiScale: (v: number) => void;
  nudgeUiScale: (delta: number) => void;
  resetUiScale: () => void;
}

/**
 * 页面缩放（纯前端偏好，localStorage 持久化）。
 *
 * 不写后端 `settings.json`：缩放是「这台机器这个屏幕」的偏好，
 * 后端值会被其它设备覆盖；桌面端（egui）用自己的 settings.uiScale。
 */
export const useUiScaleStore = create<UiScaleState>()(
  persist(
    (set, get) => ({
      uiScale: UI_SCALE_DEFAULT,
      setUiScale: (v) => {
        const s = clampUiScale(v);
        set({ uiScale: s });
        applyUiScale(s);
      },
      nudgeUiScale: (delta) => get().setUiScale(get().uiScale + delta),
      resetUiScale: () => get().setUiScale(UI_SCALE_DEFAULT),
    }),
    {
      name: 'boottracker-ui-scale',
      partialize: (s) => ({ uiScale: s.uiScale }),
      onRehydrateStorage: () => (state) => {
        applyUiScale(state?.uiScale ?? UI_SCALE_DEFAULT);
      },
    },
  ),
);
