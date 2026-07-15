# AGENTS.md — McGuffin AI Agent 指南

> 面向 AI 编程助手（Claude Code、OpenCode、Copilot 等），提供代码库结构与开发约定。

---

## 项目概览

**McGuffin** 是算法竞赛出题团队的协作工具。React SPA + Rust/Axum 后端，CP OAuth 认证，SQLite 持久化。

- 版本：0.2.1
- 前/后端版本号同步
- 架构：浏览器 → React SPA → Axum API → SQLite / CP OAuth

---

## 构建与运行

```bash
# 后端
cd server
cargo check              # 类型检查（快）
cargo test               # 运行测试
cargo build --release    # 生产构建 → target/release/mcguffin-server + mcguffin
cargo run                # 开发服务器 :3000

# 前端
cd web
bun install
bun run dev              # 开发服务器 :5173（代理 /api → :3000）
bun run build            # 生产构建 → dist/（含 tsc --noEmit）

# 全量
just build               # 构建全部
sudo just install        # 安装到 /usr/local
mcguffin init            # 交互式配置
mcguffin start           # 启动服务
```

**快速部署**（开发机）：`just fast-deploy`（debug 编译 5-15s + 重启服务）

---

## 项目结构

```
mcguffin/
├── docs/                     # 文档（部署/管理/使用）
│   ├── README.md             # 文档索引
│   ├── guide/                # 部署与开发指南
│   ├── admin/                # 管理后台手册
│   └── user/                 # 用户手册
├── justfile                  # 构建命令
├── AGENTS.md                 # 本文件
├── GUIDE.md                  # 开发者指南（人类）
├── README.md                 # 项目主页
├── Dockerfile                # Docker 构建（多架构: amd64 + arm64）
├── .dockerignore             # Docker 构建上下文排除
├── docker-compose.yml        # Docker Compose
├── docker-entrypoint.sh      # 容器入口
├── web/                      # 前端 SPA
│   ├── src/
│   │   ├── App.tsx           # 路由 + 布局 + Context 提供者
│   │   ├── main.tsx          # 入口
│   │   ├── index.css         # Tailwind + 自定义样式
│   │   ├── types.ts          # TypeScript 类型定义
│   │   ├── api.ts            # API 客户端封装
│   │   ├── AuthContext.tsx    # 认证状态
│   │   ├── SiteContext.tsx    # 站点信息
│   │   ├── DarkModeContext.tsx # 暗色模式
│   │   ├── NotificationContext.tsx # 通知轮询
│   │   ├── components/       # 9 个可复用组件
│   │   ├── pages/            # 24 个页面组件
│   │   ├── hooks/            # 自定义 Hooks
│   │   ├── utils/            # 工具函数
│   │   └── test/             # Vitest 测试
│   └── ...config files
├── server/                   # 后端
│   ├── Cargo.toml
│   ├── migrations/           # SQLite 迁移
│   ├── src/
│   │   ├── main.rs           # 入口 + 路由注册 + CORS
│   │   ├── lib.rs            # 模块导出
│   │   ├── types.rs          # 所有数据结构 + 权限常量
│   │   ├── state.rs          # AppState + 持久化 + 配置
│   │   ├── db.rs             # SQLite 初始化 + 数据导入/导出/备份
│   │   ├── auth.rs           # OAuth + 管理员密码登录
│   │   ├── user.rs           # 用户信息/更新/验证
│   │   ├── team.rs           # 成员/申请/角色/权限组
│   │   ├── problems.rs       # 题目 CRUD + 审核 + 认领
│   │   ├── contests.rs       # 赛事 CRUD + 状态流转
│   │   ├── discussions.rs    # 统一帖子系统
│   │   ├── community.rs      # 公开社区动态
│   │   ├── suggestions.rs    # 建议工作流
│   │   ├── announcements.rs  # 公告 CRUD
│   │   ├── notifications.rs  # 通知 CRUD
│   │   ├── info.rs           # 站点信息
│   │   ├── admin.rs          # 管理后台 API
│   │   ├── pages.rs          # SSR 页面（legacy）
│   │   ├── utils.rs          # 认证工具函数
│   │   └── bin/mcguffin.rs   # CLI 工具
│   └── tests/api.rs          # 集成测试
├── .github/workflows/
    ├── test.yml              # PR/Push 测试
    ├── release.yml           # Tag 触发发布
    └── docker.yml            # Docker 多架构构建（原生 amd64 + arm64 并行）
```

