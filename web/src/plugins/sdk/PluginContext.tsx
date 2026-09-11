import { createContext, useContext, type ReactNode } from "react";
import type { PluginRouteDef } from "../types";

/**
 * 插件运行时上下文。
 *
 * 由 `PluginPage`（路由页）与 `PluginSlots`（插槽）注入，插件组件通过
 * `usePluginContext()` / `usePluginId()` 拿到**准确的 pluginId**，
 * 不再依赖解析 URL（路由路径与插件 id 并不总是一致）。
 */
export interface PluginContextValue {
  pluginId: string;
  /** 命中当前页面的路由定义；插槽组件中为 undefined */
  route?: PluginRouteDef;
}

const PluginContext = createContext<PluginContextValue | null>(null);

export function PluginProvider({
  pluginId,
  route,
  children,
}: PluginContextValue & { children: ReactNode }) {
  return (
    <PluginContext.Provider value={{ pluginId, route }}>
      {children}
    </PluginContext.Provider>
  );
}

/** 读取插件上下文；不在插件运行时内时返回 null。 */
export function usePluginContext(): PluginContextValue | null {
  return useContext(PluginContext);
}
