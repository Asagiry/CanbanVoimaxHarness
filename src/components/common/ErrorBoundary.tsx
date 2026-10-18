import { Component, ReactNode } from 'react';
import { AlertTriangle, X } from 'lucide-react';

interface ErrorBoundaryProps {
  children: ReactNode;
  /** Вызывается по кнопке «Закрыть» (например, закрыть дровер). */
  onClose?: () => void;
  /** Подпись блока, где произошло падение (для текста ошибки). */
  scopeLabel?: string;
}

interface ErrorBoundaryState {
  hasError: boolean;
  message: string;
}

/**
 * Классовый ErrorBoundary со светлым фолбэком.
 * Не даёт падению одного блока (чат / Git Diff) белить всё дерево React.
 */
export default class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, message: '' };

  static getDerivedStateFromError(err: unknown): ErrorBoundaryState {
    const message = err instanceof Error ? err.message : 'Неизвестная ошибка рендера';
    return { hasError: true, message };
  }

  componentDidCatch(err: unknown): void {
    console.error(`[ErrorBoundary${this.props.scopeLabel ? `:${this.props.scopeLabel}` : ''}]`, err);
  }

  private handleReset = () => {
    this.setState({ hasError: false, message: '' });
  };

  render() {
    if (!this.state.hasError) return this.props.children;

    const { onClose, scopeLabel } = this.props;
    return (
      <div className="flex-1 flex items-center justify-center p-6 bg-[#fafafa] dark:bg-[#090a0f]">
        <div className="max-w-md w-full rounded-lg border border-[#ebebeb] dark:border-[#21262d] bg-white dark:bg-[#0d1117] p-4 shadow-xs">
          <div className="flex items-center space-x-2 pb-2 mb-2 border-b border-[#ebebeb] dark:border-[#21262d]">
            <AlertTriangle className="h-4 w-4 text-[#b91c1c] dark:text-[#ff7b72] shrink-0" />
            <span className="text-xs font-semibold text-[#171717] dark:text-[#f0f6fc] font-sans">
              Что-то пошло не так{scopeLabel ? ` (${scopeLabel})` : ''}
            </span>
          </div>
          <p className="text-xs text-[#666666] dark:text-[#8b949e] font-mono break-words select-text">
            {this.state.message || 'Ошибка отображения блока'}
          </p>
          <div className="mt-3 flex items-center space-x-2">
            <button
              type="button"
              onClick={() => {
                this.handleReset();
                onClose?.();
              }}
              className="flex items-center space-x-1.5 px-3 py-1.5 rounded-md bg-white dark:bg-[#161b22] hover:bg-[#f9f9f9] dark:hover:bg-[#21262d] text-[#171717] dark:text-[#f0f6fc] border border-[#ebebeb] dark:border-[#30363d] transition font-sans text-xs font-medium cursor-pointer shadow-xs active:scale-[0.98]"
            >
              <X className="h-3.5 w-3.5" />
              <span>Закрыть</span>
            </button>
            <button
              type="button"
              onClick={this.handleReset}
              className="px-3 py-1.5 rounded-md bg-[#f4f4f5] dark:bg-[#21262d] hover:bg-[#e4e4e7] dark:hover:bg-[#30363d] text-[#171717] dark:text-[#f0f6fc] border border-[#ebebeb] dark:border-[#30363d] transition font-sans text-xs font-medium cursor-pointer active:scale-[0.98]"
            >
              Попробовать снова
            </button>
          </div>
        </div>
      </div>
    );
  }
}
