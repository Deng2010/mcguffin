// ============== 轻量 Toast ==============
// 不引入额外依赖；同时暴露模块级 toastError/toastSuccess/toastInfo，
// 供非 React 上下文（如 services/api.ts 的 401 处理）使用。

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

export type ToastType = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  type: ToastType;
  message: string;
  detail?: string;
}

interface ToastContextValue {
  success: (message: string) => void;
  error: (message: string, detail?: string) => void;
  info: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

let busEmit: ((t: Omit<ToastItem, "id">) => void) | null = null;
let nextId = 1;

function emit(type: ToastType, message: string, detail?: string) {
  busEmit?.({ type, message, detail });
}

/** 模块级 API：非 React 组件内也可以弹 toast。 */
export function toastError(message: string, detail?: string) {
  emit("error", message, detail);
}
export function toastSuccess(message: string) {
  emit("success", message);
}
export function toastInfo(message: string) {
  emit("info", message);
}

const TYPE_STYLES: Record<ToastType, string> = {
  success:
    "border bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300",
  error:
    "border bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300",
  info:
    "border bg-blue-50 border-blue-300 text-blue-700 dark:bg-blue-900/30 dark:border-blue-800 dark:text-blue-300",
};

const ICONS: Record<ToastType, string> = {
  success: "✓",
  error: "✕",
  info: "ℹ",
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const timersRef = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const remove = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
    const timer = timersRef.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timersRef.current.delete(id);
    }
  }, []);

  const push = useCallback(
    (t: Omit<ToastItem, "id">) => {
      const id = nextId++;
      setToasts((prev) => [...prev.slice(-4), { ...t, id }]);
      const timer = setTimeout(() => remove(id), t.type === "error" ? 8000 : 4000);
      timersRef.current.set(id, timer);
    },
    [remove],
  );

  useEffect(() => {
    busEmit = push;
    return () => {
      busEmit = null;
    };
  }, [push]);

  const value: ToastContextValue = {
    success: (message) => push({ type: "success", message }),
    error: (message, detail) => push({ type: "error", message, detail }),
    info: (message) => push({ type: "info", message }),
  };

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="fixed top-4 right-4 z-[100] flex flex-col gap-2 w-80 max-w-[calc(100vw-2rem)]">
        {toasts.map((toast) => (
          <ToastCard key={toast.id} toast={toast} onClose={() => remove(toast.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function ToastCard({ toast, onClose }: { toast: ToastItem; onClose: () => void }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div
      role="alert"
      className={`shadow-lg px-4 py-3 text-sm ${TYPE_STYLES[toast.type]}`}
    >
      <div className="flex items-start gap-2">
        <span className="font-bold flex-shrink-0">{ICONS[toast.type]}</span>
        <div className="flex-1 min-w-0">
          <p className="break-words">{toast.message}</p>
          {toast.detail && (
            <button
              type="button"
              onClick={() => setExpanded((e) => !e)}
              className="text-xs text-gray-400 dark:text-gray-500 underline mt-1"
            >
              {expanded ? "收起详情" : "查看详情"}
            </button>
          )}
          {expanded && toast.detail && (
            <pre className="mt-2 text-xs bg-gray-100 dark:bg-gray-900 p-2 rounded overflow-x-auto whitespace-pre-wrap break-words">
              {toast.detail}
            </pre>
          )}
        </div>
        <button
          type="button"
          onClick={onClose}
          className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 flex-shrink-0"
          aria-label="关闭"
        >
          ×
        </button>
      </div>
    </div>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) {
    throw new Error("useToast 必须在 ToastProvider 内使用");
  }
  return ctx;
}
