import { describe, it, expect, beforeEach, vi } from 'vitest';
import { BridgeError, BusinessError } from '../src/bridge/errors';

// 动态导入，便于在 mock 与真实桥间切换
import { callNativeMock, registerMockHandler, resetMockHandlers } from '../src/bridge/mock';

describe('bridge errors classification', () => {
  it('BridgeError carries code', () => {
    const e = new BridgeError('BRIDGE_TIMEOUT', 'x');
    expect(e.name).toBe('BridgeError');
    expect(e.code).toBe('BRIDGE_TIMEOUT');
    expect(e.message).toBe('x');
  });

  it('BusinessError carries code', () => {
    const e = new BusinessError('BRIDGE_UNKNOWN_METHOD', 'y');
    expect(e.name).toBe('BusinessError');
    expect(e.code).toBe('BRIDGE_UNKNOWN_METHOD');
  });
});

describe('callNativeMock', () => {
  beforeEach(() => {
    resetMockHandlers();
  });

  it('returns data for known method', async () => {
    const result = await callNativeMock<{ version: string }>('getAppInfo');
    expect(result.version).toBe('0.1.0-mock');
  });

  it('throws BusinessError for unknown method', async () => {
    await expect(callNativeMock('nope')).rejects.toMatchObject({
      name: 'BusinessError',
      code: 'BRIDGE_UNKNOWN_METHOD',
    });
  });

  it('throws BusinessError when handler returns ok=false', async () => {
    registerMockHandler('bad', () => ({ ok: false, code: 'CUSTOM_FAIL', message: 'boom' }));
    await expect(callNativeMock('bad')).rejects.toMatchObject({
      name: 'BusinessError',
      code: 'CUSTOM_FAIL',
    });
  });

  it('respects custom handlers', async () => {
    registerMockHandler('custom', (args) => ({ ok: true, data: args.x }));
    const r = await callNativeMock<number>('custom', { x: 42 });
    expect(r).toBe(42);
  });
});

describe('callNativeMock pending handler', () => {
  it('mock framework accepts pending handler without resolving', async () => {
    // 注册一个永不 resolve 的 handler
    registerMockHandler('stuck', () => new Promise(() => {}));
    const p = callNativeMock('stuck', {}, 50);
    // mock 内部先 await 50ms 再调用 handler，handler 返回 pending Promise，
    // 整体不会 resolve；用 race 验证它确实 pending
    const settled = await Promise.race([
      p.then(() => true).catch(() => true),
      new Promise<boolean>((r) => setTimeout(() => r(false), 300)),
    ]);
    expect(settled).toBe(false);
    vi.clearAllTimers();
  });
});
