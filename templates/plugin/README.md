# McGuffin 插件模板

可直接复制成**独立插件仓库**的最小骨架：用 Vite 把 `src/` 打包成单文件 ESM，连同
`plugin.json` 打成 `dist/<id>.zip`，在 McGuffin 管理后台「上传 .zip 文件」或「从 URL 安装」即可。

插件入口不 import 主应用任何代码，只通过宿主注入的全局 SDK 自注册：

```js
const { definePlugin, React } = window.__MCGUFFIN_SDK__;
```

> 打包契约（zip 结构 / plugin.json / 入口 ESM）见主仓库 `docs/admin/plugins.md`；
> 完整开发指南见主仓库 `docs/guide/plugin-development.md`。

## 目录结构

```
.
├── plugin.json                 # 插件清单（id / name / version / permissions_needed / entry）
├── package.json                # 构建脚本与开发依赖（不改主仓库依赖）
├── tsconfig.json               # strict + jsx: react-jsx
├── vite.config.ts              # lib 模式，产物 dist/index.js，React 不打包
├── src/
│   ├── index.tsx               # 入口：definePlugin() 自注册路由（zip 内的 index.js）
│   └── Page.tsx                # 示例页面（演示 usePluginData + setPluginData）
├── types/
│   ├── mcguffin-plugin-sdk.d.ts # SDK 类型（从主仓库同步，勿手改）
│   └── global.d.ts              # window.__MCGUFFIN_SDK__ 全局声明
├── scripts/build-zip.mjs       # 构建 + 打包 zip
└── .github/workflows/release.yml # tag 触发：构建 zip 并上传为 release 资产
```

## 环境要求

- Node.js **>= 24**（见 `package.json` 的 `engines`）
- 包管理器任意（npm / bun / pnpm 均可）

## 快速开始

```bash
# 1. 复制本目录到你的新仓库（目录名随意），然后改这几处：
#    - plugin.json：id / name / description / author / permissions_needed
#    - src/index.tsx：PLUGIN_ID、name、version、routes
#    - package.json：name（仅本地用）
#    ⚠️ id 一旦发布就不要再改：插件数据按 id 隔离，改 id 等于换了一个新插件。
#       同时 id 决定 zip 文件名（dist/<id>.zip）。

# 2. 安装依赖
npm install

# 3. 构建 + 打包
npm run zip     # → dist/my-plugin.zip

# 4. 安装
#    管理后台 → 插件管理 → 「上传 .zip 文件」
```

安装后插件会**立即生效**：前端动态 `import()` 入口 ESM，无需重启或刷新。

## 本地开发流程

| 命令 | 作用 |
|------|------|
| `npm run dev` | `vite build --watch`：改代码即增量构建到 `dist/` |
| `npm run build` | `tsc --noEmit` 类型检查 + `vite build` |
| `npm run zip` | 上面两步 + 把 `plugin.json` 与 `dist/` 全部文件打成 `dist/<id>.zip` |

开发调试的推荐循环：

1. `npm run dev` 让构建常驻；
2. 需要看效果时执行 `npm run zip`，后台上传新 zip —— **同 id 会自动按「更新」处理**，
   插件数据、启用状态与已授权权限都会保留；
3. 入口加载失败时，插件列表会显示「加载失败」标记（悬停看原因），
   浏览器控制台也可按 `[plugin]` 前缀过滤日志。

> 想少点手工操作？把 zip 传到 GitHub Release，用后台的「从 URL 安装」，
> 之后每次发版只需点「从原 URL 更新」（见下文）。

## 构建与打包

`npm run zip` 会：

1. `tsc --noEmit`（`strict` 模式类型检查）；
2. `vite build` → `dist/index.js`（入口）及其它 chunk；
3. 用 `adm-zip` 把 `plugin.json` 与 `dist/` 下的**全部**文件打包成 `dist/<id>.zip`。

zip 内布局（后端按名精确读取，`plugin.json` 必须在**根目录**）：

```
my-plugin.zip
├── plugin.json        # 必需，必须在根目录
├── index.js           # 入口（plugin.json 的 entry，默认 index.js）
└── Page-<hash>.js     # 动态 import 的页面 chunk（同目录，相对路径引用）
```

`plugin.json` 字段（与 `docs/admin/plugins.md` 的契约一致）：

| 字段 | 必需 | 说明 |
|------|------|------|
| `id` | ✅ | 唯一标识：字母或数字开头，仅含字母数字、`-`、`_`，长度 ≤ 64 |
| `name` | ✅ | 显示名称 |
| `version` | ✅ | 语义化版本 |
| `description` | | 描述 |
| `author` | | 作者 |
| `permissions_needed` | | 插件权限数组，只接受已知权限名（未知名会被忽略） |
| `entry` | | zip 内入口相对路径，默认 `index.js` |

### 关于 React 与 JSX（开箱可用）

宿主（McGuffin 主应用）在 `index.html` 里提供了 import map，把
`react` / `react/jsx-runtime` / `react/jsx-dev-runtime` / `react-dom` 映射到
`/plugin-sdk/*.js` 垫片，垫片再转发到宿主暴露的 `window.__MCGUFFIN_SDK__`
（**同一份** React 实例）。因此插件里可以直接写：

```tsx
import { useState } from "react";        // 解析到宿主 React，不会打包第二份

export default function Page() {
  const [n, setN] = useState(0);
  return <button onClick={() => setN(n + 1)}>点了 {n} 次</button>;   // JSX 直接用
}
```

