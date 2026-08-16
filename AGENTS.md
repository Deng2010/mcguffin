# AGENTS.md — McGuffin AI Agent 指南

> 面向 AI 编程助手（Claude Code、OpenCode、Copilot、Cline 等），提供代码库结构与开发约定。阅读本文后再动手修改代码。

---

## 项目概览

**McGuffin** 是算法竞赛出题团队的协作工具。React 18 SPA + Rust/Axum 后端，CP OAuth 认证，SQLite 持久化，带前端插件系统与错误上报。

- 前/后端版本号必须同步（当前 `0.3.1`）
- 架构：浏览器 → React SPA → Axum API（`/api/v1/` + 兼容层 `/api/`）→ SQLite / CP OAuth
- 后端为 **分层架构**：`domain`（数据+领域逻辑）→ `handlers`（HTTP 层）→ `infra`（持久化/配置/备份），路由统一在 `routes.rs` 注册。
- 前端为 **特性分层**：`app`（路由+布局）→ `features`（按领域分组的页面）→ `services`（API 封装）→ `stores`（zustand 状态）→ `plugins`（插件系统）。

---

## ⚠️ 首要约定：保持 AGENTS.md 与代码同步

**如果本文件（AGENTS.md）与实际项目结构/约定不再一致，必须及时更新 AGENTS.md。**

- 以下任一改动完成后，都要同步更新本文档：
  - 新增/删除/重命名源码模块、目录或构建命令
  - 权限常量（`server/src/domain/permission.rs` 的 `perms` 模块）增删改
  - 数据模型、持久化方式、配置项或 API 约定变化
  - 前端目录结构、构建脚本、环境要求（如 Node 版本）变化
  - 新增或废弃某个命令行工具 / just 命令 / 环境变量
- 更新时以**代码为唯一事实来源**，不要凭记忆猜测；先查 `.md` 对应的实际源码与配置再改。
- 若发现 AGENTS.md 已过时但仍要继续工作，先修正文档（或至少标注 TODO），避免误导后续的 agent 与开发者。

---

## 构建与运行

### 前端（`web/`）

```bash
cd web
bun install           # 安装依赖（bun）
bun run dev           # 开发服务器 :5173（代理 /api → :3000）
bun run build         # 生产构建 → dist/（先 tsc --noEmit 再 vite build）
bun run test          # Vitest 测试（单次）
bun run test:watch    # Vitest 监听模式
bun run tsc --noEmit  # 仅类型检查
```

> 环境要求：**Node.js >= 24**（见 `package.json` 的 `engines`）。包管理器用 **bun**。

### 后端（`server/`）

```bash
cd server
cargo check               # 类型检查（快）
cargo test                # 运行单元 + 集成测试
cargo clippy --bins -- -D warnings  # lint（warning 即错误）
cargo fmt                 # 格式化
cargo build --release     # 生产构建 → target/release/mcguffin-server + mcguffin
cargo run                 # 开发服务器 :3000
cargo run --bin mcguffin  # CLI 工具
```

### 全量（项目根，`just`）

```bash
just build         # 构建全部（前端 + 后端 + CLI）
just dev           # 并行启动前后端开发服务器
just check         # check-backend + check-frontend（cargo check+clippy+tsc）
just test          # test-backend + test-frontend（cargo test + vitest）
just fmt / fmt-fix # Rust 格式化检查 / 修复
just fast-deploy   # 快速部署（debug 编译 5-15s + 重启服务）
sudo just install  # 安装到 /usr/local
just install-user  # 安装到 ~/.local（无需 root）
just version       # 查看前后端版本
just clean         # 清理所有构建产物
```

> 完整 just 命令清单见 `justfile` 的 `default` 帮助。常用分组：`build*` / `dev*` / `check*` / `test*` / `install*` / `docker*` / `clean*` / `init-config` / `dist` / `version`。

---

## 项目结构

