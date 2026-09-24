import { describe, it, expect, vi, afterEach } from 'vitest';
import { http } from '../src/api/client';

// 模拟 fetch
const fetchMock = vi.fn();
vi.stubGlobal('fetch', fetchMock);

afterEach(() => {
  fetchMock.mockReset();
});

describe('http client', () => {
  it('parses JSON success response', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      statusText: 'OK',
      text: () => Promise.resolve(JSON.stringify({ ok: true })),
    });
    const r = await http.get<{ ok: boolean }>('/ping');
    expect(r.ok).toBe(true);
  });

  it('throws HttpError on non-2xx', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
      text: () => Promise.resolve(JSON.stringify({ error: 'boom' })),
    });
    await expect(http.get('/data')).rejects.toMatchObject({
      name: 'HttpError',
      status: 500,
      message: 'boom',
    });
  });

  it('throws HttpError when body contains error field', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      statusText: 'OK',
      text: () => Promise.resolve(JSON.stringify({ error: 'missing session_id' })),
    });
    await expect(http.put('/sessions/x', { session_id: 'x' })).rejects.toMatchObject({
      name: 'HttpError',
      message: 'missing session_id',
    });
  });

  it('sends JSON body for POST', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      statusText: 'OK',
      text: () => Promise.resolve(JSON.stringify({ ok: true })),
    });
    await http.post('/data', { bootCount: 1 });
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/data',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({ bootCount: 1 }),
      }),
    );
  });

  it('aborts on timeout', async () => {
    // fetch 永不 resolve，靠 AbortController 触发 reject
    fetchMock.mockImplementationOnce((_url: string, init: RequestInit) => {
      return new Promise((_resolve, reject) => {
        init.signal?.addEventListener('abort', () => {
          reject(new DOMException('aborted', 'AbortError'));
        });
      });
    });
    await expect(http.get('/slow', { timeoutMs: 50 })).rejects.toThrow();
  });
});
