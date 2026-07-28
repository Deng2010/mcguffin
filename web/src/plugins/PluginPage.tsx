import { Suspense } from "react";
import { useParams } from "react-router-dom";
import { PluginRegistry } from "./registry";

interface PluginPageProps {
  pluginId?: string;
}

export default function PluginPage({
  pluginId: pluginIdProp,
}: PluginPageProps) {
  const { pluginId: pluginIdParam } = useParams<{ pluginId: string }>();
  const pluginId = pluginIdProp ?? pluginIdParam;
  const registry = PluginRegistry.getInstance();
  const component = pluginId ? registry.getComponent(pluginId) : null;

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
    <Suspense
      fallback={
        <div className="text-center py-12 text-gray-400">加载中...</div>
      }
    >
      <Component />
    </Suspense>
  );
}
