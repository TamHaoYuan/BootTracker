import { Component } from 'react';
import type { ErrorInfo, ReactNode } from 'react';
import { Button, Result } from 'antd';

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

/**
 * 错误边界：捕获子树渲染错误与 React.lazy 动态 chunk 加载失败。
 *
 * Suspense 只处理“加载中”，不处理“加载失败”——lazy 的 import() reject 会向上冒泡，
 * 若无 ErrorBoundary，React 会卸载整棵树导致不可恢复白屏（典型场景：原地更新后旧 hash
 * 分包被替换，用户切到尚未加载的懒路由）。此处兜底并提供“重新加载”恢复入口。
 */
class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error('[ErrorBoundary] 渲染 / 懒加载失败:', error, info.componentStack);
  }

  private handleReload = (): void => {
    window.location.reload();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (error) {
      return (
        <Result
          status="warning"
          title="页面加载失败"
          subTitle={error.message || '渲染该页面时发生错误'}
          extra={
            <Button type="primary" onClick={this.handleReload}>
              重新加载
            </Button>
          }
        />
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;