```
mcguffin/
├── docs/                     # 文档（部署/管理/使用）
│   ├── README.md             # 文档索引
│   ├── guide/                # 部署与开发指南（quick-start/development/configuration/deployment）
│   ├── admin/                # 管理后台手册（overview/users/contests/backups/plugins）
│   ├── user/                 # 用户手册（getting-started/problems/community）
│   └── images/               # 文档图片
├── justfile                  # 构建/部署/测试命令（just）
├── AGENTS.md                 # 本文件
├── README.md                 # 项目主页
├── Containerfile / Dockerfile# Docker 构建（多架构 amd64 + arm64）
├── .dockerignore             # Docker 构建上下文排除
├── docker-compose.yml        # Docker Compose
├── docker-entrypoint.sh      # 容器入口
├── web/                      # 前端 SPA（React 18 + Vite + TS + Tailwind + zustand）
│   ├── index.html
│   ├── package.json          # 依赖与脚本（bun）
│   ├── tsconfig.json / tsconfig.node.json
│   ├── vite.config.ts / vitest.config.ts
│   ├── tailwind.config.js / postcss.config.js
│   └── src/
│       ├── main.tsx          # 入口
│       ├── App.tsx           # 根组件：Provider 组装 + 路由挂载
│       ├── index.css         # Tailwind + 自定义样式
│       ├── types.ts          # TS 类型 + Permission 联合类型 + defaultRolePermissions 回退映射
│       ├── app/
│       │   ├── layouts/      # MainLayout / AdminLayout
│       │   └── routes.tsx    # 路由表 + 路由守卫
│       ├── features/         # 按领域划分的页面（见下方列表）
│       ├── services/         # API 客户端分层封装（每领域一个 *.service.ts）
│       ├── stores/           # zustand 全局状态（authStore / siteStore / themeStore）
│       ├── components/       # 通用组件 + ui/ 基础组件库
│       ├── hooks/            # 自定义 Hooks（useDifficulties / useMention）
│       ├── utils/            # 工具函数（groups / time）
│       ├── errors/           # 错误边界 + 错误标准化 + 上报 + Toast
│       ├── plugins/          # 前端插件系统（含 SDK + 内置插件）
│       └── test/             # Vitest 测试（含 setup.ts）
├── server/                   # 后端（Rust / Axum / sqlx / rusqlite）
│   ├── Cargo.toml            # 依赖与二进制定义
│   ├── migrations/           # SQLite 迁移（sqlx migrate）
│   ├── src/
│   │   ├── main.rs           # 入口：启动服务器
│   │   ├── lib.rs            # 模块导出 + build_router + configured_port
│   │   ├── routes.rs         # 全部路由注册（/api/v1/ 与 /api/ 兼容层）+ 中间件
│   │   ├── state.rs          # AppState + 请求级状态 + 配置路径解析 + ADMIN_USER_ID
│   │   ├── types.rs          # 纯 re-export：pub use crate::domain::*（勿在此新增类型）
│   │   ├── error.rs          # ErrorCode 枚举 + 统一错误响应 + error 中间件
│   │   ├── db.rs             # SQLite 连接 + 数据导入/导出/备份
│   │   ├── utils.rs          # 认证工具函数
│   │   ├── domain/           # 数据模型 + 领域逻辑（核心业务，无 HTTP 依赖）
│   │   │   ├── mod.rs        # 模块声明与 re-export
│   │   │   ├── user.rs / team.rs / problem.rs / contest.rs
│   │   │   ├── post.rs       # 统一帖子（讨论/建议/公告）
│   │   │   ├── notification.rs / site.rs / admin.rs
│   │   │   ├── oauth.rs      # OAuth 领域逻辑
│   │   │   ├── config.rs     # 配置结构体
│   │   │   ├── plugin.rs     # 插件领域模型
│   │   │   └── permission.rs # perms 权限常量 + 默认角色权限映射（权限唯一权威）
│   │   ├── handlers/         # HTTP handler（薄层：鉴权 → 调 domain → 返回 JSON）
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs / user.rs / team.rs / problem.rs / contest.rs
│   │   │   ├── post.rs / notification.rs / info.rs / plugin.rs / pages.rs / errors.rs
│   │   │   └── admin/        # 管理后台子模块（users/groups/acl/backup/config/export/audit/showcase/mod）
│   │   ├── infra/            # 基础设施：persistence / backup / config
│   │   └── bin/mcguffin.rs   # CLI 工具（init/config/backup/service）
│   └── tests/api.rs          # 集成测试
└── .github/workflows/
    ├── test.yml              # PR/Push 测试
    └── docker.yml            # Docker 多架构构建
```

