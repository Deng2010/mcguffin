import type { ComponentType, LazyExoticComponent } from "react";

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author?: string;
  homepage?: string;
  permissions_needed: string[];
  enabled: boolean;
  /** 安装来源：code = 前端代码注册；zip = 后台上传安装 */
  source?: "code" | "zip";
  /** zip 插件入口文件（assets 内相对路径） */
  entry?: string;
}

export interface PluginRouteDef {
  path: string;
  label: string;
  icon?: string;
  required_permission?: string;
  nav_placement: "main" | "admin" | "hidden";
  /**
   * 该路由专属的页面组件。省略时回退到 `definePlugin(def, component)`
   * 传入的插件级组件（适合单页面插件；多页面插件应为每条路由指定组件）。
   */
  component?: ComponentType<any> | LazyExoticComponent<ComponentType<any>>;
}

export interface PluginDefinition {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  routes?: PluginRouteDef[];
  slots?: PluginSlotDef[];
  permissions_needed?: string[];
}

export interface PluginSlotDef {
  slot: string;
  component: ComponentType<any>;
}
