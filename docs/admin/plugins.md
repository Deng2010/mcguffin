# 🧩 插件管理

McGuffin 支持通过插件系统扩展前端功能。插件可以添加独立页面、在现有页面插入组件、使用键值存储持久化数据。

## 插件安装方式

插件以 **ZIP 包**安装，由后端管理其生命周期：既可以**上传本地 .zip**，也可以**从 URL 安装**
（服务端下载并记录来源，便于之后一键更新）。主仓库**不含任何具体插件**，只包含插件系统本身
（`web/src/plugins/`）；早期基于 `import.meta.glob` 的前端代码注册机制已移除。

---

## 安装插件

### 方式一：上传 .zip

1. 进入后台 → 插件管理 → 点击「上传 .zip 文件」
2. 选择一个包含 `plugin.json` 的 .zip 包（格式见下文「打包分发插件」）
3. 安装成功后插件立即出现在列表中，前端会**动态加载**其入口模块，无需刷新页面

### 方式二：从 URL 安装

1. 在「从 URL 安装」输入框填写**可直接下载 .zip** 的地址，例如插件仓库 Release 的稳定地址
   `https://github.com/<owner>/<repo>/releases/latest/download/<id>.zip`
2. 点击「从 URL 安装」，服务端下载并安装（下载失败/体积超限会直接报错）
3. 安装成功后会记录**来源 URL**，插件列表里可看到「来源」并一键「从原 URL 更新」

限制与约定：

- 仅支持 `http` / `https`，不接受带账号密码的 URL；云元数据地址（`169.254.169.254`）被拒绝
- 包体上限 64 MiB（与服务端解压后总大小上限一致），下载超时 120s
- 该接口仅超级管理员可用；内网镜像地址可以正常使用

无论哪种方式，安装后的插件文件都存放在数据目录 `plugins/{插件id}/assets/` 下，通过
`GET /api/plugins/{id}/assets/*` 公开访问（仅静态代码；数据接口仍需权限校验）。

---

## 更新插件

插件列表每行的「更新」按钮支持两种更新方式：

| 方式 | 操作 | 适用场景 |
|------|------|----------|
| 上传新 .zip | 点「更新」→ 选择新的 .zip（包内 `plugin.json` 的 id 必须与该插件一致） | 本地上传、手工发布 |
| 从原 URL 更新 | 点「从原 URL 更新」（仅当插件记录了来源 URL 时出现） | 插件发布在 Release / 自建分发地址 |

更新语义（与「卸载后重装」不同，务必注意）：

- **插件数据保留**：KV / 计数器 / 集合 / 文件存储（`plugins/{id}/files/`）都不受影响
- **启用状态保留**：更新前被禁用的插件，更新后仍是禁用状态
- **权限冻结**：不会因为新版本声明了新权限就自动授权。若新版本申请了额外权限，
  响应里会给出提示（`new_permissions`），需要管理员在「权限」里手动勾选后才生效
- **资产整体替换**：`assets/` 目录被新包覆盖，旧的入口与资源文件不再存在
- 更新操作会写入审计日志（`plugin.update`）

> 通过「上传 .zip」安装的插件没有来源 URL，只能用「更新」按钮上传新包更新；
> 此时调用「从原 URL 更新」会返回 `PLUGIN_UPDATE_UNAVAILABLE`。

---

## 打包分发插件

ZIP 插件是独立于主应用构建的 ESM bundle，约定如下：

### zip 包结构

```
my-plugin.zip
├── plugin.json      # 必需，插件清单
├── index.js         # 默认入口（可在 plugin.json 的 entry 字段改）
└── ...              # 其它资源（css / 图片 / 子模块 js）
```

### plugin.json

```json
{
  "id": "my-plugin",
  "name": "我的插件",
  "version": "1.0.0",
  "description": "可选描述",
  "author": "可选作者",
  "permissions_needed": ["storage", "read:team"],
  "entry": "index.js"
}
```

- `id`：以字母或数字开头，仅含字母数字、`-`、`_`，长度 ≤ 64
- `entry`：zip 内相对路径，默认 `index.js`
- `permissions_needed`：仅接受已知权限字符串（见下文 SDK API 各接口标注）

### 入口模块契约

入口文件必须是 **ESM**，通过主应用暴露的全局 SDK 注册自身，
**不要**把 React 打包进插件（复用主应用的副本，避免多实例问题）：

```js
const { definePlugin, React, usePluginTeamMembers } = window.__MCGUFFIN_SDK__;

function MyPage() {
  const { members } = usePluginTeamMembers("my-plugin");
  return React.createElement("div", null, `成员数: ${members.length}`);
}

definePlugin(
  {
    id: "my-plugin",
    name: "我的插件",
    version: "1.0.0",
    permissions_needed: ["storage", "read:team"],
    routes: [
      { path: "/plugins/my", label: "我的插件", icon: "🔌", nav_placement: "main" },
    ],
  },
  React.lazy(() => Promise.resolve({ default: MyPage })),
);
```

