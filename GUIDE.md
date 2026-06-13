# McGuffin 开发者指南

> 面向项目开发者（人类），包含技术架构、API 文档、开发流程与最佳实践。

---

## 1. 项目概述

**McGuffin** 是一个面向算法竞赛出题团队的协作工具，提供成员管理、题目投稿与审核、赛事管理、讨论社区等核心功能。

- 版本：0.2.2（前后端同步）
- 架构：前后端分离（React SPA + Rust/Axum API）
- 存储：SQLite（WAL 模式，自动备份）

```
浏览器 ──→ React SPA ──→ Axum API ──→ SQLite / CP OAuth
```

---

## 2. 环境搭建

### 2.1 依赖

| 工具 | 版本 | 安装 |
|------|------|------|
| Rust | ≥ 1.70 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun | 最新稳定 | `curl -fsSL https://bun.sh/install \| bash` |
| just（可选） | 最新 | `brew install just` / `cargo install just` |

### 2.2 启动开发服务器

```bash
# 终端 1：后端 (0.0.0.0:3000)
cd server && cargo run

# 终端 2：前端 (localhost:5173，代理 /api → :3000)
cd web && bun install && bun run dev
```

### 2.3 环境变量

**前端（web/.env）**：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `VITE_API_BASE_URL` | `/api` | API 代理路径 |

**后端**：

配置集中在 `/usr/share/mcguffin/config.toml`（或通过 `mcguffin init` 生成）。运行时配置通过 `toml_edit` 编辑。

### 2.4 配置项

| 配置路径 | 说明 | 是否需要重启 |
|----------|------|-------------|
| `server.site_url` | 站点 URL | 是 |
| `server.port` | 端口 | 是 |
| `server.data_file` | SQLite 路径 | 是 |
| `admin.password` | 管理员密码 | 是 |
| `admin.display_name` | 管理员显示名 | 否 |
| `site.name` / `title` | 站点信息 | 否 |
| `oauth.cp_client_id` / `cp_client_secret` | CP OAuth 凭据 | 是 |
| `difficulty.levels` | 难度等级 | 否 |
| `permissions` | 角色→权限映射 | 否 |
| `backup.interval_minutes` | 自动备份间隔 | 否 |
| `backup.retention_count` | 备份保留数 | 否 |

---

## 3. 项目结构

```
mcguffin/
├── justfile                   # 构建命令（just）
├── AGENTS.md                  # AI 编程助手指南
├── GUIDE.md                   # 本文件
├── README.md                  # 项目主页
├── Dockerfile / docker-compose.yml
├── web/                       # 前端 SPA
│   ├── src/
│   │   ├── App.tsx            # 路由 + Layout + Context
│   │   ├── api.ts             # API 客户端
│   │   ├── AuthContext.tsx    # 认证
│   │   ├── SiteContext.tsx    # 站点信息
│   │   ├── DarkModeContext.tsx # 暗色模式
│   │   ├── NotificationContext.tsx # 通知轮询
│   │   ├── types.ts           # TS 类型定义
│   │   ├── components/        # 9 个组件
│   │   ├── pages/             # 24 个页面
│   │   ├── hooks/             # 自定义 Hooks
│   │   ├── utils/             # 工具函数
│   │   └── test/              # 测试
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   └── tsconfig.json
├── server/                    # 后端
│   ├── Cargo.toml
│   ├── migrations/            # SQLite 迁移
│   ├── src/
│   │   ├── main.rs            # 入口 + 路由注册 + CORS
│   │   ├── lib.rs             # 模块导出
│   │   ├── types.rs           # 数据模型 + 20 种权限常量
│   │   ├── state.rs           # AppState + 持久化 + 配置
│   │   ├── db.rs              # SQLite 初始化/导入/导出/备份
│   │   ├── auth.rs            # OAuth + 密码登录
│   │   ├── user.rs            # 用户 CRUD
│   │   ├── team.rs            # 团队管理 + 权限组
│   │   ├── problems.rs        # 题目 CRUD + 审核 + 认领
│   │   ├── contests.rs        # 赛事管理
│   │   ├── discussions.rs     # 统一帖子系统
│   │   ├── community.rs       # 社区动态
│   │   ├── suggestions.rs     # 建议工作流
│   │   ├── announcements.rs   # 公告系统
│   │   ├── notifications.rs   # 通知系统
│   │   ├── info.rs            # 站点信息
│   │   ├── admin.rs           # 管理后台 API
│   │   ├── pages.rs           # SSR 页面（legacy）
│   │   ├── utils.rs           # 认证工具函数
│   │   └── bin/mcguffin.rs    # CLI 工具
│   └── tests/api.rs           # 集成测试
├── .github/workflows/
│   ├── test.yml               # PR/Push 测试（Linux/macOS/Windows）
│   ├── release.yml            # Tag 触发发布
│   └── docker.yml             # Docker 构建
└── backups/                   # 自动备份目录
```

