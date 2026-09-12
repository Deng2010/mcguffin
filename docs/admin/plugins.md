# 🧩 插件管理

McGuffin 支持通过插件系统扩展前端功能。插件可以添加独立页面、在现有页面插入组件、使用键值存储持久化数据。

## 插件注册方式

插件有两种存在形式：

| 方式 | 适用场景 | 说明 |
|------|----------|------|
| **代码注册** | 开发/定制 | 在 `web/src/plugins/` 下编写 `definePlugin()` 调用，构建时自动发现 |
| **ZIP 安装** | 生产分发 | 上传包含 `plugin.json` 的 .zip 包，后端管理生命周期 |

两种方式注册的插件共用同一个运行时（路由、插槽、数据 API），可并存。

---

## 安装插件

### 方式一：代码注册（开发环境）

1. 在 `web/src/plugins/` 下创建插件目录，例如 `my-plugin/`
2. 创建入口文件 `index.ts`，调用 `definePlugin()`
3. 重新构建前端（`bun run build`），插件会在启动时自动发现

插件目录需满足以下约定之一即可被自动扫描：
- 文件名匹配 `*.plugin.ts`
- 子目录下存在 `index.ts`

### 方式二：ZIP 上传（管理后台）

1. 进入后台 → 插件管理 → 点击「上传 .zip 文件」
2. 选择一个包含 `plugin.json` 的 .zip 包（格式见下文「打包分发插件」）
3. 安装成功后插件立即出现在列表中，前端会**动态加载**其入口模块，无需刷新页面

安装后的插件文件存放在数据目录 `plugins/{插件id}/assets/` 下，通过
`GET /api/plugins/{id}/assets/*` 公开访问（仅静态代码；数据接口仍需权限校验）。

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

`window.__MCGUFFIN_SDK__` 暴露的完整能力：`React`、`definePlugin`、`PluginSlots`，
以及下文「SDK API」列出的全部数据函数与 React Hooks。

### 安全说明

ZIP 插件代码以主应用同等权限运行在浏览器中（可访问 DOM 与已登录用户的会话），
请只安装来源可信的插件。安装接口仅超级管理员可用；后端对 zip 包做了
文件数（512）、单文件（8 MiB）、解压总大小（64 MiB）与路径穿越防护。

---

## 卸载插件

### 代码注册的插件

直接删除 `web/src/plugins/` 下对应目录，重新构建即可。

### ZIP 安装的插件

1. 进入后台 → 插件管理
2. 在插件列表中找到要卸载的插件（标记为「ZIP 安装」）
3. 点击「卸载」按钮确认

卸载会同时删除插件的持久化数据（KV 存储、文件存储与 zip 资产目录）。

---

## 数据持久化

插件系统状态全部持久化，重启不丢失：

| 数据 | 位置 |
|------|------|
| 插件清单（含启用状态、安装来源） | SQLite `plugins` 表 |
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
`plugin.install` / `plugin.uninstall` / `plugin.enable` / `plugin.disable` /
`plugin.set_permissions` / `plugin.global_toggle`，资源标识为 `plugin:{id}`。

---

## 开发插件

### 本地专属插件（不入版本库）

主仓库**不跟踪任何具体插件**，只跟踪插件系统本身（`registry.ts` / `types.ts` /
`PluginPage.tsx` / `index.ts` / `sdk/`）。以下两个插件是部署/本地专属的，已列入
`.gitignore`，**克隆仓库后不会出现**：

| 插件 | 说明 |
|------|------|
| `plugins/lollipop-rank/` | 榜榜糖（站点专属的趣味点糖玩法） |
| `plugins/team-members/` | 团队成员页的插件化实现（参考实现） |

这不会影响构建：插件由 `registry.ts` 的
`import.meta.glob("/src/plugins/*/index.ts")` 在构建期发现，目录不存在时自动跳过。
`bun run build` 与 `bun run test` 在没有这两个插件的干净克隆中均通过。

> 如需自行维护这些插件，请从各自仓库获取后放入 `web/src/plugins/<插件id>/`
> （目录结构见下文），或改以 ZIP 方式安装分发。

### 最小示例

