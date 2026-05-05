# AGENTS.md — McGuffin AI Agent 指南

> 面向 AI 编程助手（如 Claude、GitHub Copilot、opencode 等），提供代码库结构、开发约定与操作指南。

---

## 1. 项目概览

**McGuffin** 是面向算法竞赛出题团队的协作工具，采用前后端分离架构。

- 前端：React + TypeScript + Vite + Tailwind CSS（包管理器：Bun）
- 后端：Rust + Axum（HTTP 框架）+ Tokio（异步运行时）
- 认证：CP OAuth（支持 PKCE）
- 存储：内存存储（HashMap + Arc\<RwLock\>），服务重启后数据丢失

---

## 2. 关键文件索引

### 2.1 项目根目录

| 文件 | 用途 |
|------|------|
| `DEMAND.md` | 需求文档（功能定义、UI 规范） |
| `DEVELOPMENT.md` | 详细开发文档（数据模型、API、架构） |
| `GUIDE.md` | 面向人类开发者的指南 |
| `README.md` | 项目主页（面向用户） |

### 2.2 前端（`web/`）

| 文件 | 用途 |
|------|------|
| `web/src/App.tsx` | **核心文件**：所有页面组件、路由、权限逻辑集中在此 |
| `web/src/main.tsx` | 应用入口 |
| `web/src/oauthConfig.ts` | OAuth 配置、PKCE 工具函数、token 管理 |
| `web/package.json` | 依赖与脚本 |
| `web/vite.config.ts` | Vite 构建配置 |
| `web/tailwind.config.js` | Tailwind CSS 配置 |
| `web/tsconfig.json` | TypeScript 配置 |

### 2.3 后端（`server/`）

| 文件 | 用途 |
|------|------|
| `server/src/main.rs` | 服务入口：路由注册、中间件配置、启动逻辑 |
| `server/src/lib.rs` | **核心文件**：数据模型定义、AppState、所有 API handler |
| `server/Cargo.toml` | Rust 依赖配置 |

---

## 3. 开发约定

### 3.1 代码风格

- **前端**：
  - 使用 TypeScript（`~5.6.2`），严格模式
  - React 函数组件，使用 hooks
  - 不使用圆角（`border-radius: 0`）
  - 使用 Tailwind CSS 的 `gray` 色阶，避免渐变
  - 无 emoji，注释使用中文

- **后端**：
  - Rust 2021 edition
  - 使用 `tracing` 而非 `println!` 做日志
  - 错误处理使用 `Result`，API 返回 JSON 格式
  - 数据模型使用 `serde` 的 `Serialize`/`Deserialize`

### 3.2 命名规范

- **Rust**：snake_case（变量/函数）、PascalCase（类型/结构体）、UPPER_SNAKE_CASE（常量）
- **TypeScript**：camelCase（变量/函数）、PascalCase（组件/类型）
- **API 路径**：kebab-case（`/api/team-members`）或 snake_case（`/api/team/members`）

### 3.3 权限体系

前端定义了 7 种权限标识符：

```
view_portfolio     - 查看成果展示（所有人，含未登录）
apply_join         - 申请加入团队（登录用户，非 pending）
view_team          - 查看团队成员（member 及以上）
manage_team        - 团队管理（仅 admin）
submit_problem     - 投稿题目（member 及以上）
view_all_problems  - 查看所有题目含待审核（member 及以上）
approve_problem    - 审核题目（仅 admin）
```

---

## 4. 数据模型

### 4.1 User

```rust
// 后端 (lib.rs)
struct User {
    id: String,           // CP OAuth sub
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    email: Option<String>,
    role: String,         // "admin" | "member" | "guest" | "pending"
    team_status: String,  // "none" | "pending" | "joined"
    created_at: DateTime<Utc>,
}
```

### 4.2 Problem

```rust
struct Problem {
    id: String,
    title: String,
    author_id: String,
    author_name: String,
    contest: String,
    difficulty: String,   // "Easy" | "Medium" | "Hard"
    content: String,      // Markdown 格式
    status: String,       // "pending" | "approved" | "rejected"
    created_at: DateTime<Utc>,
    public_at: Option<DateTime<Utc>>,
}
```

### 4.3 TeamMember

```rust
struct TeamMember {
    id: String,
    user_id: String,
    name: String,
    avatar: String,
    role: String,         // "admin" | "member"
    joined_at: String,    // YYYY-MM-DD
}
```

### 4.4 JoinRequest

```rust
struct JoinRequest {
    id: String,
    user_id: String,
    user_name: String,
    user_email: String,
    reason: String,
    status: String,       // "pending" | "approved" | "rejected"
    created_at: DateTime<Utc>,
}
```

