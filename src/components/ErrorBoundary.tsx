import React, { Component, ReactNode } from 'react';
import { Button, Result } from 'antd';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
  };

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <Result
          status="error"
          title="出错了"
          subTitle={this.state.error?.message || '应用发生了未知错误'}
          extra={[
            <Button key="retry" type="primary" onClick={this.handleReset}>
              重试
            </Button>,
            <Button key="reload" onClick={this.handleReload}>
              刷新页面
            </Button>,
          ]}
        />
      );
    }

    return this.props.children;
  }
}