### 前端 features 目录（页面一览）

| 目录 | 页面 |
| --- | --- |
| `features/auth/` | LoginPage、AuthCallbackPage |
| `features/community/` | CommunityPage、PostDetailPage |
| `features/contests/` | ContestDetailPage、ContestManagePage |
| `features/problems/` | ProblemsPage、ProblemDetailPage |
| `features/profile/` | ProfilePage |
| `features/showcase/` | ShowcasePage |
| `features/team/` | TeamPage、ApplyPage |
| `features/notfound/` | NotFoundPage |
| `features/admin/` | AdminUsers/Groups/Roles/Config/Backups/Discussions/Errors/Plugins/Init 页 + `sections/`（配置分区）+ `config-context.ts` |

---

## 权限体系

### 校验方式

两级校验：后端计算 `effective_role`（综合 `role` + `team_status` + 用户自定义权限 + 权限组），前端通过 `GET /api/auth/permissions` 获取角色→权限映射。前端 `types.ts` 中的 `defaultRolePermissions` 仅在前后端不同步时作为回退。

> ⚠️ **权限检查必须用 `effective_role` 而非 `role`**（涉及团队状态覆盖与用户级权限覆盖）。

### 角色层级

`superadmin`（id=`admin`，不可删除/降级）> `admin` > `member` > `guest` > `pending`

超级管理员拥有通配符 `*`（`PERM_WILDCARD`），表示全部权限。

### 权限常量（权威定义在 `server/src/domain/permission.rs::perms`）

| 常量 | 权限字符串 | 说明 |
| --- | --- | --- |
| `VIEW_SHOWCASE` | `view_showcase` | 查看成果展示（公开） |
| `APPLY_JOIN` | `apply_join` | 申请加入团队 |
| `VIEW_TEAM` | `view_team` | 查看团队成员 |
| `MANAGE_TEAM` | `manage_team` | 审核入队申请 |
| `MANAGE_MEMBERS` | `manage_members` | 踢出成员/变更角色 |
| `SUBMIT_PROBLEM` | `submit_problem` | 投稿题目 |
| `VIEW_PENDING_PROBLEMS` | `view_pending_problems` | 查看待审核题目 |
| `VIEW_APPROVED_PROBLEMS` | `view_approved_problems` | 查看已通过题目 |
| `VIEW_PUBLIC_PROBLEMS` | `view_public_problems` | 查看公开题目 |
| `APPROVE_ALL_PROBLEMS` | `approve_all_problems` | 审核/批准题目 |
| `MANAGE_ALL_CONTESTS` | `manage_all_contests` | 创建/编辑/删除/切换赛事可见性 |
| `VIEW_ALL_CONTESTS` | `view_all_contests` | 查看全部赛事（含 draft） |
| `VIEW_PUBLIC_CONTESTS` | `view_public_contests` | 查看公开赛事 |
| `ACCESS_ADMIN` | `access_admin` | 进入管理后台 |
| `EDIT_SHOWCASE` | `edit_showcase` | 编辑站点介绍与展示选中项 |
| `VIEW_ALL_POSTS` | `view_all_posts` | 查看全部帖子（讨论/建议/公告） |
| `MANAGE_TAGS` | `manage_tags` | 管理讨论标签与 emoji |
| `MANAGE_NOTIFICATIONS` | `manage_notifications` | 发送全局通知 |
| `MANAGE_BACKUPS` | `manage_backups` | 备份/恢复数据 |
| `VIEW_STATS` | `view_stats` | 查看统计 |
| `MANAGE_POSTS` | `manage_posts` | 管理统一帖子（取代已废弃的 `manage_discussions`） |

> **新增/删除权限的步骤**：
> 1. 在 `server/src/domain/permission.rs` 的 `perms` 模块新增常量，并加入 `perms::ALL` 数组。
> 2. 在 `default_role_permissions()` 中按角色分配。
> 3. 同步修改前端 `web/src/types.ts` 的 `Permission` 联合类型与 `defaultRolePermissions`（前后端权限名必须一致）。
> 4. （如适用）在对应的 handler 中加权限校验。
> `permission.rs` 内置测试 `test_all_role_permissions_are_in_known_set` 会捕获忘记加入 `perms::ALL` 的权限。

