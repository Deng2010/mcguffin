/**
 * McGuffin 插件 SDK —— 规范（canonical）类型声明。
 *
 * 本文件描述宿主在 `window.__MCGUFFIN_SDK__` 上暴露的**全部**能力，供独立插件仓库在
 * 不依赖主仓库源码的前提下获得完整类型提示与检查：
 *
 *   const { definePlugin, React, usePluginData } = window.__MCGUFFIN_SDK__;
 *
 * 成员清单与宿主实现严格一一对应，可逐项核对：
 *   - `web/src/main.tsx`                     —— window 上实际挂载的成员集合
 *   - `web/src/plugins/sdk/data.ts`          —— 数据 API（KV / 计数器 / 集合 / 文件 / 用户 / 通知）
 *   - `web/src/plugins/sdk/hooks.ts`         —— React Hooks 封装
 *   - `web/src/plugins/sdk/PluginContext.tsx`—— PluginProvider / usePluginContext
 *   - `web/src/plugins/sdk/PluginSlots.tsx`  —— PluginSlots
 *   - `web/src/plugins/types.ts`             —— 插件定义类型（PluginDefinition 等）
 *
 * ⚠️ 本文件是**唯一事实来源**。插件模板中的
 * `templates/plugin/types/mcguffin-plugin-sdk.d.ts` 必须是本文件的逐字节副本，
 * 由 `web/src/test/plugin-sdk-types.test.ts` 强制校验（改动后请执行 `cp` 同步）。
 *
 * 打包契约（zip 结构、plugin.json、入口 ESM）见 `docs/admin/plugins.md`；
 * 独立插件仓库的开发流程见 `docs/guide/plugin-development.md`。
 */
import type { ComponentType, LazyExoticComponent } from "react";

// ============================================================================
// 一、插件定义类型（与 web/src/plugins/types.ts 完全一致）
// ============================================================================

/**
 * 插件清单：安装进 McGuffin 后由后端维护的记录。
 * 对应 `plugin.json` 的字段加上宿主补充的运行期字段。
 */
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

/** 插件注册的一条前端路由。 */
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

/** `definePlugin()` 的入参：一个插件的自描述（须与 plugin.json 保持一致）。 */
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

/** 插件插入宿主页面的一个插槽组件。 */
export interface PluginSlotDef {
  slot: string;
  component: ComponentType<any>;
}

// ============================================================================
// 二、SDK 数据 API 的类型（与 web/src/plugins/sdk/data.ts 完全一致）
// ============================================================================

/** 插件可见的用户资料（`pluginUserGet` / `pluginUserMe` 返回值）。 */
export interface PluginUserInfo {
  id: string;
  username: string;
  display_name: string;
  avatar_url: string | null;
  role: string;
  effective_role: string;
  team_status: string;
  bio: string;
  created_at: string;
}

/** 团队成员（`pluginUserList` / `usePluginTeamMembers` 的条目）。 */
export interface PluginTeamMember extends PluginUserInfo {
  user_id: string;
  joined_at: string;
}

/** `pluginUserList()` 的返回值。 */
export interface PluginUserListResult {
  members: PluginTeamMember[];
  count: number;
}

/** `pluginWriteFile()` 的返回值。 */
export interface PluginFileWriteResult {
  path: string;
  size: number;
}

// ============================================================================
// 三、React Hooks 的返回值（与 web/src/plugins/sdk/hooks.ts 完全一致）
// ============================================================================

/** `usePluginData()` 的返回值。 */
export interface PluginDataResult<T> {
  value: T | null;
  loading: boolean;
  refresh: () => Promise<void>;
  setValue: import("react").Dispatch<import("react").SetStateAction<T | null>>;
}

/** `usePluginCounter()` 的返回值。 */
export interface PluginCounterResult {
  count: number;
  add: (delta: number) => Promise<number>;
}

/** `usePluginSet()` 的返回值。 */
export interface PluginSetResult {
  members: string[];
  add: (member: string) => Promise<boolean>;
  remove: (member: string) => Promise<boolean>;
  isMember: (member: string) => Promise<boolean>;
  refresh: () => Promise<void>;
}

/** `usePluginKeys()` 的返回值。 */
export interface PluginKeysResult {
  keys: string[];
  refresh: () => Promise<void>;
}

