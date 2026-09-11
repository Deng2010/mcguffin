import { PluginRegistry } from "../registry";
import ErrorBoundary from "../../errors/ErrorBoundary";
import { PluginProvider } from "./PluginContext";

interface PluginSlotsProps {
  slot: string;
  props?: Record<string, unknown>;
}

/**
 * Render all plugin components registered for a named slot.
 * Used like:
 *   <PluginSlots slot="member_card_actions" props={{ member }} />
 *
 * 每个插件组件都包在独立的 ErrorBoundary 中：单个插件崩溃只替换自己那块，
 * 不会拖垮宿主页面，同时错误会自动上报（scope = plugin:{id}）。
 */
export default function PluginSlots({ slot, props = {} }: PluginSlotsProps) {
  const registry = PluginRegistry.getInstance();
  const components = registry.getSlotComponents(slot);

  if (components.length === 0) return null;

  return (
    <>
      {components.map(({ pluginId, component: Component }) => (
        <ErrorBoundary
          key={pluginId}
          scope={`plugin:${pluginId}`}
          fallback={
            <div className="border border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950/30 px-3 py-2 text-xs text-red-600 dark:text-red-400">
              插件「{pluginId}」组件渲染出错，已自动上报
            </div>
          }
        >
          <PluginProvider pluginId={pluginId}>
            <Component {...props} />
          </PluginProvider>
        </ErrorBoundary>
      ))}
    </>
  );
}
