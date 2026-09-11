// ============== 全局渲染错误边界 ==============
// 捕获渲染期异常，自动上报，并提供降级页（重新加载 + 可选反馈）。

import { Component, type ErrorInfo, type ReactNode } from "react";
import { normalizeError } from "./normalize";
import { reportNormalizedError } from "./reporter";
import { toastInfo } from "./ToastContext";

interface Props {
  children: ReactNode;
  /**
   * 自定义降级 UI。提供时替换默认整页降级页，用于「局部隔离」场景
   * （如插件插槽组件崩溃时不应让整个宿主页面消失）。
   */
  fallback?: ReactNode | ((error: Error) => ReactNode);
  /** 错误来源标签（如 plugin:lollipop-rank），会随上报一起记录，便于定位。 */
  scope?: string;
}

interface State {
  error: Error | null;
  reported: boolean;
  note: string;
}

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null, reported: false, note: "" };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    if (!this.state.reported) {
      const normalized = normalizeError(error);
      reportNormalizedError(normalized, {
        stack: `${error.stack || ""}\n\n组件栈:\n${info.componentStack}`,
        ...(this.props.scope
          ? { message: `[${this.props.scope}] ${normalized.message}` }
          : {}),
      });
      this.setState({ reported: true });
    }
  }

  handleReload = () => {
    try {
      window.location.hash = "#/";
    } catch {
      /* ignore */
    }
    window.location.reload();
  };

  handleSubmitNote = () => {
    const note = this.state.note.trim();
    if (!this.state.error || !note) return;
    reportNormalizedError(normalizeError(this.state.error), {
      message: `${this.state.error.message}（用户反馈：${note.slice(0, 500)}）`,
    });
    this.setState({ note: "", reported: true });
    toastInfo("反馈已提交，感谢你的帮助");
  };

  render() {
    if (!this.state.error) return this.props.children;

    // 局部降级：给出 `scope`（插件等可隔离单元）或显式 `fallback` 时，
    // 只替换出错区域，不影响宿主页面。
    if (this.props.fallback !== undefined || this.props.scope !== undefined) {
      const { fallback, scope } = this.props;
      return (
        <>
          {typeof fallback === "function"
            ? fallback(this.state.error)
            : (fallback ?? (
                <div className="border border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950/30 px-3 py-2 text-xs text-red-600 dark:text-red-400">
                  {scope ? `「${scope}」` : "该插件"}渲染出错，已自动上报
                </div>
              ))}
        </>
      );
    }

    return (
      <div className="min-h-screen bg-gray-100 dark:bg-gray-950 flex items-center justify-center p-6">
        <div className="bg-white dark:bg-gray-900 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 max-w-lg w-full p-8 text-center">
          <div className="text-4xl mb-4">⚠️</div>
          <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100 mb-2">
            页面出错了
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
            发生了一个意外错误，错误信息已自动上报。你可以重新加载页面，或附上说明帮助我们修复。
          </p>
          <p className="text-xs text-gray-400 dark:text-gray-500 mb-6 break-words">
            {this.state.error.message}
          </p>
          <div className="flex flex-col gap-3">
            <button
              type="button"
              onClick={this.handleReload}
              className="bg-gray-800 dark:bg-gray-100 text-white dark:text-gray-900 px-5 py-2 text-sm font-medium hover:opacity-90 transition-opacity"
            >
              重新加载
            </button>
            <div className="flex gap-2">
              <input
                value={this.state.note}
                onChange={(e) => this.setState({ note: e.target.value })}
                placeholder="可选：描述你刚刚做了什么…"
                className="flex-1 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-700 dark:text-gray-200 focus:outline-none"
              />
              <button
                type="button"
                onClick={this.handleSubmitNote}
                disabled={!this.state.note.trim()}
                className="border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm text-gray-600 dark:text-gray-300 disabled:opacity-40 hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
              >
                提交反馈
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }
}
