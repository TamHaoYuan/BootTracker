/** 通信层错误：原生桥调用本身失败（超时、未就绪、解析失败等） */
export class BridgeError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = 'BridgeError';
  }
}

/** 业务错误：通信成功，但 Python 端返回 ok=false（未知方法、参数错误、内部异常） */
export class BusinessError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = 'BusinessError';
  }
}

/** HTTP 通信错误 */
export class HttpError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = 'HttpError';
  }
}