`window.__MCGUFFIN_SDK__` 暴露的完整能力：`React`、`ReactDOM`、`definePlugin`、
`PluginSlots`，以及下文「SDK API」列出的全部数据函数与 React Hooks。

### 关于 React / JSX

宿主 `index.html` 提供了 **import map**，把 `react` / `react/jsx-runtime` /
`react/jsx-dev-runtime` / `react-dom` 映射到 `/plugin-sdk/*.js` 垫片，垫片再转发到
`window.__MCGUFFIN_SDK__` 里的同一份 React 实例。所以插件产物里可以直接写

```js
import { useState } from "react";   // 解析到宿主 React，不会打包出第二份
```

以及 JSX（编译产物 `import { jsx } from "react/jsx-runtime"` 同样由 import map 接住）。
构建时请把这几个模块保持 `external`（不要打进产物）；其它裸模块名不在映射范围内，
需要按普通依赖打包进来。

### 安全说明

ZIP 插件代码以主应用同等权限运行在浏览器中（可访问 DOM 与已登录用户的会话），
请只安装来源可信的插件。安装接口仅超级管理员可用；后端对 zip 包做了
文件数（512）、单文件（8 MiB）、解压总大小（64 MiB）与路径穿越防护。

---

## 卸载插件

1. 进入后台 → 插件管理
2. 在插件列表中找到要卸载的插件
3. 点击「卸载」按钮确认

卸载会同时删除插件的持久化数据（KV 存储、文件存储与 zip 资产目录）。

---

## 数据持久化

插件系统状态全部持久化，重启不丢失：

| 数据 | 位置 |
|------|------|
| 插件清单（含启用状态、安装来源与来源 URL） | SQLite `plugins` 表 |
| 插件 KV / 计数器 / 集合数据 | SQLite `plugin_data` 表 |
| 全局插件开关 | SQLite `meta` 表（`plugins_disabled`） |
| 插件文件存储 | 数据目录 `plugins/{id}/files/` |
| ZIP 插件资产 | 数据目录 `plugins/{id}/assets/` |

所有数据接口（KV、计数器、集合、keys、文件）均在服务端原子完成并即时写回 SQLite。

### 配额与限制

| 项目 | 上限 |
|------|------|
| 单个 KV 值（含集合序列化结果） | 64 KiB |
| 单插件 KV 条目总数（跨命名空间） | 2000 |
| `namespace` / `key` / 集合成员长度 | 64 / 256 / 256 字符 |
| 单个插件文件 | 8 MiB |
| zip 插件包 | 512 个文件、单文件 8 MiB、解压总量 64 MiB |

超出配额的写入返回 400（`PLUGIN_DATA_INVALID` / `PLUGIN_INVALID_PACKAGE`），
更新已存在的 key 不受条目总数限制。

### 审计

插件生命周期操作会写入审计日志（`GET /api/v1/admin/audit-log`，需 `view_stats`）：
`plugin.install` / `plugin.update` / `plugin.uninstall` / `plugin.enable` / `plugin.disable` /
`plugin.set_permissions` / `plugin.global_toggle`，资源标识为 `plugin:{id}`。

---

## 开发插件

### 开发流程

插件独立于主应用构建，**建议单独一个 repo**（源码不放进主仓库的 `web/src/plugins/`）：

1. 复制模板骨架 `templates/plugin/` 到新 repo（Vite lib 模式、`plugin.json`、zip 打包脚本都已配好），
   详见 `docs/guide/plugin-development.md`；
2. 编写页面组件与入口 ESM：入口通过 `window.__MCGUFFIN_SDK__.definePlugin()` 自注册
   （示例见上文「入口模块契约」）；类型提示来自模板的 `types/mcguffin-plugin-sdk.d.ts`
   （与主仓库 `web/src/plugins/sdk/plugin-sdk.d.ts` 保持一致）；
3. 构建并打包：`bun run build && bun run zip` 产出 `<id>.zip`；
4. 安装到 McGuffin：后台上传 zip，或填写 Release 稳定地址用「从 URL 安装」；
   之后发版只需点「从原 URL 更新」，插件数据与已授权权限都会保留。

调试建议：入口加载失败的原因会显示在插件列表的「加载失败」标记上（悬停查看详情），
也可在浏览器控制台按 `[plugin]` 前缀过滤日志。

### 多页面插件（路由级组件）

`definePlugin(def, component)` 的第二个参数是**插件级默认组件**，适合单页面插件。
一个插件有多条路由时，应在每条路由上指定各自的组件；未指定的路由回退到插件级组件：

