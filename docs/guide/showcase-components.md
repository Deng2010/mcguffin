# 展板（Showcase）组件设计文档

> 面向开发者：展板页组件化架构、数据模型、持久化方式与扩展指南。
> 阅读前提：了解前端 `web/src/features/showcase/` 目录与后端 meta 表持久化模式。

---

## 1. 背景与目标

展板页（`/` 首页，`ShowcasePage`）是团队对外展示的入口，原先将「团队简介 /
公告 / 公开题目 / 比赛」四个区块硬编码在一个页面文件里（`ShowcasePage.tsx`，
约 760 行），存在以下问题：

- 四个区块职责混杂，无法单独复用或调整展示方式；
- 「展示什么」与「怎么排布」都写死在代码里，管理员只能粗粒度地勾选题目/比赛；
- 新增一个展示区块需要改动页面主体，成本高、易回归。

本设计将展板重构为**组件化系统**：

- 每个展示区块是一个独立「展板组件」，自带渲染实现与设置 Schema；
- 组件支持**设置（settings）**、**大小（size：宽度 + 最小高度）**、**位置（position：顺序）**；
- 布局作为一份可持久化 JSON，由管理员在线编辑，公开访客读取渲染；
- 新增组件只写前端代码并注册，**后端无需改动**。

---

## 2. 核心概念

| 概念 | 说明 |
| --- | --- |
| 展板组件（Component） | 一个可独立渲染的展示区块：类型 + 设置 + 渲染组件 |
| 布局（Layout） | 展板上全部组件的有序集合（`schema_version` + `components[]`） |
| 组件设置（settings） | 该组件专属的可配置项（如公告条数、选中题目 id 列表） |
| 大小（size） | 宽度 = 栅格列数（1..4），最小高度 = px（0 = 自适应） |
| 位置（position） | 流式顺序 `order`（未来可扩展显式行列） |
| 注册表（registry） | 类型 → 组件定义（label / 默认设置 / 设置字段 Schema / 渲染组件） |
| 组件上下文（ctx） | 页面 → 组件的共享数据契约（题目、比赛、公告、站点信息等） |

---

## 3. 数据模型（前端微信 Schema 权威）

布局 JSON 整体是一个对象，由**前端定义与归一化**，后端只做持久化。

```ts
// web/src/features/showcase/types.ts
export type ShowcaseComponentType =
  | "intro"        // 团队简介
  | "announcements" // 公告
  | "problems"      // 公开题目
  | "contests";     // 比赛

export interface ShowcaseComponentSize {
  width: number;    // 栅格列数 1..4，4 = 整行
  height?: number;  // 最小高度 px；0 / 缺省 = 内容自适应
}

export interface ShowcaseComponentPosition {
  order: number;    // 流式顺序（与 components 数组下标一致）
}

export interface ShowcaseComponentConfig {
  id: string;                       // 实例 id（内置组件 = type）
  type: ShowcaseComponentType;
  enabled: boolean;                 // 是否在展板中展示
  settings: Record<string, unknown>; // 组件设置，结构见 registry
  size: ShowcaseComponentSize;
  position: ShowcaseComponentPosition;
}

export interface ShowcaseLayout {
  schema_version: number;           // 当前 = 1
  components: ShowcaseComponentConfig[];
}
```

### 3.1 组件设置（settings）约定

内置组件的 settings 结构由注册表中的 `defaultSettings()` + `fields` 声明，
读取侧通过 `getXxxSettings(config)` 工具函数取类型化值（缺失/非法回退默认）：

| 组件 | settings 键 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- | --- |
| intro | （无） | — | — | 名称/简介来自站点信息 |
| announcements | `showCount` | number | 3 | 公告展示条数（置顶优先，最少展示置顶数） |
| problems | `selectedIds` | string[] | [] | 选中题目 id；**空数组 = 全部展示** |
| problems | `showDifficulty` | boolean | true | 是否显示难度徽标 |
| contests | `selectedIds` | string[] | [] | 选中比赛 id；空数组 = 全部展示 |
| contests | `showDescription` | boolean | true | 是否显示比赛简介 |
| contests | `showProblems` | boolean | true | 是否显示比赛内嵌题目 |

> `selectedIds` 的「空数组 = 全部展示」与旧版 `showcase_problem_ids` /
> `showcase_contest_ids` 语义一致，迁移时零感知。

### 3.2 布局归一化（normalize）

`registry.ts::normalizeShowcaseLayout(raw, legacyProblemIds, legacyContestIds)`
把任意来源（后端、旧版数据、本地编辑）的布局收拢为合法布局：

