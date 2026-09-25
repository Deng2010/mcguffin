# 🧩 插件开发指南（独立仓库）

> 面向插件作者：如何在**独立仓库**里开发、构建、发布并安装 McGuffin 前端插件。
> 打包契约（zip 结构 / plugin.json / 入口 ESM）见 [插件管理](../admin/plugins.md)，
> 宿主 SDK 实现在 `web/src/plugins/sdk/`，可直接复制的骨架在 `templates/plugin/`。

---

## 为什么插件放在独立仓库

McGuffin 主仓库**只包含插件系统本身**（注册表、SDK、类型、插槽渲染），不含任何具体插件：

- `web/src/plugins/registry.ts` —— 插件注册中心（含 zip 插件的动态加载）
- `web/src/plugins/sdk/` —— 宿主通过 `window.__MCGUFFIN_SDK__` 暴露的全部能力
- `web/src/plugins/types.ts` —— 插件定义类型
- 具体插件的代码**不在源码树里**：安装后解压到数据目录 `plugins/{id}/assets/`，
  由 `GET /api/plugins/{id}/assets/*` 公开托管，前端动态 `import()` 其入口 ESM

插件独立成仓的好处：

| 维度 | 独立仓库 | 放进主仓库 |
|------|----------|------------|
| 依赖 | 自己一份 `package.json`，随便升级 Vite / TS | 与主应用依赖耦合 |
| 发版 | 打 tag 即发布，不必等宿主发版 | 每次改动都要走主仓库 CI 与评审 |
| 构建 | 只需 Node，不必装 Rust 与整个前端 | 要跑全量构建 |
| 权限 | 插件权限与主应用角色权限是两套体系，互不影响 | 同左 |

代价是：插件只能用 `window.__MCGUFFIN_SDK__` 暴露的能力，**不能 import 主应用代码**
（宿主的内部组件、service、store 都不在 SDK 范围内）。

---

## 模板骨架

`templates/plugin/` 是一个可直接复制成新仓库的最小骨架：

```bash
cp -R <mcguffin>/templates/plugin ~/my-plugin && cd ~/my-plugin
rm -rf dist node_modules
npm install
npm run zip        # → dist/my-plugin.zip
```

需要改的地方只有三处：

| 文件 | 改什么 |
|------|--------|
| `plugin.json` | `id` / `name` / `description` / `author` / `permissions_needed` |
| `src/index.tsx` | `PLUGIN_ID`、`name`、`version`、`routes` |
| `package.json` | `name`（仅本地用，也决定 css 产物名） |

> ⚠️ `id` 一旦发布就不要再改：插件数据按 id 隔离，改 id 等于换了一个新插件；
> 同时 id 决定 zip 文件名 `dist/<id>.zip`。

目录与命令：

```
├── plugin.json                  # 插件清单（zip 根目录必须有它）
├── vite.config.ts               # lib 模式；产物 dist/index.js；React 不打包
├── src/index.tsx                # 入口：definePlugin() 自注册
├── src/Page.tsx                 # 示例页面（KV 读写）
├── types/mcguffin-plugin-sdk.d.ts  # SDK 类型（从主仓库同步，勿手改）
├── types/global.d.ts            # window.__MCGUFFIN_SDK__ 全局声明
└── scripts/build-zip.mjs        # 构建 + 打包 zip
```

| 命令 | 作用 |
|------|------|
| `npm run dev` | `vite build --watch`，改代码即增量构建 |
| `npm run build` | `tsc --noEmit` + `vite build` |
| `npm run zip` | 上面两步 + 打包 `dist/<id>.zip` |

最小入口长这样（React 与 `definePlugin` 都来自全局 SDK）：

```js
const { definePlugin, React } = window.__MCGUFFIN_SDK__;

definePlugin(
  {
    id: "my-plugin",
    name: "我的插件",
    version: "1.0.0",
    permissions_needed: ["storage"],
    routes: [
      {
        path: "/plugins/my",
        label: "我的插件",
        icon: "🔌",
        nav_placement: "main",
        component: React.lazy(() => import("./Page")),
      },
    ],
  },
);
```

多页面插件在每条 route 上各指定 `component`；单页面插件可以省略 `component`，
改用 `definePlugin(def, React.lazy(() => import("./Page")))` 的第二个参数。

---

## SDK 类型同步