`vite.config.ts` 相应地把这几个模块声明为 `external`（**不要**把它们打进产物，
否则会出现多份 React 实例，hooks 与 context 全部失效），产物里的裸模块名交给
宿主的 import map 解析。

插件的 SDK 能力（KV / 用户 / 团队 / 通知…）仍从全局取：

```ts
const { usePluginData, setPluginData } = window.__MCGUFFIN_SDK__;
```

需要 ReactDOM（例如 `createPortal` 做弹窗）时 `import { createPortal } from "react-dom"`
也可以；但 `react-dom/client`（createRoot）没有映射 —— 插件页面由宿主挂载，
不要自己创建 React 根。

> ⚠️ import map 只覆盖上面这 4 个名字。第三方库要么保持默认打包（不要加进
> `rollupOptions.external`），要么改相对路径导入；`npm run zip` 会在产物里出现
> 未映射的裸模块名时给出警告。

### 样式

插件的 UI 跑在宿主页面里，但宿主的 Tailwind 构建**扫描不到**插件源码，
所以 `className="p-4"` 这类工具类不会生效。建议用内联样式或自带 CSS。

自带样式表：`import "./style.css"` 会在 `dist/` 产出一个 CSS 文件
（默认文件名取自 `package.json` 的 `name`，可用 `build.lib.cssFileName` 改名），
它会被一起打进 zip；但 lib 模式**不会自动注入**，需要自己插入 `<link>`。

## 安装到 McGuffin

### 方式一：后台上传 zip

管理后台 → 插件管理 → 「上传 .zip 文件」→ 选择 `dist/<id>.zip`。
仅超级管理员可安装；若该 id 已存在，则按「更新」处理。

### 方式二：从 URL 安装（推荐，便于持续更新）

把 zip 传到任意 https 地址，管理后台 → 插件管理 → 「从 URL 安装」→ 填入 URL。
GitHub Release 会给出稳定地址：

```
https://github.com/<owner>/<repo>/releases/latest/download/<id>.zip
```

`latest/download` 永远指向最新一版的资产，因此这个 URL 可以长期不变，
配合后台的「从原 URL 更新」即可一键升级。

## 更新流程

后台对已安装插件提供两种更新方式：

- **「更新」**：上传新的 zip（包内 `plugin.json` 的 id 必须与插件 id 一致）；
- **「从原 URL 更新」**：重新下载安装时记录的来源 URL（仅对「从 URL 安装」的插件可用）。

更新语义（后端保证）：

| 项目 | 更新时 |
|------|--------|
| 插件 KV 数据（`plugin_data`）与文件存储 | **保留** |
| 启用 / 禁用状态 | **保留** |
| 已授权权限 | **保留（冻结）**，不会因新版本声明的权限自动扩权 |
| 静态资源（`assets/`） | 整体替换为新版本 |
| 新版本**额外申请**的权限 | 不会自动授予，需管理员在后台插件页勾选确认后生效 |

所以升级时请留意：如果新版本新增了 `permissions_needed`，装完要在后台补勾，
否则相关数据接口会返回 403。

## 发布（tag → Release）

`.github/workflows/release.yml` 已配置好：推送 `v*` tag 时自动
安装依赖 → `npm run zip` → 把 `dist/*.zip` 作为 release 资产上传。

```bash
git tag v1.0.0
git push origin v1.0.0
```

资产名固定为 `<id>.zip`，正好对应上面「从 URL 安装」的稳定地址。

## SDK 类型同步

`types/mcguffin-plugin-sdk.d.ts` 是主仓库
`web/src/plugins/sdk/plugin-sdk.d.ts` 的**逐字节副本**（主仓库有测试强制校验两份一致）。
宿主 SDK 有更新时，从主仓库同步：

```bash
cp <mcguffin>/web/src/plugins/sdk/plugin-sdk.d.ts types/mcguffin-plugin-sdk.d.ts
```

不要手改这个文件——改了会在主仓库的测试里暴露为「类型漂移」。

## 权限一览

`plugin.json` 的 `permissions_needed` 只接受下列已知权限（未知名会被静默忽略）：

| 权限 | 用途 |
|------|------|
| `storage` | 插件私有 KV / 计数器 / 集合 / 文件存储 |
| `notify` | 给用户发站内通知 |
| `read:team` | 读取团队成员列表 |
| `write:team` | 审批入队、移除成员（蕴含 `read:team`） |
| `write:team_roles` | 变更成员角色（最敏感，蕴含 `read:team`） |
| `read:users` | 读取用户资料（受限字段） |
| `read:users:email` | 额外可读用户邮箱（蕴含 `read:users`） |
| `read:problems` / `read:contests` / `read:posts` | 读取题目 / 赛事 / 帖子 |

## 常见问题

**Q：改动 `src/index.tsx` 里的 routes 不生效？**
A：路由是入口注册时写入注册表的，重新上传 zip（或让宿主重新加载插件）后生效。

**Q：多次注册会重复添加路由 / 插槽吗？**
A：不会，注册表按插件 id 覆盖。

**Q：`window.__MCGUFFIN_SDK__` 是 undefined？**
A：插件入口只在 McGuffin 页面内运行；直接打开 `index.js` 或在别的页面里跑当然没有。

**Q：能直接用主应用的组件（按钮、弹窗等）吗？**
A：不能，SDK 只暴露 React、数据 API 与 Hooks，插件 UI 请自带。