```typescript
definePlugin({
  id: "my-plugin",
  name: "我的插件",
  version: "1.0.0",
  routes: [
    { path: "/plugins/my", label: "我的插件", nav_placement: "main",
      component: React.lazy(() => import("./HomePage")) },
    { path: "/plugins/my/stats", label: "统计", nav_placement: "main",
      component: React.lazy(() => import("./StatsPage")) },
  ],
});
```

### 获取当前插件 id

插件组件内用 `usePluginId()` / `usePluginContext()` 获取**准确的** pluginId
（宿主通过 `PluginProvider` 注入；不要解析 URL —— 路由路径与插件 id 并不总是一致）：

```js
const { usePluginId, usePluginContext } = window.__MCGUFFIN_SDK__;

const pluginId = usePluginId();                 // 可靠来源：上下文
const { route } = usePluginContext() ?? {};     // 命中当前页面的路由定义（插槽中为 undefined）
```

### `definePlugin()` 配置项

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | `string` | ✅ | 唯一标识符，用于数据隔离和路由匹配 |
| `name` | `string` | ✅ | 显示名称 |
| `version` | `string` | ✅ | 语义化版本号 |
| `description` | `string` | | 简短描述，显示在管理后台插件列表 |
| `author` | `string` | | 作者名 |
| `routes` | `PluginRouteDef[]` | | 路由定义，注册插件页面 |
| `slots` | `PluginSlotDef[]` | | 插槽定义，在现有页面插入组件 |
| `permissions_needed` | `string[]` | | 插件需要的插件权限（见「权限模型」），仅接受已知权限名 |

### 路由配置 (`PluginRouteDef`)

```typescript
{
  path: "/plugins/my",              // 路由路径
  label: "我的插件",                 // 导航栏显示文本
  icon: "🔌",                        // 图标（emoji 或字符）
  required_permission: "access_admin", // 可选，访问该路由需要的权限
  nav_placement: "main",             // main=主导航 | admin=管理后台 | hidden=不显示
}
```

路由页面组件需单独导出，建议用 `React.lazy()` 实现代码分割：

```js
const { definePlugin, React } = window.__MCGUFFIN_SDK__;

const plugin = definePlugin(
  { /* ...路由定义... */ },
  React.lazy(() => import("./MyPage.js"))
);
```

### 插槽 (`PluginSlotDef`)

插槽允许插件在现有页面的指定位置插入组件，无需修改主应用代码。

```typescript
slots: [
  {
    slot: "member_card_actions",  // 插槽名称（由主应用预留）
    component: MyActionButton,     // 要渲染的 React 组件
  },
]
```

主应用通过 `<PluginSlots slot="member_card_actions" />` 渲染该位置的所有插件组件。

## SDK API

插件通过全局 `window.__MCGUFFIN_SDK__` 访问系统能力（宿主侧实现在 `web/src/plugins/sdk/`）。所有 API 调用传入的 `pluginId` 自动限定数据访问范围，不同插件之间数据隔离。

### 键值存储

```typescript
const { getPluginData, setPluginData, pluginKeys } = window.__MCGUFFIN_SDK__;

// 写入
await setPluginData("my-plugin", "config", "theme", JSON.stringify({ color: "blue" }));

// 读取
const raw = await getPluginData("my-plugin", "config", "theme");
const config = raw ? JSON.parse(raw) : null;

// 列出某命名空间下所有 key
const keys = await pluginKeys("my-plugin", "config");
```

### 计数器

```typescript
const { pluginIncr, pluginDecr, pluginAdd } = window.__MCGUFFIN_SDK__;

const newVal = await pluginIncr("my-plugin", "stats", "visits");  // +1
await pluginDecr("my-plugin", "stats", "pending");                 // -1
await pluginAdd("my-plugin", "score", "total", 100);               // +100
```

### 集合

```typescript
const { pluginSetAdd, pluginSetRemove, pluginSetMembers } = window.__MCGUFFIN_SDK__;

await pluginSetAdd("my-plugin", "groups", "admins", "user-123");
await pluginSetRemove("my-plugin", "groups", "admins", "user-123");
const members = await pluginSetMembers("my-plugin", "groups", "admins");
```

### 文件存储

```typescript
const { pluginWriteFile, pluginReadFile, pluginListFiles, pluginDeleteFile } =
  window.__MCGUFFIN_SDK__;

// 写入文件
await pluginWriteFile("my-plugin", "assets/logo.png", fileBlob);

// 读取文件（返回 Blob）
const blob = await pluginReadFile("my-plugin", "assets/logo.png");

// 列出文件
const files = await pluginListFiles("my-plugin", "assets");

// 删除文件
await pluginDeleteFile("my-plugin", "assets/logo.png");
```

### 用户信息

