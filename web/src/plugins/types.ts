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
  /**
   * 安装来源：`zip` = 后台上传 .zip 安装（当前唯一安装方式）；
   * `code` = 早期的前端代码注册，机制已移除，仅历史数据可能残留。
   */
  source?: "code" | "zip";
  /** zip 插件入口文件（assets 内相对路径） */
  entry?: string;
  /** 安装来源 URL（从 URL 安装/更新时记录；本地上传 .zip 安装时为空） */
  source_url?: string;
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
