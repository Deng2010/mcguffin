import { lazy, type ComponentType, type LazyExoticComponent } from "react";
import { PluginRegistry } from "../registry";
import type { PluginDefinition } from "../types";

/**
 * Declare a React plugin and register it with the PluginRegistry.
 *
 * ZIP 插件的入口 ESM（由 registry 动态 import）通过全局 SDK 调用它：
 *
 *   // index.js —— zip 包入口
 *   const { definePlugin, React } = window.__MCGUFFIN_SDK__
 *
 *   definePlugin(
 *     {
 *       id: 'my-plugin',
 *       name: 'My Plugin',
 *       version: '1.0.0',
 *       routes: [
 *         { path: '/plugins/my', label: 'My Plugin', icon: '🔌', nav_placement: 'main' },
 *       ],
 *       slots: [
 *         { slot: 'member_card_actions', component: MyButton },
 *       ],
 *     },
 *     React.lazy(() => import('./MyPage.js')),
 *   )
 */
export function definePlugin(
  definition: PluginDefinition,
  component?: LazyExoticComponent<ComponentType<unknown>>,
): PluginDefinition {
  const registry = PluginRegistry.getInstance();
  registry.register(definition, component);
  return definition;
}