宿主在 `window.__MCGUFFIN_SDK__` 上暴露的成员，其**规范类型声明**是主仓库的
`web/src/plugins/sdk/plugin-sdk.d.ts`。插件仓库使用它的逐字节副本：

```bash
cp <mcguffin>/web/src/plugins/sdk/plugin-sdk.d.ts types/mcguffin-plugin-sdk.d.ts
```

- 主仓库的 `web/src/test/plugin-sdk-types.test.ts` 会强制校验两份文件完全一致，
  所以**不要手改**插件仓库里的副本（否则就是类型漂移）。
- 宿主 SDK 升级后（新增成员或调整签名），重新 `cp` 一次并跑 `npm run build`，
  `tsc` 会立刻告诉你哪些调用不兼容。

### SDK 暴露了什么

`web/src/main.tsx` 挂到 `window.__MCGUFFIN_SDK__` 上的成员是**固定清单**：

| 成员 | 说明 | 权限 |
|------|------|------|
| `React` | 宿主 React 实例（唯一一份，勿打包自己的 React） | — |
| `definePlugin(def, component?)` | 注册插件（zip 入口自注册的唯一方式） | — |
| `PluginSlots` / `PluginProvider` / `usePluginContext` | 插槽渲染与插件上下文（宿主侧用） | — |
| `getPluginData` / `setPluginData` / `pluginKeys` | KV 读写与 key 列表 | `storage` |
| `pluginAdd` / `pluginIncr` / `pluginDecr` | 原子计数器 | `storage` |
| `pluginSetAdd` / `pluginSetRemove` / `pluginSetMembers` / `pluginSetIsMember` | 集合 | `storage` |
| `pluginWriteFile` / `pluginReadFile` / `pluginDeleteFile` / `pluginListFiles` | 文件存储（单文件 ≤ 8 MiB） | `storage` |
| `pluginCreateNotification` | 给用户发站内通知 | `notify` |
| `pluginUserMe` | 当前登录用户 | — |
| `pluginUserGet` | 指定用户资料 | `read:users`（邮箱需 `read:users:email`） |
| `pluginUserList` | 团队成员列表 | `read:team` |
| `usePluginId` / `usePluginContext` | 取当前插件 id / 上下文（不要解析 URL） | — |
| `usePluginData` / `usePluginCounter` / `usePluginSet` / `usePluginKeys` | 数据 API 的 React 封装 | 同对应数据 API |
| `usePluginUserMe` / `usePluginUser` / `usePluginTeamMembers` | 用户 / 团队 Hook | 同对应数据 API |

未列出的成员一律不可用；需要新能力时先在主仓库扩展 SDK。

### 权限模型

插件权限与主应用角色权限是**两套独立体系**：

- 插件私有数据接口（`/plugins/{id}/data`、`/files`、`/users`…）校验**插件权限**（`permissions_needed`）；
- 插件调用主应用 API（题目、赛事、团队审批等）以**当前登录用户**的权限执行，与插件权限无关。

已知权限名（未知名会被静默忽略）：`storage`、`notify`、`read:team`、`write:team`、
`write:team_roles`、`read:problems`、`read:contests`、`read:posts`、`read:users`、`read:users:email`。
其中 `write:team` / `write:team_roles` 蕴含 `read:team`，`read:users:email` 蕴含 `read:users`。

已禁用插件或全局禁用状态下，所有数据接口一律返回 403。

---

## 构建与 zip 打包

zip 插件是**独立于主应用构建**的 ESM bundle，约定如下：

```
my-plugin.zip
├── plugin.json        # 必需，必须在 zip 根目录（后端按名精确读取）
├── index.js           # 入口（plugin.json 的 entry，默认 index.js）
└── Page-<hash>.js     # 动态 import 的页面 chunk，与入口同目录、相对路径引用
```

| 限制 | 值 |
|------|-----|
| 文件数 | ≤ 512 |
| 单文件 | ≤ 8 MiB |
| 解压总量 | ≤ 64 MiB |

Vite 配置要点（模板已配好）：

```ts
export default defineConfig({
  build: {
    target: "esnext",
    lib: { entry: "src/index.tsx", formats: ["es"], fileName: () => "index.js" },
    rollupOptions: {
      // React 由宿主提供，绝不打包进插件（多份 React 实例会让 hooks / context 失效）；
      // 这几个裸模块名由宿主的 import map 解析（见下文「两个容易踩的坑」）
      external: [
        "react",
        "react-dom",
        "react/jsx-runtime",
        "react/jsx-dev-runtime",
      ],
    },
  },
});
```