```typescript
const { pluginUserMe, pluginUserGet, pluginUserList } = window.__MCGUFFIN_SDK__;

// 当前登录用户
const me = await pluginUserMe("my-plugin");

// 指定用户
const user = await pluginUserGet("my-plugin", "user-123");

// 团队成员列表
const { members } = await pluginUserList("my-plugin");
```

### 通知

```typescript
const { pluginCreateNotification } = window.__MCGUFFIN_SDK__;

await pluginCreateNotification(
  "my-plugin",
  "user-123",           // 目标用户 ID
  "标题",
  "通知正文",
  "/plugins/my/page",   // 可选，点击通知跳转的链接
);
```

### React Hooks

SDK 也提供了 React Hooks 封装，适合在组件中直接使用：

```typescript
const {
  usePluginData,
  usePluginCounter,
  usePluginSet,
  usePluginKeys,
  usePluginUserMe,
  usePluginUser,
  usePluginTeamMembers,
} = window.__MCGUFFIN_SDK__;

// 响应式数据读取
const { value, loading, refresh } = usePluginData("my-plugin", "config", "theme");

// 当前用户
const { user } = usePluginUserMe("my-plugin");

// 团队成员（可响应刷新）
const { members, refresh } = usePluginTeamMembers("my-plugin");
```

---

## 插件系统目录结构

主仓库只包含插件系统本身，**不含任何具体插件**：

```
web/src/plugins/
├── index.ts              # 导出 PluginRegistry、definePlugin 等
├── registry.ts           # 插件注册中心（单例，含 zip 插件动态加载）
├── types.ts              # 类型定义
├── PluginPage.tsx         # 插件路由页面容器
└── sdk/
    ├── index.ts           # SDK 公开 API 导出
    ├── definePlugin.ts    # definePlugin() 入口（zip 插件经 window.__MCGUFFIN_SDK__ 调用）
    ├── data.ts            # 数据 API（KV、计数器、文件、用户）
    ├── hooks.ts           # React Hooks 封装
    └── PluginSlots.tsx    # 插槽渲染组件
```

具体插件的文件不在源码树里：zip 包解压到数据目录 `plugins/{插件id}/assets/`，
插件写入的文件在其 `plugins/{插件id}/files/` 下。

## 权限模型

插件权限与主应用的角色权限是**两套独立体系**：

| 场景 | 校验方式 |
|------|----------|
| 插件私有数据接口（`/plugins/{id}/data`、`/files`、`/users`…） | 校验**插件权限**（`permissions_needed`） |
| 插件调用主应用 API（如题目、赛事、团队审批） | 以**当前登录用户**的权限执行，与插件权限无关 |

### 权限清单

| 权限 | 含义 | 蕴含 |
|------|------|------|
| `storage` | 读写插件自己的 KV / 计数器 / 集合 / 文件 | — |
| `notify` | 给用户发通知 | — |
| `read:team` | 读取团队成员列表 | — |
| `write:team` | 审批入队、移除成员 | ⇒ `read:team` |
| `write:team_roles` | 变更成员角色（最敏感） | ⇒ `read:team` |
| `read:problems` / `read:contests` / `read:posts` | 读取题目 / 赛事 / 帖子 | — |
| `read:users` | 读取用户资料（受限字段） | — |
| `read:users:email` | 额外可读用户邮箱 | ⇒ `read:users` |

### 授予与变更

- **首次登记**（ZIP 安装写入清单）、或插件入口页面加载时自注册，按声明自动授予已知权限，未知权限名被静默忽略。
- **权限冻结**：注册接口无需鉴权，因此**重注册不会修改已注册插件的权限**，只刷新名称/版本等元信息（防止有人用同一 id 顶替并扩权）。
- **调整权限**：仅超级管理员，两种方式 ——
  1. 管理后台 → 插件管理 → 「权限」按钮勾选保存；
  2. `PUT /api/v1/admin/plugins/{id}/permissions`，body `{"permissions": ["storage", "read:team"]}`。
- 已禁用插件或全局禁用状态下，所有数据接口一律 403。

---

## 常见问题

**Q: 插件代码如何与主应用隔离？**  
A: 插件的路由页面通过 `React.lazy()` 动态加载，数据存储按 `pluginId + namespace` 隔离，不同插件无法互相访问对方的数据。

**Q: 插件可以使用主应用的组件吗？**  
A: 通过 `window.__MCGUFFIN_SDK__` 可以用到 React、全部数据 API 与 Hooks；主应用的内部组件不在 SDK 暴露范围内，插件 UI 建议自带。

**Q: 插件注册后不显示？**  
A: 检查 `nav_placement` 是否正确（`main` 出现在主导航栏，`admin` 出现在管理后台左侧栏，`hidden` 不显示但路由仍可访问）。若有 `required_permission`，确认当前用户拥有该权限。