/** `usePluginUserMe()` 的返回值。 */
export interface PluginUserMeResult {
  user: PluginUserInfo | null;
  loading: boolean;
  refresh: () => Promise<void>;
}

/** `usePluginTeamMembers()` 的返回值。 */
export interface PluginTeamMembersResult {
  members: PluginTeamMember[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

// ============================================================================
// 四、插件上下文与插槽组件的 props
// ============================================================================

/**
 * 插件运行时上下文（`PluginContext.tsx`）。
 * 由 `PluginPage`（路由页）与 `PluginSlots`（插槽）注入。
 */
export interface PluginContextValue {
  pluginId: string;
  /** 命中当前页面的路由定义；插槽组件中为 undefined */
  route?: PluginRouteDef;
}

/** `<PluginSlots>` 的 props（`PluginSlots.tsx`）。 */
export interface PluginSlotsProps {
  /** 插槽名称，由主应用预留（如 `member_card_actions`） */
  slot: string;
  /** 透传给插件组件的 props */
  props?: Record<string, unknown>;
}

// ============================================================================
// 五、window.__MCGUFFIN_SDK__
// ============================================================================

/**
 * 宿主暴露在 `window.__MCGUFFIN_SDK__` 上的完整 SDK。
 *
 * 成员的来源（一个不漏、一个不多）：
 *   `React` / `definePlugin` / `PluginSlots` / `PluginProvider` / `usePluginContext`
 *   + `data.ts` 的全部运行时导出 + `hooks.ts` 的全部运行时导出。
 *
 * 所有数据 API 的第一个参数都是 `pluginId`，数据按 `pluginId + namespace` 隔离，
 * 插件之间无法互相访问；调用时传入的权限要求写在各成员注释里（权限在
 * `plugin.json` 的 `permissions_needed` 声明，由管理员在后台授予）。
 */
export interface McGuffinPluginSdk {
  // ── 宿主 React（与主应用同一份实例，切勿把 React 打包进插件）──
  React: typeof import("react");
  /**
   * 宿主 ReactDOM（同一份实例）。
   *
   * 插件通常不需要它（页面渲染在宿主树里），但 UI 库常用 `createPortal`
   * 做弹窗 / 浮层。浏览器里也可以直接 `import { createPortal } from "react-dom"`：
   * 宿主的 import map 把 `react` / `react/jsx-runtime` / `react-dom` 映射到
   * `web/public/plugin-sdk/*.js` 垫片，垫片再转发到这里暴露的实例。
   */
  ReactDOM: typeof import("react-dom");

  // ── 插件注册 ──
  /**
   * 注册插件（zip 入口 ESM 自注册的唯一入口）。
   * @param definition 插件定义，须与 plugin.json 的 id/name/version 一致
   * @param component  插件级默认页面组件（`React.lazy`），单页面插件使用；
   *                   多页面插件应在每条 route 上指定 `component`
   */
  definePlugin: (
    definition: PluginDefinition,
    component?: LazyExoticComponent<ComponentType<unknown>>,
  ) => PluginDefinition;

  // ── 宿主渲染能力 ──
  /** 渲染某个插槽下所有插件组件（宿主页面用；插件一般不需要） */
  PluginSlots: ComponentType<PluginSlotsProps>;
  /** 注入插件上下文（宿主用；插件一般不需要手动包裹） */
  PluginProvider: ComponentType<
    PluginContextValue & { children: import("react").ReactNode }
  >;
  /** 读取当前插件上下文；不在插件运行时内时返回 null */
  usePluginContext: () => PluginContextValue | null;

  // ── 键值存储（需 `storage` 权限）──
  /** 读取一个字符串值（通常自行 JSON.parse；不存在时返回空串） */
  getPluginData: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => Promise<string>;
  /** 写入一个字符串值（单值 ≤ 64 KiB，需自行 JSON.stringify） */
  setPluginData: (
    pluginId: string,
    namespace: string,
    key: string,
    value: string,
  ) => Promise<void>;
  /** 列出某命名空间下的 key（可选前缀过滤） */
  pluginKeys: (
    pluginId: string,
    namespace: string,
    prefix?: string,
  ) => Promise<string[]>;

  // ── 计数器（需 `storage` 权限，服务端原子自增）──
  /** 原子累加 delta，返回累加后的值 */
  pluginAdd: (
    pluginId: string,
    namespace: string,
    key: string,
    delta: number,
  ) => Promise<number>;
  /** 原子 +1 */
  pluginIncr: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => Promise<number>;
  /** 原子 -1 */
  pluginDecr: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => Promise<number>;

  // ── 集合（需 `storage` 权限）──
  /** 加入集合成员，返回是否新增 */
  pluginSetAdd: (
    pluginId: string,
    namespace: string,
    key: string,
    member: string,
  ) => Promise<boolean>;
  /** 移除集合成员，返回是否确实移除 */
  pluginSetRemove: (
    pluginId: string,
    namespace: string,
    key: string,
    member: string,
  ) => Promise<boolean>;
  /** 列出集合全部成员 */
  pluginSetMembers: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => Promise<string[]>;
  /** 判断成员是否在集合中 */
  pluginSetIsMember: (
    pluginId: string,
    namespace: string,
    key: string,
    member: string,
  ) => Promise<boolean>;

  // ── 文件存储（需 `storage` 权限，单文件 ≤ 8 MiB）──
  /** 写入文件（Blob / ArrayBuffer） */
  pluginWriteFile: (
    pluginId: string,
    filePath: string,
    data: Blob | ArrayBuffer,
  ) => Promise<PluginFileWriteResult>;
  /** 读取文件（返回 Blob） */
  pluginReadFile: (pluginId: string, filePath: string) => Promise<Blob>;
  /** 删除文件 */
  pluginDeleteFile: (pluginId: string, filePath: string) => Promise<void>;
  /** 列出文件（可选前缀过滤） */
  pluginListFiles: (pluginId: string, prefix?: string) => Promise<string[]>;

  // ── 通知（需 `notify` 权限）──
  /** 给指定用户发送站内通知（link 为点击后的跳转路径） */
  pluginCreateNotification: (
    pluginId: string,
    userId: string,
    title: string,
    body: string,
    link?: string,
  ) => Promise<void>;

  // ── 用户与团队（需 `read:users` / `read:team` 权限）──
  /** 当前登录用户（无需插件权限） */
  pluginUserMe: (pluginId: string) => Promise<PluginUserInfo>;
  /** 指定用户资料（需 `read:users`） */
  pluginUserGet: (pluginId: string, userId: string) => Promise<PluginUserInfo>;
  /** 团队成员列表（需 `read:team`；write:team / write:team_roles 蕴含 read:team） */
  pluginUserList: (pluginId: string) => Promise<PluginUserListResult>;

  // ── React Hooks：插件 id ──
  /**
   * 当前插件 id（优先读宿主注入的上下文，仅在无上下文时回退解析 URL）。
   * 插件组件内应优先用它，不要自己解析 `location.pathname`。
   */
  usePluginId: () => string;

  // ── React Hooks：键值存储 / 计数器 / 集合 / keys（需 `storage` 权限）──
  /** 响应式读取 KV：值按 JSON 解析，失败/不存在时回退 `options.defaultValue` */
  usePluginData: <T = string>(
    pluginId: string,
    namespace: string,
    key: string,
    options?: { defaultValue?: T },
  ) => PluginDataResult<T>;
  /** 计数器 hook：`add(delta)` 原子累加并同步本地 count */
  usePluginCounter: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => PluginCounterResult;
  /** 集合 hook：add / remove / isMember / refresh */
  usePluginSet: (
    pluginId: string,
    namespace: string,
    key: string,
  ) => PluginSetResult;
  /** keys hook：`refresh()` 重新拉取 key 列表 */
  usePluginKeys: (
    pluginId: string,
    namespace: string,
    prefix?: string,
  ) => PluginKeysResult;

  // ── React Hooks：用户与团队 ──
  /** 当前登录用户 hook（无需插件权限） */
  usePluginUserMe: (pluginId: string) => PluginUserMeResult;
  /** 指定用户 hook（需 `read:users`），直接返回用户对象或 null */
  usePluginUser: (pluginId: string, userId: string) => PluginUserInfo | null;
  /** 团队成员 hook（需 `read:team`），带 loading / error / refresh */
  usePluginTeamMembers: (pluginId: string) => PluginTeamMembersResult;
}
