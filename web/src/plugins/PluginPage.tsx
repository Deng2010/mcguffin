import { Suspense } from "react";
import { useParams } from "react-router-dom";
import { PluginRegistry } from "./registry";
import ErrorBoundary from "../errors/ErrorBoundary";
import { PluginProvider } from "./sdk/PluginContext";
import type { PluginRouteDef } from "./types";

interface PluginPageProps {
  pluginId?: string;
  /** 命中该页面的路由定义（由 routes.tsx 传入），用于按路由选择组件与注入上下文 */
  route?: PluginRouteDef;
}

export default function PluginPage({
  pluginId: pluginIdProp,
  route,
}: PluginPageProps) {
  const { pluginId: pluginIdParam } = useParams<{ pluginId: string }>();
  const pluginId = pluginIdProp ?? pluginIdParam;
  const registry = PluginRegistry.getInstance();
  // 路由级组件优先；未指定时回退到 definePlugin 的插件级组件
  const component = pluginId
    ? (route?.component ?? registry.getComponent(pluginId))
    : null;

  if (!pluginId || !component) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <p className="text-gray-500 dark:text-gray-400">插件未找到或未加载</p>
      </div>
    );
  }

  if (!registry.isPluginEnabled(pluginId)) {
    return (
      <div className="min-h-screen flex flex-col items-center justify-center gap-4">
        <div className="text-6xl">🚫</div>
        <h1 className="text-xl font-semibold text-gray-700 dark:text-gray-300">
          插件已被禁用
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400">
          该插件已被管理员禁用，如需使用请联系管理员启用
        </p>
      </div>
    );
  }

  const Component = component;
  return (
    <ErrorBoundary
      scope={`plugin:${pluginId}`}
      fallback={(error) => (
        <div className="min-h-[40vh] flex flex-col items-center justify-center gap-3 px-6 text-center">
          <div className="text-4xl">🧩</div>
          <h1 className="text-lg font-semibold text-gray-700 dark:text-gray-300">
            插件「{pluginId}」出错了
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 max-w-md break-words">
            {error.message}
          </p>
          <p className="text-xs text-gray-400 dark:text-gray-500">
            错误已自动上报，其它功能不受影响。
          </p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            className="mt-1 border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
          >
            重新加载
          </button>
        </div>
      )}
    >
      <Suspense
        fallback={
          <div className="text-center py-12 text-gray-400">加载中...</div>
        }
      >
        <PluginProvider pluginId={pluginId} route={route}>
          <Component />
        </PluginProvider>
      </Suspense>
    </ErrorBoundary>
  );
}
