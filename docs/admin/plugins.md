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
2. 选择一个包含 `plugin.json` 的 .zip 包
3. 安装成功后插件出现在列表中

> **注意**：ZIP 安装的后端接口（`POST /api/admin/plugins/install-zip`）当前为预留接口，需后端实现后生效。

---

## 卸载插件

### 代码注册的插件

直接删除 `web/src/plugins/` 下对应目录，重新构建即可。

### ZIP 安装的插件

1. 进入后台 → 插件管理
2. 在插件列表中找到要卸载的插件（标记为「ZIP 安装」）
3. 点击「卸载」按钮确认

卸载会同时删除插件的持久化数据。

---

## 开发插件

### 最小示例

项目内置了一个完整的参考实现：`web/src/plugins/team-members/`，将团队成员页以插件形式重写。

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
| `permissions_needed` | `string[]` | | 插件需要的额外权限（预留） |

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

## 常见问题

**Q: 插件代码如何与主应用隔离？**  
A: 插件的路由页面通过 `React.lazy()` 动态加载，数据存储按 `pluginId + namespace` 隔离，不同插件无法互相访问对方的数据。

**Q: 插件可以使用主应用的组件吗？**  
A: 可以。插件代码运行在主应用的构建上下文中，可以 import 主应用的任何组件或工具函数。但建议尽量自包含以保证可移植性。

**Q: 插件注册后不显示？**  
A: 检查 `nav_placement` 是否正确（`main` 出现在主导航栏，`admin` 出现在管理后台左侧栏，`hidden` 不显示但路由仍可访问）。若有 `required_permission`，确认当前用户拥有该权限。