---

## 4. 数据模型

### 4.1 User（用户）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `username` | String | 用户名（唯一） |
| `display_name` | String | 显示名 |
| `avatar_url` | Option\<String\> | 头像 |
| `email` | Option\<String\> | 邮箱 |
| `role` | String | admin / member / guest / pending |
| `team_status` | String | none / pending / joined |
| `effective_role` | String | 综合 role + team_status 后的实际角色 |
| `group_ids` | Vec\<String\> | 所属权限组 ID |
| `user_permissions` | Vec\<String\> | 个人额外权限 |
| `password_hash` | Option\<String\> | 管理员密码哈希 |
| `bio` | String | 个人简介 |
| `created_at` | DateTime\<Utc\> | 创建时间 |

### 4.2 Problem（题目）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `title` | String | 标题 |
| `author_id` | String | 作者 ID |
| `author_name` | String | 作者名 |
| `contest` | Option\<String\> | 所属赛事名 |
| `contest_id` | Option\<String\> | 关联赛事 ID |
| `difficulty` | String | 难度（可自定义） |
| `content` | String | Markdown 内容 |
| `solution` | Option\<String\> | 题解 |
| `status` | String | pending / approved / rejected / published |
| `claimed_by` | Option\<String\> | 认领人 ID |
| `verifier_solution` | Option\<String\> | 验题人解决方案 |
| `visible_to` | Vec\<String\> | 可见性 ACL |
| `link` | Option\<String\> | 外部链接 |
| `remark` | Option\<String\> | 备注 |
| `editable_by` | Vec\<String\> | 可编辑 ACL |
| `created_at` | DateTime\<Utc\> | 创建时间 |
| `public_at` | Option\<DateTime\<Utc\>\> | 公开时间 |

### 4.3 Contest（赛事）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `name` | String | 名称 |
| `start_time` | Option\<DateTime\> | 开始时间 |
| `end_time` | Option\<DateTime\> | 结束时间 |
| `description` | String | 描述 |
| `created_by` | String | 创建者 ID |
| `created_at` | DateTime\<Utc\> | 创建时间 |
| `status` | String | draft / upcoming / ongoing / finished |
| `link` | Option\<String\> | 外部链接 |
| `problem_order` | Vec\<String\> | 题目 ID 排序列表 |
| `visible_to` | Vec\<String\> | 可见性 ACL |
| `editable_by` | Vec\<String\> | 可编辑 ACL |

### 4.4 Post（统一帖子——讨论/公告/建议）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `title` | String | 标题 |
| `content` | String | 内容（Markdown） |
| `author_id` | String | 作者 |
| `author_name` | String | 作者名 |
| `tags` | Vec\<String\> | 标签 |
| `pinned` | bool | 是否置顶 |
| `team_only` | bool | 仅团队成员可见 |
| `emoji` | Option\<String\> | 类型 emoji |
| `reactions` | HashMap | 表情反应 |
| `replies` | Vec\<Reply\> | 回复列表 |
| `status` | String | 建议状态（suggestion 用） |
| `mentioned_user_ids` | Vec\<String\> | @提及用户 |
| `visible_to` / `editable_by` | Vec\<String\> | ACL |
| `created_at` / `updated_at` | DateTime\<Utc\> | 时间戳 |

