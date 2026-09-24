/**
 * HTTP API 客户端
 *
 * 设计要点：
 * - 统一前缀 /api（dev 时由 vite proxy 转发到 Python，生产由 Python 同源服务）
 * - 超时（AbortController，默认 8s）
 * - 错误分类：HttpError（status 非 2xx 或 body.error）
 * - JSON 安全解析
 */
import { HttpError } from '../bridge/errors';

const DEFAULT_TIMEOUT = 8000;

export interface RequestOptions extends Omit<RequestInit, 'body'> {
  body?: unknown;
  timeoutMs?: number;
}

async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const { body, timeoutMs = DEFAULT_TIMEOUT, headers, ...rest } = options;

  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);

  try {
    const init: RequestInit = {
      ...rest,
      signal: ctrl.signal,
      headers: {
        'Content-Type': 'application/json',
        ...headers,
      },
    };
    if (body !== undefined) {
      init.body = JSON.stringify(body);
    }

    const res = await fetch(`/api${path}`, init);
    const text = await res.text();
    let data: unknown = null;
    if (text) {
      try {
        data = JSON.parse(text);
      } catch {
        throw new HttpError(res.status, `invalid JSON response: ${text.slice(0, 200)}`);
      }
    }

    if (!res.ok || (typeof data === 'object' && data !== null && 'error' in data)) {
      const msg =
        (typeof data === 'object' && data !== null && 'error' in data
          ? String((data as { error: unknown }).error)
          : res.statusText) || `HTTP ${res.status}`;
      throw new HttpError(res.status, msg);
    }

    return data as T;
  } finally {
    clearTimeout(timer);
  }
}

export const http = {
  get: <T>(path: string, opts?: RequestOptions) => request<T>(path, { ...opts, method: 'GET' }),
  post: <T>(path: string, body?: unknown, opts?: RequestOptions) =>
    request<T>(path, { ...opts, method: 'POST', body }),
  put: <T>(path: string, body?: unknown, opts?: RequestOptions) =>
    request<T>(path, { ...opts, method: 'PUT', body }),
  delete: <T>(path: string, body?: unknown, opts?: RequestOptions) =>
    request<T>(path, { ...opts, method: 'DELETE', body }),
  /**
   * 文件上传（multipart/form-data），content-type 由浏览器自动设置（含 boundary）
   * @param path 相对 /api 的路径
   * @param formData 预填好的 FormData
   * @param timeoutMs 超时，默认 60s（文件上传用更久）
   */
  upload: <T>(path: string, formData: FormData, timeoutMs = 60_000): Promise<T> => {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), timeoutMs);
    return fetch(`/api${path}`, {
      method: 'POST',
      signal: ctrl.signal,
      body: formData,
    })
      .then(async (res) => {
        const text = await res.text();
        let data: unknown = null;
        if (text) {
          try {
            data = JSON.parse(text);
          } catch {
            throw new HttpError(res.status, `invalid JSON response: ${text.slice(0, 200)}`);
          }
        }
        if (!res.ok || (typeof data === 'object' && data !== null && 'error' in data)) {
          const msg =
            (typeof data === 'object' && data !== null && 'error' in data
              ? String((data as { error: unknown }).error)
              : res.statusText) || `HTTP ${res.status}`;
          throw new HttpError(res.status, msg);
        }
        return data as T;
      })
      .finally(() => clearTimeout(timer));
  },
};
