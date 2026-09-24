import { http } from './client';
import type { RaiseWindowResponse, WindowThemeResponse, OkResponse, UploadBgResponse } from './types';

export const windowApi = {
  /** 唤起正在运行的实例主窗口（用于第二个启动进程请求） */
  raise: () => http.post<RaiseWindowResponse>('/raise-window'),

  /** 同步主窗口外壳主题（标题栏 + 窗口背景色） */
  setTheme: (mode: 'dark' | 'light') =>
    http.put<WindowThemeResponse>('/window-theme', { mode }),
};

/* 通用（在 http_handler.py 中直接处理，未归入子模块） */

export const systemApi = {
  /** 重启后端 HTTP 服务（实际会重启整个 Python 进程） */
  restartServer: () =>
    http.post<OkResponse & { message?: string }>('/restart-server'),

  /** 上传自定义背景图（multipart/form-data） */
  uploadBgImage: (file: File) => {
    const fd = new FormData();
    // 字段名取什么？旧前端 app.js 用 multipart 直接提交文件，http_handler._upload_bg_image 里找 b"filename" 作为 key 解析。
    // 具体是解析 Content-Disposition 带 filename= 的分段，name 其实不敏感；所以我们随便取一个字段名即可（file）。
    fd.append('file', file);
    return http.upload<UploadBgResponse>('/upload-bg', fd);
  },
};