### 4.5 TeamMember（团队成员）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `user_id` | String | 关联用户 ID |
| `joined_at` | String | 加入日期 |

### 4.6 JoinRequest（入队申请）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `user_id` | String | 申请人 |
| `user_name` | String | 姓名 |
| `user_email` | Option\<String\> | 邮箱 |
| `reason` | String | 理由 |
| `status` | String | pending / approved / rejected |
| `created_at` | DateTime\<Utc\> | 申请时间 |

### 4.7 MemberGroup（权限组）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `name` | String | 组名 |
| `permissions` | Vec\<String\> | 权限列表 |

### 4.8 Notification（通知）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | UUID |
| `user_id` | String | 目标用户 |
| `title` | String | 标题 |
| `body` | String | 内容 |
| `read` | bool | 已读 |
| `link` | Option\<String\> | 跳转链接 |
| `created_at` | DateTime\<Utc\> | 时间 |

---

## 5. API 接口文档

所有 API 路径以 `/api` 开头，返回 JSON。绑定 `0.0.0.0:3000`。

### 5.1 系统

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/health` | 健康检查 | 无 |

### 5.2 认证

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/oauth/authorize` | 发起 OAuth（重定向到 CP OAuth） | 无 |
| GET | `/api/oauth/callback?code=xxx` | OAuth 回调 | 无 |
| POST | `/api/oauth/token` | 刷新 token | Bearer |
| POST | `/api/auth/login` | 管理员密码登录 | 无 |
| GET | `/api/auth/permissions` | 获取角色→权限映射 | Bearer |
| GET | `/api/user/verify` | 验证 token | Bearer |
| POST | `/api/logout` | 登出 | Bearer |

### 5.3 用户

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/user/me` | 当前用户信息 | Bearer |
| PUT | `/api/user/profile` | 更新个人资料 | Bearer |
| GET | `/api/user/profile/{username}` | 公开个人页 | 无 |
| GET | `/api/user/check-name` | 检查用户名可用性 | 无 |

### 5.4 团队

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/team/members` | 成员列表 | view_team |
| POST | `/api/team/apply` | 申请加入 | apply_join |
| GET | `/api/team/requests` | 待审核申请列表 | manage_team |
| POST | `/api/team/review/{id}/{action}` | 审核申请 | manage_team |
| POST | `/api/team/members/role/{user_id}` | 变更角色 | manage_members |
| POST | `/api/team/members/remove/{user_id}` | 移除成员 | manage_members |

### 5.5 题目

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/problems` | 题目列表（?all=true 全部） | 公开（仅 approved） |
| POST | `/api/problems` | 投稿 | submit_problem |
| GET | `/api/problems/detail/{id}` | 题目详情 | view_problems |
| PUT | `/api/problems/{id}` | 更新题目 | 作者/admin |
| DELETE | `/api/problems/{id}` | 删除题目 | 作者/admin |
| POST | `/api/problems/review/{id}/{action}` | 审核 | approve_problem |
| POST | `/api/problems/claim/{id}` | 认领 | approve_problem |
| POST | `/api/problems/unclaim/{id}` | 取消认领 | approve_problem |
| POST | `/api/problems/verifier-solution/{id}` | 提交验题方案 | 认领人 |
| POST | `/api/problems/visibility/{id}` | 设置可见性 | manage_site |
| POST | `/api/problems/contest/{id}` | 分配赛事 | manage_contests |
| GET | `/api/problems/admin/pending` | 待审核列表（admin） | approve_problem |
| GET | `/api/problems/admin/members` | 成员列表（可见性设置用） | manage_site |

### 5.6 赛事

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/contests` | 赛事列表 | 公开/鉴权 |
| POST | `/api/contests` | 创建 | manage_contests |
| PUT | `/api/contests/{id}` | 更新 | manage_contests |
| DELETE | `/api/contests/{id}` | 删除 | manage_contests |
| POST | `/api/contests/{id}/status` | 设置状态 | manage_contests |
| GET | `/api/contests/{id}/problems` | 赛事题目列表 | 公开 |
| POST | `/api/contests/{id}/problem-order` | 题目排序 | manage_contests |

