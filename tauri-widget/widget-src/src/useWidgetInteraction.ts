import { useEffect } from 'react';

/** 绑定窗口交互：左键拖动、右键菜单、双击打开主界面 */
export function useWidgetInteraction(): void {
  useEffect(() => {
    const tauri = window.__TAURI__;
    const win = tauri?.window;
    const core = tauri?.core;
    const invoke = core?.invoke;

    // 左键拖动窗口
    const onMouseDown = (e: MouseEvent) => {
      if (e.button !== 0 || !win?.getCurrentWindow) return;
      try {
        const w = win.getCurrentWindow();
        void w.startDragging().catch(() => {});
      } catch {
        // 拖动失败静默
      }
    };

    // 右键弹原生菜单
    const onContextMenu = (e: MouseEvent) => {
      e.preventDefault();
      void invoke?.('show_context_menu', {});
    };

    // 双击打开主界面
    const onDblClick = () => {
      void invoke?.('open_main_ui', {});
    };

    document.addEventListener('mousedown', onMouseDown);
    document.addEventListener('contextmenu', onContextMenu);
    document.addEventListener('dblclick', onDblClick);

    return () => {
      document.removeEventListener('mousedown', onMouseDown);
      document.removeEventListener('contextmenu', onContextMenu);
      document.removeEventListener('dblclick', onDblClick);
    };
  }, []);
}