### 资源级 ACL

除全局角色权限外，系统支持**资源级 ACL**（如题目 `Problem` 的 per-problem ACL、`set_resource_acl` / `set_problem_acl`）。涉及「谁能看/谁能改某个具体资源」的判断时，需同时考虑全局权限与资源 ACL，不要只看 `effective_role`。

---

## 关键设计模式

### 后端

- **分层结构**：`domain`（业务逻辑，不含 axum 依赖）→ `handlers`（薄 HTTP 层）→ `infra`（持久化/配置/备份）。业务规则下沉到 `domain`，handler 只做「取状态 → 鉴权 → 调 domain → 包装成 JSON」。
- **路由注册**：全部集中在 `src/routes.rs` 的 `build_router()`。API 挂在 `/api/v1/`（canonical）与 `/api/`（向后兼容）；旧的讨论/建议/公告路径保留在 `/api/` 下。
- **API 模式**：handler 取 `State<AppState>` + `HeaderMap`，调 `resolve_user()` 鉴权，内联权限检查，返回 `Json(serde_json::json!(...))`。鉴权不作为全局中间件。
- **错误处理**：`src/error.rs` 的 `ErrorCode` 枚举集中注册错误码（模块级粒度约 40 个），提供 `json_error` / `http_error` 生成统一错误响应 `{ error, message, hint, suggestion }`；全局中间件注入 `request_id` 并统一打日志；404/405/panic 全部兜底为统一 JSON。
- **持久化**：启动时从 SQLite 读入内存（`HashMap`），运行期操作内存，定时写回 SQLite。相关逻辑在 `db.rs` / `infra/persistence.rs`（`save_all_to_db()` / `reload_all_from_db()`）。迁移文件在 `migrations/`，用 sqlx migrate 管理。
- **配置**：TOML 格式，平台感知路径（见 `state.rs::resolve_config_path`）：优先 `MCGUFFIN_DATA_DIR` 环境变量，其次 CWD 的 `mcguffin.toml`/`config.toml`，最后平台默认路径（Linux `/usr/share/mcguffin/config.toml`）。运行时用 `toml_edit` 编辑。
- **CLI**：`src/bin/mcguffin.rs`，子命令 `init`、`config`、`backup`、`service start/stop/restart/status`。
- **Superadmin 保护**：`ADMIN_USER_ID = "admin"` 硬编码（`state.rs`），不可降级/删除；仅 superadmin 可操作其他 admin。
- **统一帖子系统**：`Post` 结构体（`domain/post.rs`）替代独立的讨论/建议/公告表；标签、emoji、反应、回复、可见性 ACL 都挂在 `Post` 上。
- **日志**：用 `tracing`，不用 `println!`。

### 前端

- **路由**：集中在 `src/app/routes.tsx`；布局在 `src/app/layouts/`。新增页面在 `routes.tsx` 注册，页面本体放 `features/<领域>/`。
- **状态管理**：全局状态用 **zustand**（`src/stores/`：`authStore` / `siteStore` / `themeStore`）。不再使用大量 React Context。
- **API 调用**：统一走 `src/services/*.service.ts`（每个领域一个 service），底层在 `services/api.ts`。不要在组件里直接 `fetch`。
- **权限控制**：路由级用 `ProtectedRoute`（组件级），条件渲染用 `hasPermission()`。权限类型在 `types.ts`。
- **插件系统**：前端有插件框架，见下方「插件系统」小节。
- **错误边界**：`src/errors/` 提供 `ErrorBoundary` + 错误标准化（`normalize.ts`）+ 上报（`reporter.ts`）+ `ToastContext`。

### 技术栈速查

- 后端：axum 0.8、sqlx 0.8（runtime-tokio + sqlite + migrate）、rusqlite 0.32（bundled）、tower-http 0.6（cors/fs/compression/request-id/catch-panic）、chrono、uuid v4、clap 4、reqwest 0.13、toml_edit、tracing。
- 前端：React 18、Vite 6、TypeScript ~5.6、Tailwind 3、zustand 5、react-router-dom 6、react-markdown + remark/rehype（GFM、KaTeX、Prism）、Vitest 4 + Testing Library。