---

## 5. API 速查表

### 5.1 认证

```
GET  /api/oauth/authorize              → 重定向到 CP OAuth
GET  /api/oauth/callback?code=xxx      → OAuth 回调
POST /api/oauth/token                  → 刷新 token
POST /api/logout                       → 登出
GET  /api/user/me                      → 当前用户信息 (Bearer Token)
GET  /api/user/verify                  → 验证 token
```

### 5.2 团队

```
GET  /api/team/members                 → 团队成员列表
GET  /api/team/requests                → 待处理入队申请
POST /api/team/apply                   → 申请加入 (body: {"reason": "..."})
POST /api/team/review/:id/:action      → 审核申请 (action: approve|reject)
```

### 5.3 题目

```
GET  /api/problems                     → 题目列表 (?all=true 返回全部)
POST /api/problems                     → 投稿题目
POST /api/problems/review/:id/:action  → 审核题目 (action: approve|reject)
```

---

## 6. 常用操作

### 6.1 安装与启动

```bash
# 前端
cd web && bun install && bun run dev

# 后端
cd server && cargo run

# 生产构建
cd web && bun run build          # → dist/
cd server && cargo build --release  # → target/release/mcguffin-server
```

### 6.2 运行测试

```bash
# 后端
cd server && cargo test

# 前端
cd web && bun run test
```

### 6.3 格式检查

```bash
# Rust
cd server && cargo fmt && cargo clippy

# TypeScript (类型检查已在 build 脚本中)
cd web && bun run build  # 包含 tsc --noEmit
```

---

## 7. 修改代码时的注意事项

### 7.1 前端修改

- 所有组件集中在 `web/src/App.tsx`，修改时注意保持文件结构清晰
- 新增页面需要在路由表中注册（`App.tsx` 中的 `<Routes>`）
- 新增权限需要在权限矩阵和角色权限映射中添加对应项
- OAuth 配置在 `web/src/oauthConfig.ts` 中修改

### 7.2 后端修改

- 数据模型定义在 `server/src/lib.rs` 顶部
- API handler 函数在 `server/src/lib.rs` 中
- 路由注册在 `server/src/main.rs` 的 `Router` 中
- 新增接口需要：
  1. 在 `lib.rs` 中定义 handler 函数
  2. 在 `main.rs` 中注册路由
  3. 考虑权限校验（检查 `role` 或 `team_status`）

### 7.3 新增数据模型

- 结构体需要实现 `Serialize`、`Deserialize`、`Clone`
- 使用 `uuid::Uuid::new_v4().to_string()` 生成 ID
- 时间字段使用 `chrono::Utc::now()`

### 7.4 数据存储

- 当前全部使用内存存储（`Arc<RwLock<HashMap<...>>>`）
- 所有状态在 `AppState` 中管理
- `AppState` 通过 `Axum` 的 `Extension` 中间件注入到 handler

---

## 8. 已知限制与待办

> 修改代码时请留意以下已知问题，避免引入新 bug。

### 8.1 必须注意的安全问题

- Client Secret 硬编码（前端 + 后端），生产环境需迁移到环境变量
- Token 存储在 localStorage（XSS 风险）
- CORS 配置为 `Any`（允许所有来源）
- 部分使用 GET 请求的接口实际修改了状态（应改为 POST/PUT/DELETE）

### 8.2 数据丢失

- 所有数据存储在内存中，服务重启后丢失

### 8.3 未实现功能

- 踢出成员的后端 API
- 非成员投稿功能
- 题目详情查看页面
- Markdown 渲染（内容已保存为 Markdown 但未在前端渲染）
- 用户个人资料页面

### 8.4 工程质量

- 前端所有组件集中在单一文件 `App.tsx`
- 缺少单元测试和集成测试
- 缺少错误边界和全局错误处理

---

## 9. UI 设计规范

- 灰白色系，低饱和度素色
- **不使用圆角**（所有 `border-radius` 应为 0）
- 不使用渐变色
- 使用 Tailwind 的 `gray` 色阶（gray-50 到 gray-900）
- 状态色：绿色（成功/通过）、黄色（待处理）、红色（失败/拒绝）

---

## 10. 演示模式

当 CP OAuth 未配置或请求失败时，系统回退到演示模式：

| 输入的 token | 模拟身份 |
|-------------|---------|
| `admin_token` | 张三，admin，已加入团队 |
| `member_token` | 赵六，member，已加入团队 |
| `pending_token` | 申请者，pending，待审核 |
| `new_user_token` | 新用户，guest，未加入团队 |

其他任意 token 也会创建 guest 用户。