### 5.7 帖子（统一系统）

**新路径**（推荐）：

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/posts` | 帖子列表 | 公开 |
| POST | `/api/posts` | 创建帖子 | view_discussions |
| GET | `/api/posts/{id}` | 帖子详情 | 公开 |
| PUT | `/api/posts/{id}` | 更新 | 作者/admin |
| DELETE | `/api/posts/{id}` | 删除 | 作者/admin |
| POST | `/api/posts/{id}/reply` | 回复 | view_discussions |
| DELETE | `/api/posts/{id}/reply/{rid}` | 删除回复 | 作者/admin |
| POST | `/api/posts/{id}/react` | 反应（帖子） | 登录 |
| POST | `/api/posts/{id}/reply/{rid}/react` | 反应（回复） | 登录 |
| GET | `/api/posts/tags` | 标签列表 | 无 |
| GET | `/api/posts/emojis` | emoji 列表 | 无 |

**旧路径兼容**：`/api/discussions/*` 别名到同一 handler。

### 5.8 社区

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/community/posts` | 公开社区动态 | 无 |

### 5.9 建议（旧路径兼容）

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET/POST | `/api/suggestions` | 列表/创建 | view_discussions |
| GET/PUT/DELETE | `/api/suggestions/{id}` | 详情/更新/删除 | 作者/admin |
| POST | `/api/suggestions/{id}/reply` | 回复 | view_discussions |
| DELETE | `/api/suggestions/{id}/reply/{rid}` | 删除回复 | 作者/admin |

### 5.10 公告（旧路径兼容）

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET/POST | `/api/announcements` | 列表/创建 | manage_posts |
| GET/PUT/DELETE | `/api/announcements/{id}` | 详情/更新/删除 | 作者/admin |

### 5.11 通知

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/notifications` | 通知列表 | Bearer |
| POST | `/api/notifications/read/{id}` | 标为已读 | Bearer |
| POST | `/api/notifications/read-all` | 全部已读 | Bearer |

### 5.12 站点信息

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/site/info` | 站点信息 | 无 |
| PUT | `/api/site/description` | 更新描述 | manage_site |
| GET | `/api/site/difficulties` | 难度等级 | 无 |

### 5.13 管理后台

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET/PUT | `/api/admin/config` | 配置查看/修改 | manage_site |
| POST | `/api/admin/restart` | 重启服务 | manage_site |
| POST | `/api/admin/backup` | 创建备份 | manage_backups |
| GET | `/api/admin/backups` | 备份列表 | manage_backups |
| POST | `/api/admin/backup/restore/{name}` | 恢复备份 | manage_backups |
| POST | `/api/admin/backup/restore-upload` | 上传恢复 | manage_backups |
| GET | `/api/admin/backup/download/{name}` | 下载备份 | manage_backups |
| DELETE | `/api/admin/backup/{name}` | 删除备份 | manage_backups |
| GET/PUT | `/api/admin/showcase` | 展示页配置 | manage_site |
| GET | `/api/admin/export/data` | 导出数据（JSON） | manage_backups |
| GET | `/api/admin/export/config` | 导出配置（TOML） | manage_backups |
| POST | `/api/admin/import/data` | 导入数据 | manage_backups |
| POST | `/api/admin/import/config` | 导入配置 | manage_backups |
| GET | `/api/admin/audit-log` | 审计日志 | manage_site |
| GET | `/api/admin/users` | 用户列表 | manage_site |
| POST | `/api/admin/users/{id}/role` | 变更用户角色 | manage_site |
| POST | `/api/admin/users/{id}/remove` | 移除用户 | manage_site |
| PUT | `/api/admin/users/{id}/groups` | 设置用户组 | manage_site |
| PUT | `/api/admin/users/{id}/permissions` | 设置个人权限 | manage_site |
| GET/POST | `/api/admin/groups` | 权限组列表/创建 | manage_site |
| PUT/DELETE | `/api/admin/groups/{id}` | 更新/删除权限组 | manage_site |
| PUT | `/api/admin/problems/{id}/acl` | 题目 ACL | manage_site |
| PUT | `/api/admin/acl/{type}/{id}` | 统一资源 ACL | manage_site |

---

## 6. 权限体系

**三级权限**（OR 关系取并集）：角色基础权限 → 成员组权限 → 个人额外权限

**角色→权限默认映射**（可通过 `config.toml` 覆盖）：

| 角色 | 权限 |
|------|------|
| superadmin | `*`（全部） |
| admin | 全部（manage_site, manage_backups, approve 等） |
| member | submit_problem, view_problems, view_discussions 等 |
| guest | view_showcase, apply_join, view_public_contests |
| pending | 仅 apply_join（已申请状态） |

`effective_role` 计算：如果 `role=member` 且 `team_status=joined` → `member`；如果 `team_status=pending` → `pending`；如果 `team_status=none` → `guest`。

---

## 7. 数据存储

### 7.1 SQLite 持久化

- 路径：由 `config.toml` 的 `server.data_file` 决定，默认 `mcguffin_data.db`
- 连接池：`SqlitePool`，`max_connections=5`
- WAL 模式：`PRAGMA journal_mode=WAL` + `synchronous=NORMAL`
- 迁移：`server/migrations/` 目录，启动时自动运行 `sqlx::migrate!()`

### 7.2 内存缓存 + 定时写回

启动时 SQLite → 内存 `HashMap`，运行时操作内存，通过 `save_all_to_db()` 写回 SQLite。

### 7.3 自动备份

启动时自动启动定时备份线程（间隔和保留数从 `config.toml` 读取）。备份文件存到 `backups/` 目录。

### 7.4 手动备份（CLI）

```bash
mcguffin backup create
mcguffin backup list
mcguffin backup restore <name>
mcguffin backup delete <name>
```

---

## 8. 构建与部署

### 8.1 开发构建

```bash
just fast-deploy    # debug 编译后端 + 构建前端 + 重启服务（5-15s）
```

### 8.2 生产构建

```bash
just build          # release 构建后端 + 前端
sudo just install   # 安装到 /usr/local
mcguffin init       # 交互式配置
mcguffin start      # 启动后台服务
```

### 8.3 Docker 部署

```bash
docker compose up -d
# → http://localhost:3000
```

### 8.4 运行测试

```bash
# 全部
just test

# 后端
cd server && cargo test

# 前端
cd web && bun run test
```

### 8.5 代码检查

```bash
just check               # cargo check + clippy + tsc
cd server && cargo fmt   # Rust 格式化
```

---

## 9. UI 设计规范

- 灰白色系，低饱和度
- **直角**（`border-radius: 0`）
- 无渐变色
- Tailwind `gray` 色阶（gray-50 ~ gray-900）
- 状态色：绿（成功/通过）、黄（待处理/待审核）、红（失败/拒绝）

---

## 10. CLI 工具

```bash
mcguffin init                     # 交互式生成配置
mcguffin config show              # 查看配置
mcguffin config set <key> <value>  # 修改配置
mcguffin backup create            # 创建备份
mcguffin backup list              # 列出备份
mcguffin backup restore <name>    # 恢复备份
mcguffin backup delete <name>     # 删除备份
mcguffin service start            # 启动服务
mcguffin service stop             # 停止服务
mcguffin service restart          # 重启服务
mcguffin service status           # 查看状态
```

---

## 11. 已知限制与待改进

### 安全性

- Client Secret 硬编码，需迁移至环境变量
- Token 存 localStorage（XSS 风险）
- CORS 开发期允许所有来源，生产环境应限制

### 功能

- 非成员投稿功能未实现
- 缺少国际化支持

### 工程质量

- 前端 `App.tsx` 较大（路由 + Layout + Context）
- 测试覆盖不完整