---

## 权限体系

两级校验：后端计算 `effective_role`（综合 role + team_status + 用户权限 + 组权限），前端通过 `GET /api/auth/permissions` 获取角色→权限映射。

**角色层级**：`superadmin` (id=admin, 不可删除/降级) > `admin` > `member` > `guest` > `pending`

**20 种权限**（`perms` 模块）：

| 权限 | 说明 |
|------|------|
| `view_showcase` | 查看成果展示（公开） |
| `apply_join` | 申请加入团队 |
| `view_team` | 查看团队成员 |
| `manage_team` | 审核入队申请 |
| `manage_members` | 踢出成员/变更角色 |
| `submit_problem` | 投稿题目 |
| `view_problems` | 查看题目 |
| `approve_problem` | 审核题目 |
| `manage_contests` | 管理赛事 |
| `view_all_contests` | 查看全部赛事（含 draft） |
| `view_public_contests` | 查看公开赛事 |
| `manage_site` | 站点配置 |
| `edit_showcase` | 编辑展示页 |
| `view_discussions` | 查看讨论 |
| `manage_discussions` | 管理讨论 |
| `manage_tags` | 管理标签 |
| `manage_notifications` | 管理通知 |
| `manage_backups` | 管理备份 |
| `view_stats` | 查看统计 |
| `manage_posts` | 管理统一帖子 |

超级管理员（user_id=admin）拥有通配符 `*` 全部权限。

---

## 关键设计模式

- **API 模式**：每个 handler 取 `State<AppState>` + `HeaderMap`，调用 `resolve_user()` 鉴权，内联权限检查，返回 `Json(serde_json::json!(...))`。没有中间件鉴权层。
- **持久化**：启动时 SQLite → 内存 HashMap，运行时操作内存，定时写回 SQLite。通过 `db.rs` 的 `save_all_to_db()` 和 `reload_all_from_db()` 同步。
- **配置**：`/usr/share/mcguffin/config.toml`（TOML），通过 `toml_edit` 支持运行时编辑。包含 server/site/oauth/difficulty/permissions 等。
- **CLI**：`cargo run --bin mcguffin` — 子命令 `init`、`config`、`backup`、`service start/stop/restart/status`。
- **Superadmin 保护**：`ADMIN_USER_ID = "admin"` 硬编码，不可降级/删除。仅 superadmin 可操作其他 admin。
- **统一帖子系统**：Post 结构体替代独立的讨论/建议/公告表。标签、emoji、反应、回复、可见性 ACL 都在 Post 上。

---

## 修改代码注意事项

### 后端
- 数据模型在 `types.rs`，API handler 在各自模块，路由注册在 `main.rs`
- 新增接口：handler + 路由 + 权限校验
- 新增数据模型：`Serialize` + `Deserialize` + `Clone`；ID 用 `Uuid::new_v4()`；时间用 `chrono::Utc::now()`
- **权限检查必须用 `effective_role` 而非 `role`**（涉及团队状态覆盖）
- 日志用 `tracing`，不用 `println!`

### 前端
- 页面组件在 `pages/`，通用组件在 `components/`
- 新增页面需在 `App.tsx` 的 `<Routes>` 注册
- 新增权限需在 `types.ts` 的 `Permission` 联合类型和权限映射中添加
- 路由守卫用 `ProtectedRoute`（组件级）+ `hasPermission()`（条件渲染）

---

## 演示模式

CP OAuth 不可用时回退。输入的 token 直接作为 user_id 前缀匹配预设用户数据。

---

## 已知限制

- Client Secret 硬编码（应环境变量化）
- Token 存 localStorage（XSS 风险）
- CORS 配置为 `Any`
- 前端 `App.tsx` 较大（含路由 + 所有 Layout + Context）