### 两个容易踩的坑

**JSX / `import "react"` 是开箱可用的。** 宿主 `index.html` 里的 import map 把
`react` / `react/jsx-runtime` / `react/jsx-dev-runtime` / `react-dom` 映射到
`/plugin-sdk/*.js` 垫片，垫片再转发到宿主的同一份 React 实例。所以插件里直接写
`import { useState } from "react"` 与 JSX 即可，只要把上面 `external` 里的几个模块
保持 external（别打进产物）。**其它裸模块名不在映射范围内**：第三方依赖保持默认打包，
`npm run zip` 发现未映射的裸导入时会给出警告。

**别用 Tailwind 工具类。** 宿主的 Tailwind 构建扫描不到插件源码，
`className="p-4"` 之类不会生效；用内联样式或自带 CSS（`import "./style.css"`
会产出 `dist/<package name>.css`，但 lib 模式不会自动注入，需自行插入 `<link>`）。

---

## 安装

安装接口仅**超级管理员**可用，两种方式：

| 方式 | 后台入口 | 接口 |
|------|----------|------|
| 上传 zip | 插件管理 → 「上传 .zip 文件」 | `POST /api/v1/admin/plugins/install-zip` |
| 从 URL 安装 | 插件管理 → 「从 URL 安装」 | `POST /api/v1/admin/plugins/install-url` |

「从 URL 安装」由服务端下载 zip（`http` / `https`，建议 https）并记录来源 URL，
便于之后一键更新。GitHub Release 提供稳定地址：

```
https://github.com/<owner>/<repo>/releases/latest/download/<id>.zip
```

`latest/download` 永远指向最新一版资产，因此这条 URL 可以长期不变。

安装后：

- 文件解压到数据目录 `plugins/{id}/assets/`，经 `GET /api/plugins/{id}/assets/*` 公开托管；
- 前端发现 zip 插件后动态 `import()` 其入口 ESM，入口调用 `definePlugin()` 自注册，
  页面与插槽**立即生效**，无需刷新或重启；
- 首次安装按 `plugin.json` 声明的已知权限自动授予；入口加载失败会在插件列表显示「加载失败」；
- 安装会写入审计日志（`plugin.install`，资源 `plugin:{id}`）。

---

## 更新语义

后台对已安装插件提供两种更新方式：

| 方式 | 后台入口 | 接口 |
|------|----------|------|
| 更新（上传新 zip） | 插件管理 → 「更新」 | `POST /api/v1/admin/plugins/{id}/update-zip` |
| 从原 URL 更新 | 插件管理 → 「从原 URL 更新」 | `POST /api/v1/admin/plugins/{id}/update` |

「从原 URL 更新」只对**从 URL 安装**的插件可用（没有记录来源 URL 时返回
`PLUGIN_UPDATE_UNAVAILABLE`）。两种方式都要求包内 `plugin.json` 的 id 与插件 id 一致。

更新时后端保证：

| 项目 | 行为 |
|------|------|
| 插件 KV 数据（`plugin_data` 表）与文件存储（`plugins/{id}/files/`） | **保留** |
| 启用 / 禁用状态 | **保留** |
| 已授权权限 | **保留（冻结）**，新版本声明不会自动扩权 |
| 静态资源 `plugins/{id}/assets/` | 整体替换为新版本 |
| 新版本额外申请的权限 | 不会自动授予；需管理员在后台插件页勾选保存后才生效 |

因此升级后如果新版本用了新权限，记得让管理员在后台插件页补勾，否则相关接口 403。
更新会写入审计日志（`plugin.update`）。

> 注意：插件入口自注册时刷新的是 `name` / `version` / `description` 等元信息，
> 权限与安装来源永远不会被自注册改写（注册接口无需鉴权，所以权限只能由超管调整）。
> 请让 `plugin.json` 与 `definePlugin()` 的 `id` / `name` / `version` 保持一致，
> 否则后台显示的版本会与 zip 版本不一致。

---

## 版本与兼容建议

- **插件版本号**：`plugin.json` 与 `definePlugin()` 都用语义化版本，每次发布递增，
  便于在后台插件列表辨认当前安装的是哪一版。