1. 按注册表默认值补齐缺失字段；
2. **未知组件类型保留**（向前兼容：未来组件数据不被丢弃，渲染时跳过）；
3. 内置组件缺失时补默认（旧库升级自动获得新组件）；
4. 按 `position.order` 排序并重刷序号 = 数组下标。

---

## 4. 后端存储与 API

后端将布局视为 **opaque JSON**（不解析结构），因此新增组件类型、修改 settings
Schema 都无需改后端。

### 4.1 存储

- SQLite `meta` 表键：`showcase_layout`，值为布局 JSON 的字符串形式；
- 内存：`AppState.showcase_layout: Arc<RwLock<Option<serde_json::Value>>>`；
- 备份/导入导出：`SavedData.showcase_layout`（`serde(default)`，旧备份缺失时为 `None`）。

### 4.2 API

| 接口 | 方法 | 权限 | 说明 |
| --- | --- | --- | --- |
| `/api/v1/site/info`（兼容 `/api/site/info`） | GET | 公开 | 返回 `showcase_layout`（可能为 `null`），公开访客据此渲染 |
| `/api/v1/admin/showcase/layout` | GET | `edit_showcase` | 返回当前布局 |
| `/api/v1/admin/showcase/layout` | PUT | `edit_showcase` | 保存布局（需为 JSON 对象 `{ schema_version, components }`） |
| `/api/v1/admin/showcase`（旧） | GET/PUT | `edit_showcase` | 旧版题目/比赛勾选接口，保留兼容 |

### 4.3 相关后端文件

| 文件 | 改动 |
| --- | --- |
| `server/src/domain/site.rs` | `SiteInfo` 增加 `showcase_layout: Option<Value>` |
| `server/src/domain/admin.rs` | 新增 `ShowcaseLayoutPayload { layout: Value }` |
| `server/src/state.rs` | `AppState` 增加 `showcase_layout` |
| `server/src/db.rs` | `import_meta_fields` / `load_all_from_sqlite` 读写 meta 键 |
| `server/src/infra/persistence.rs` | `SavedData` 字段 + 加载/重建/`reload()` 同步 |
| `server/src/handlers/info.rs` | `get_site_info` 返回布局 |
| `server/src/handlers/admin/showcase.rs` | 新增 `get/update_showcase_layout` |
| `server/src/routes.rs` | 注册 `/admin/showcase/layout`（v1 与兼容层自动双挂） |

---

## 5. 前端架构

```
web/src/features/showcase/
├── ShowcasePage.tsx           # 页面：数据加载 + 布局状态 + 管理入口（薄）
├── types.ts                   # 数据模型类型 + 组件上下文契约
├── registry.ts                # 组件注册表 + 布局构建/归一化 + 设置读取工具
├── ShowcaseBoard.tsx          # 栅格渲染层（宽/高/顺序 → CSS Grid）
├── ShowcaseSettingsPanel.tsx  # 管理面板（Schema 驱动的设置编辑器）
└── components/
    ├── IntroComponent.tsx         # 团队简介
    ├── AnnouncementsComponent.tsx # 公告
    ├── ProblemsComponent.tsx      # 公开题目
    ├── ContestsComponent.tsx      # 比赛
    └── ProblemCard.tsx            # 共享题目卡片（大卡 / 紧凑卡）
```

### 5.1 组件契约

所有组件通过统一 Props 接收数据，不直接依赖页面：

```ts
export interface ShowcaseComponentProps {
  config: ShowcaseComponentConfig; // 本组件的设置 / 大小 / 位置
  ctx: ShowcaseContext;            // 共享数据
}

export interface ShowcaseContext {
  siteInfo: ShowcaseSiteInfo | null; // 站点名称/简介/时区（最小子集）
  announcements: Announcement[];     // 公告
  problems: ShowcaseProblem[];       // 已发布题目
  contests: ShowcaseContest[];       // 公开比赛
  difficultyMap: Map<string, DifficultyInfo>;
  isAdmin: boolean;                  // edit_showcase
  canAccessAdmin: boolean;
  getServerNow: () => number | null;
}
```

> 约定：组件从 `ctx` 取数据，从 `config.settings` 取自身配置；需要站点级操作
> （如编辑简介）时可自行调用 zustand store（如 `useSiteStore`）。

### 5.2 栅格渲染（ShowcaseBoard）

- 栅格：`grid grid-cols-1 lg:grid-cols-4 gap-6`（宽屏 4 列，窄屏自动单列堆叠）；
- 宽度：`size.width`(1..4) → `lg:col-span-N`（Tailwind 需要静态类名，见
  `ShowcaseBoard.tsx` 中的 `COL_SPAN_CLASS` 映射表，**新增列数需同步该表**）；