以下是一个完整的参考实现（团队成员页的插件化重写）。
该实现是**本地专属插件**，不在版本库中（见「本地专属插件」），此处保留其源码作为编写参考：

```typescript
// web/src/plugins/team-members/index.ts
import React from "react";
import { definePlugin } from "../sdk";

const plugin = definePlugin(
  {
    id: "team-members",
    name: "团队成员",
    version: "1.0.0",
    description: "团队成员列表、角色管理、入队审批",
    author: "mcguffin",
    routes: [
      {
        path: "/plugins/team",
        label: "团队",
        icon: "👥",
        nav_placement: "main",
        required_permission: "view_team",
      },
    ],
  },
  React.lazy(() => import("./TeamMembersPage")),
);

export default plugin;
```

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

```typescript
import { usePluginId, usePluginContext } from "../sdk";

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

```typescript
import { definePlugin } from "../sdk";

const plugin = definePlugin(
  { /* ...路由定义... */ },
  React.lazy(() => import("./MyPage"))
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

插件通过 `web/src/plugins/sdk` 访问系统能力。所有 API 调用传入的 `pluginId` 自动限定数据访问范围，不同插件之间数据隔离。

### 键值存储

```typescript
import { getPluginData, setPluginData, pluginKeys } from "../sdk";

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
import { pluginIncr, pluginDecr, pluginAdd } from "../sdk";

const newVal = await pluginIncr("my-plugin", "stats", "visits");  // +1
await pluginDecr("my-plugin", "stats", "pending");                 // -1
await pluginAdd("my-plugin", "score", "total", 100);               // +100
```

### 集合

```typescript
import { pluginSetAdd, pluginSetRemove, pluginSetMembers } from "../sdk";

await pluginSetAdd("my-plugin", "groups", "admins", "user-123");
await pluginSetRemove("my-plugin", "groups", "admins", "user-123");
const members = await pluginSetMembers("my-plugin", "groups", "admins");
```

### 文件存储

```typescript
import { pluginWriteFile, pluginReadFile, pluginListFiles, pluginDeleteFile } from "../sdk";

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
import { pluginUserMe, pluginUserGet, pluginUserList } from "../sdk";

// 当前登录用户
const me = await pluginUserMe("my-plugin");

// 指定用户
const user = await pluginUserGet("my-plugin", "user-123");

// 团队成员列表
const { members } = await pluginUserList("my-plugin");
```

### 通知

```typescript
import { pluginCreateNotification } from "../sdk";

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
import {
  usePluginData,
  usePluginCounter,
  usePluginSet,
  usePluginKeys,
  usePluginUserMe,
  usePluginUser,
  usePluginTeamMembers,
} from "../sdk";

// 响应式数据读取
const { value, loading, refresh } = usePluginData("my-plugin", "config", "theme");

// 当前用户
const { user } = usePluginUserMe("my-plugin");

// 团队成员（可响应刷新）
const { members, refresh } = usePluginTeamMembers("my-plugin");
```

---

## 插件目录结构参考

```
web/src/plugins/
├── index.ts              # 导出 PluginRegistry、definePlugin 等
├── registry.ts           # 插件注册中心（单例）
├── types.ts              # 类型定义
├── PluginPage.tsx         # 插件路由页面容器
└── sdk/
    ├── index.ts           # SDK 公开 API 导出
    ├── definePlugin.ts    # definePlugin() 入口
    ├── data.ts            # 数据 API（KV、计数器、文件、用户）
    ├── hooks.ts           # React Hooks 封装
    └── PluginSlots.tsx    # 插槽渲染组件
```

你自己的插件放在 `web/src/plugins/` 下的子目录中，每个插件一个目录。

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

- **首次注册**（代码插件页面加载自注册 / ZIP 安装）时，按声明自动授予已知权限，未知权限名被静默忽略。
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
A: 可以。插件代码运行在主应用的构建上下文中，可以 import 主应用的任何组件或工具函数。但建议尽量自包含以保证可移植性。

**Q: 插件注册后不显示？**  
A: 检查 `nav_placement` 是否正确（`main` 出现在主导航栏，`admin` 出现在管理后台左侧栏，`hidden` 不显示但路由仍可访问）。若有 `required_permission`，确认当前用户拥有该权限。