- **没有 API 版本协商**：宿主不校验插件声明的 SDK 版本，兼容性靠
  `plugin-sdk.d.ts` 这份快照 + 类型检查。插件升级宿主后请重新同步类型并跑一次
  `npm run build`，`tsc` 报错即为不兼容点。
- **优先用稳定 API**：数据 API 与 Hooks 的签名保持向后兼容；不要在插件里
  依赖 `PluginPage`、`registry.ts` 等宿主内部实现。
- **发布**：模板自带 `.github/workflows/release.yml`，推 `v*` tag 即构建 zip
  并作为 release 资产上传（资产名固定为 `<id>.zip`）。
- **卸载**：后台「卸载」会删除插件的持久化数据（KV、文件存储与 assets 目录），
  不可恢复；插件侧不要把这当成常规更新手段。
- **安全**：zip 插件以主应用同等权限运行（可访问 DOM 与已登录会话），
  只安装来源可信的插件；建议把 zip 放在可控的 https 地址上，必要时固定版本 URL。

---

## 调试与排错

| 现象 | 排查方向 |
|------|----------|
| 插件列表显示「加载失败」 | 悬停看原因；控制台按 `[plugin]` 过滤。多为入口 ESM 报错 |
| `Failed to resolve module specifier "xxx"` | 该裸模块名不在宿主 import map 里（只有 react 系列 4 个），把它按普通依赖打包进来（别加进 `external`）或改相对路径导入 |
| 数据接口 403 | 插件被禁用 / 全局禁用 / 权限未授予（后台插件页勾选） |
| 插件不显示在导航 | `nav_placement` 是否为 `main`/`admin`，是否有 `required_permission` |
| 页面路由 404 | 路由 `path` 与导航项的对应关系；插件是否已被卸载（本地注册表会即时移除） |
| zip 安装失败 | 包内根目录缺 `plugin.json`、`id` 非法、`entry` 指向的文件不存在、超过体积限制 |

---

## 参考实现：榜榜糖（lollipop-rank）

模板之外还有一个**完整的真实插件**可以参考：榜榜糖（团队成员点糖 / 周冠军小游戏）。
它原本以「内联插件」形式躺在主仓库（`web/src/plugins/lollipop-rank/`，基于已移除的
前端代码注册机制），现已按插件规范重写并迁到独立仓库，本地与主仓库同级：

```
mcguffin/                              # 宿主（本仓库）
mcguffin-plugin-lollipop-rank/         # 插件仓库（独立 git repo）
```

它比模板多演示了这些常见需求：

| 需求 | 做法 |
|------|------|
| 插件内可调参数 | 参数存 KV（`config/params`），配一个仅管理员可见的设置面板（区间校验 + 成功率预览），管理员判定用 `pluginUserMe().effective_role` |
| 服务端原子计数 | 每日次数用 `pluginIncr` / `pluginAdd`，替代 KV「读-改-写」，避免多 Tab 并发超额 |
| 周期任务 | 插件没有后台任务上下文，改用「打开页面时懒结算 + KV 占位锁」实现每周一 0:00 结算 |
| 自带样式 | 把 `?inline` 的 CSS 字符串打进 JS 并在运行时注入 `<style>`，深色模式跟随宿主 `html.dark` |
| 数据兼容 | 插件 id 与 KV 布局保持不变，升级 zip 不丢数据 |
| 质量保障 | 单测（纯逻辑 / 存储 / 清单 / jsdom 页面集成）+ 产物冒烟测试（真正 `import()` 入口 ESM 校验自注册结果）+ CI / Release 工作流 |

仓库为本地兄弟目录 `mcguffin-plugin-lollipop-rank`；推到远端后可长期用
`https://github.com/<owner>/mcguffin-plugin-lollipop-rank/releases/latest/download/lollipop-rank.zip`
在后台「从 URL 安装 / 更新」。

---

## 相关文档

| 文档 | 内容 |
|------|------|
| [插件管理](../admin/plugins.md) | 安装 / 卸载 / 打包契约 / plugin.json / 权限清单 |
| [开发环境搭建](development.md) | 主仓库前端 / 后端本地开发 |
| `templates/plugin/` | 可直接复制的插件仓库骨架 |
| `mcguffin-plugin-lollipop-rank`（兄弟目录） | 榜榜糖插件仓库：完整参考实现 |
| `web/src/plugins/sdk/plugin-sdk.d.ts` | SDK 规范类型声明（插件仓库需同步此文件） |