- 最小高度：`size.height`(px) → 外容器 `style.minHeight`；
- 顺序：按 `position.order` 升序渲染，`enabled === false` 跳过；
- 未知类型：跳过渲染（数据保留）。

### 5.3 管理面板（ShowcaseSettingsPanel）

`edit_showcase` 权限可见的「展板管理」面板，编辑即时反映到下方展板（实时预览）：

- 组件开关（enabled）；
- 位置调整（↑↓ 交换顺序，自动重刷 `order`）；
- 大小：宽度下拉（1/4、1/2、3/4、整行）+ 最小高度数字输入（0 = 自适应）；
- 组件设置：由注册表 `fields` Schema **自动生成表单**：
  - `number` → 数字输入；
  - `boolean` → 开关；
  - `ids` → 候选项多选列表（选自 `ctx.problems` / `ctx.contests`，
    选中项居前可 ↑↓ 排序，序即展示序）。
- 保存 → `PUT /admin/showcase/layout` → 刷新站点信息。

---

## 6. 内置组件一览

| 类型 | 定义 | 渲染要点 |
| --- | --- | --- |
| `intro` | 团队简介 | 站点名称 + Markdown 简介；管理员可内联编辑 |
| `announcements` | 公告 | 置顶优先、时间倒序，`showCount` 条；空态含管理引导 |
| `problems` | 公开题目 | 已发布题目；`selectedIds` 筛选、`showDifficulty` 难度徽标；external link 优先外跳 |
| `contests` | 比赛 | 公开比赛；状态徽标（进行中/未开始/已结束）、比赛简介、按 `problem_order` 排序的内嵌题目 |

---

## 7. 迁移与兼容

- 新装：`showcase_layout` 为 `null`，前端 `createDefaultLayout([], [])` 生成默认布局
  （四组件整行、题目/比赛全部展示）。
- 已有部署：`siteInfo.showcase_layout` 为 `null` 时，前端用旧版
  `showcase_problem_ids` / `showcase_contest_ids` 作为题目/比赛组件
  `selectedIds` 默认值生成布局——**首次保存前展示效果与旧版完全一致**。
- 保存后 `showcase_layout` 成为唯一事实源；旧字段保留（API 兼容），前端不再写入。
- Schema 演进：修改 `schema_version` 并在 `normalize` 中做版本分支
  （不同 version 走不同归一化逻辑），后端无感。

---

## 8. 如何新增一个展板组件

以新增「成员风采」组件为例（假设复用团队数据）：

1. **扩展类型**：在 `types.ts` 的 `ShowcaseComponentType` 联合类型中加 `"members"`。
2. **实现展示组件**：新建 `components/MembersComponent.tsx`，实现
   `ShowcaseComponentProps` 契约（从 `ctx` 取数据、从 `config.settings` 取配置，
   根元素建议 `h-full` 以配合等高/最小高度）。
3. **注册**：在 `registry.ts::SHOWCASE_COMPONENT_DEFS` 增加一条：

   ```ts
   members: {
     type: "members",
     label: "成员风采",
     description: "团队核心成员展示",
     defaultSettings: () => ({ showCount: 8 }),
     fields: [
       { kind: "number", key: "showCount", label: "展示人数", min: 1, max: 50, step: 1 },
     ],
     component: MembersComponent,
   },
   ```

4. **（可选）设置读取工具**：若设置是类型化结构，在 `registry.ts` 底部加
   `getMembersSettings(config)`。
5. 完成。**渲染（Board）、编辑（SettingsPanel）、持久化（后端）自动生效**，
   无需改动后端或其他文件。

> 若需要组件取到**新数据**（如团队成员接口），将数据在 `ShowcasePage` 拉取后
> 注入 `ShowcaseContext` 即可，组件本身保持无数据获取逻辑（或自行调用 store）。

### 8.1 扩展设置字段类型

在 `registry.ts::SettingField` 联合类型中新增一种 `kind`，并在
`ShowcaseSettingsPanel.renderField` 中实现对应控件，即可让所有组件复用。

---

## 9. 未来演进建议（暂未实现）

- **显式行列**：`position` 增加可选 `row` / `col`，栅格改用网格线命名区域，
  支持“左图右文”等排版；`order` 仍作流式回退。
- **拖拽排布**：面板内用 HTML5 Drag & Drop 或 dnd-kit 替换 ↑↓ 按钮。
- **多实例组件**：同一类型可添加多个实例（`id` 独立），如两个题目组件
  （“热门题目” + “最新题目”）不同设置并存。
- **插件化组件**：接入前端插件系统（`plugins/`），第三方组件经
  `definePlugin` 注册进 `SHOWCASE_COMPONENT_DEFS`，后端仍零改动。
- **每实例标题**：settings 提供 `title` 覆盖默认标题文案。