---

## 插件系统

McGuffin 支持**前端插件**，动态扩展页面与能力。

- 插件注册表：`web/src/plugins/registry.ts`，类型在 `plugins/types.ts`，SDK 在 `plugins/sdk/`（`definePlugin` / `PluginSlots` / `hooks` / `data`）。
- 内置插件：`plugins/lollipop-rank/`（趣味排行榜）、`plugins/team-members/`（团队成员展示）。
- 插件通过 `definePlugin()` 定义元信息、声明所需权限、挂接 UI 插槽与数据 hooks；后端 `/api/v1/plugins/*` 提供插件注册、启用/禁用、全局状态与数据存储（见 `handlers/plugin.rs`、`domain/plugin.rs`）。
- 插件可声明**任意字符串权限**（对应前端 `Permission` 联合类型中的 `(string & {})` 分支）。

---

## 错误上报系统

- 前端：`src/errors/` 的 `reporter.ts` + `normalize.ts`，`ErrorBoundary` 捕获渲染错误并上报，`ToastContext` 展示非致命错误。
- 后端：`handlers/errors.rs` 提供 `report_error` / `list_errors` / `update_error_status` / `delete_error` / `clear_errors`；管理后台 `AdminErrorsPage` 查看与处理。
- 迁移文件：`migrations/20260804_add_error_reports.sql`。

---

## 题目验题（verifiers）

- 迁移文件 `migrations/20260816000001_add_verifiers.sql` 引入「验题人」概念，支持下钻多验题人流程。
- 相关 handler：`submit_verifier_comment` / `submit_verifier_solution` / `claim_problem` / `unclaim_problem` / `resubmit_problem` / `review_problem`（见 `handlers/problem.rs`）。
- 题目状态含「退回」状态（见最新提交「题目退回状态、多验题人与资源权限死锁修复」）。

---

## 修改代码注意事项（速查）

### 后端

- 数据模型 → `domain/`（**不要**在 `types.rs` 里新增，它只是 re-export）。
- HTTP handler → `handlers/`（管理后台相关进 `handlers/admin/`）。
- 路由 → 在 `routes.rs` 的 `build_router()` 注册；新增接口需同时挂 `/api/v1/` 与兼容层（如适用）。
- 新增数据模型：实现 `Serialize` + `Deserialize`（+ `Clone` 等）；ID 用 `Uuid::new_v4()`；时间用 `chrono::Utc::now()`。
- 新增权限 → 按上文权限清单的 4 步走。
- 业务规则放 `domain`，handler 保持薄。
- 错误码 → 新增到 `error.rs::ErrorCode`，别魔法字符串。
- 日志用 `tracing`；鉴权用 `effective_role`。
- 新增持久化字段 → 检查是否需要新迁移文件（`migrations/`）并同步 `db.rs` / `infra/persistence.rs`。

### 前端

- 页面组件 → `features/<领域>/`；通用组件 → `components/`（基础组件进 `components/ui/`）。
- 新增页面 → 在 `app/routes.tsx` 注册路由，布局复用 `app/layouts/`。
- 新增权限 → 改 `types.ts` 的 `Permission` 联合类型 + `defaultRolePermissions`（与后端 `perms` 保持一致）。
- API 调用 → 新增/扩展 `services/*.service.ts`，别在组件里裸 `fetch`。
- 路由守卫 → `ProtectedRoute`（组件级）+ `hasPermission()`（条件渲染）。
- 全局状态 → 用 zustand 的 `stores/`，而非临时起大量 Context。

---

## 演示模式

CP OAuth 不可用时回退。输入的 token 直接作为 user_id 前缀匹配预设用户数据，便于本地开发与演示。

---

## 提交与版本规范

- 前/后端版本号保持同步（`server/Cargo.toml` 的 `version` 与 `web/package.json` 的 `version`，当前均 `0.3.1`）。
- 提交信息建议使用 Conventional Commits（`feat:` / `fix:` / `chore:` / `refactor:` / `docs:` 等）。
- CI：`.github/workflows/test.yml`（PR/Push 跑测试与检查）、`docker.yml`（多架构镜像构建与推送）。
