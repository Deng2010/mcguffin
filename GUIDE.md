# McGuffin 开发者指南

> 面向项目开发者（人类），包含技术架构、API 文档、开发流程与最佳实践。

---

## 1. 项目概述

**McGuffin** 是一个面向算法竞赛出题团队的协作工具，提供成员管理、题目投稿与审核、成果展示等核心功能。

- 项目名称：McGuffin（麦高芬）
- 当前版本：后端 0.1.0 / 前端 0.0.1
- 架构：前后端分离

```
浏览器 ──→ web (React SPA) ──→ server (Rust/Axum) ──→ CP OAuth
```

---

## 2. 环境搭建

### 2.1 安装依赖

**前端：**
```bash
curl -fsSL https://bun.sh/install | bash
cd web && bun install
```

**后端：**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cd server && cargo build
```

### 2.2 启动开发服务器

```bash
# 终端 1：后端 (0.0.0.0:3000)
cd server && cargo run

# 终端 2：前端 (localhost:5173)
cd web && bun run dev
```

### 2.3 环境变量

**前端 (web)：**

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `VITE_CPOAUTH_BASE_URL` | `https://cpoauth.com` | CP OAuth 服务地址 |
| `VITE_CPOAUTH_CLIENT_ID` | `your-client-id` | 应用 Client ID |
| `VITE_CPOAUTH_CLIENT_SECRET` | `your-client-secret` | 应用 Client Secret |
| `VITE_CPOAUTH_REDIRECT_URI` | `{origin}/oauth/cpoauth/callback` | 回调地址 |

**后端 (server)：**

OAuth 配置目前硬编码在 `AppState::new()` 中，建议后续迁移到环境变量：
- `cpoauth_client_id`: `"mcguffin_app"`
- `cpoauth_client_secret`: `"your_client_secret_here"`
- `cpoauth_redirect_uri`: `"http://localhost:3000/api/oauth/callback"`

---

## 3. 项目结构

```
mcguffin/
├── DEMAND.md                  # 需求文档
├── DEVELOPMENT.md             # 详细开发文档
├── GUIDE.md                   # 本文件：开发者指南
├── AGENTS.md                  # AI Agent 开发指南
├── web/                       # 前端项目
│   ├── index.html
│   ├── package.json
│   ├── vite.config.ts
│   ├── tailwind.config.js
│   ├── tsconfig.json
│   └── src/
│       ├── App.tsx            # 主应用组件（含所有页面与逻辑）
│       ├── oauthConfig.ts     # OAuth 配置与工具函数
│       └── main.tsx           # 应用入口
│
└── server/                    # 后端项目
    ├── Cargo.toml
    └── src/
        ├── main.rs            # 服务入口（路由注册、启动配置）
        └── lib.rs             # 核心业务逻辑（数据模型、API 处理）
```

---

## 4. 数据模型

### 4.1 User（用户）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 用户唯一标识（来自 CP OAuth sub） |
| `username` | String | 用户名 |
| `display_name` | String | 显示名称 |
| `avatar_url` | Option\<String\> | 头像 URL |
| `email` | Option\<String\> | 邮箱 |
| `role` | String | 角色：admin / member / guest / pending |
| `team_status` | String | 团队状态：none / pending / joined |
| `created_at` | DateTime\<Utc\> | 创建时间 |

### 4.2 TeamMember（团队成员）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 成员记录 ID |
| `user_id` | String | 关联用户 ID |
| `name` | String | 成员名称 |
| `avatar` | String | 头像标识 |
| `role` | String | 角色：admin / member |
| `joined_at` | String | 加入日期（YYYY-MM-DD） |

### 4.3 Problem（题目）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 题目唯一标识 |
| `title` | String | 题目标题 |
| `author_id` | String | 作者用户 ID |
| `author_name` | String | 作者显示名 |
| `contest` | String | 所属赛事 |
| `difficulty` | String | 难度：Easy / Medium / Hard |
| `content` | String | 题目内容（Markdown 格式） |
| `status` | String | 状态：pending / approved / rejected |
| `created_at` | DateTime\<Utc\> | 创建时间 |
| `public_at` | Option\<DateTime\<Utc\>\> | 公开时间 |

### 4.4 JoinRequest（入队申请）

| 字段 | 类型 | 说明 |
|------|------|------|
| `id` | String | 申请 ID |
| `user_id` | String | 申请人用户 ID |
| `user_name` | String | 申请人名称 |
| `user_email` | String | 申请人邮箱 |
| `reason` | String | 申请理由 |
| `status` | String | 状态：pending / approved / rejected |
| `created_at` | DateTime\<Utc\> | 申请时间 |

---

## 5. API 接口文档

后端服务监听 `0.0.0.0:3000`。

### 5.1 页面路由

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/` | 重定向到 /portfolio |
| GET | `/login` | 登录页面（SSR HTML） |
| GET | `/portfolio` | 成果展示页面（SSR） |

### 5.2 OAuth 认证

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/oauth/authorize` | 发起 OAuth 授权，重定向到 CP OAuth |
| GET | `/api/oauth/callback` | OAuth 回调，交换 token 并创建会话 |
| POST | `/api/oauth/token` | 使用 refresh_token 刷新 access_token |

**OAuth 流程：**
1. 重定向至 `/api/oauth/authorize`
2. 构建 CP OAuth 授权 URL（含 PKCE code_challenge）并跳转
3. 用户授权后回调至 `/api/oauth/callback?code=xxx`
4. 使用 code 向 CP OAuth 交换 access_token
5. 使用 access_token 获取用户信息
6. 创建本地会话，session_token 通过重定向传给前端

### 5.3 用户接口

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/user/me` | 获取当前用户信息 | Bearer Token |
| GET | `/api/user/verify` | 验证 token 有效性 | Bearer Token |

### 5.4 团队接口

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/team/members` | 获取团队成员列表 | 需要团队身份 |
| GET | `/api/team/requests` | 获取待处理入队申请 | 需要团队身份 |
| POST | `/api/team/apply` | 申请加入团队 | 需要登录 |
| POST | `/api/team/review/:request_id/:action` | 审核入队申请 | 需要 admin |

### 5.5 题目接口

| 方法 | 路径 | 说明 | 鉴权 |
|------|------|------|------|
| GET | `/api/problems` | 获取题目列表（`?all=true` 返回全部） | 无（公开仅 approved） |
| POST | `/api/problems` | 投稿题目 | 需要团队成员身份 |
| POST | `/api/problems/review/:problem_id/:action` | 审核题目 | 需要 admin |

**投稿题目请求体：**
```json
{
  "title": "题目标题",
  "contest": "赛事名称",
  "difficulty": "Easy|Medium|Hard",
  "content": "Markdown 格式的题目内容"
}
```

### 5.6 登出

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/logout` | 登出，销毁当前会话 |

---

## 6. 前端路由与权限

### 6.1 路由表

| 路径 | 组件 | 权限要求 |
|------|------|----------|
| `/` | HomePage（→ /portfolio） | 无 |
| `/login` | LoginPage | 无 |
| `/portfolio` | PortfolioPage | 无（公开） |
| `/oauth/cpoauth/authorize` | OAuthAuthorizePage | 无 |
| `/oauth/cpoauth/callback` | → /portfolio | 无 |
| `/team` | TeamPage | view_team |
| `/add` | AddProblemPage | submit_problem |
| `/apply` | ApplyPage | apply_join |

### 6.2 权限矩阵

| 权限 | 说明 | admin | member | guest | pending | 未登录 |
|------|------|-------|--------|-------|---------|--------|
| `view_portfolio` | 查看成果展示 | Y | Y | Y | Y | Y |
| `apply_join` | 申请加入团队 | - | Y | Y | Y | - |
| `view_team` | 查看团队成员 | Y | Y | - | - | - |
| `manage_team` | 团队管理 | Y | - | - | - | - |
| `submit_problem` | 投稿题目 | Y | Y | - | - | - |
| `view_all_problems` | 查看所有题目 | Y | Y | - | - | - |
| `approve_problem` | 审核题目 | Y | - | - | - | - |

### 6.3 认证流程

1. 用户点击"通过 CP OAuth 登录"
2. 前端生成 CSRF state，存储在 sessionStorage
3. 重定向至 CP OAuth 授权页面
4. 用户授权后回调至前端 `/oauth/cpoauth/callback`
5. 前端使用 code 换取 token，获取用户信息
6. 如果 OAuth 请求失败，回退到演示模式（模拟登录）
7. 用户信息存入 localStorage，权限根据团队成员列表判定

**演示模式账号：**

| Token | 角色 |
|-------|------|
| `admin_token` | 张三（管理员） |
| `member_token` | 赵六（成员） |
| `pending_token` | 申请者（待审核） |
| `new_user_token` | 新用户（游客） |

---

## 7. 数据存储

**当前状态：** 全部使用内存存储（`HashMap + Arc<RwLock>`），服务重启后数据丢失。

| 存储项 | 数据结构 | 说明 |
|--------|----------|------|
| `users` | `HashMap<String, User>` | 用户信息 |
| `sessions` | `HashMap<String, String>` | session_token → user_id |
| `refresh_tokens` | `HashMap<String, String>` | refresh_token → user_id |
| `team_members` | `HashMap<String, TeamMember>` | 团队成员 |
| `problems` | `HashMap<String, Problem>` | 题目列表 |
| `join_requests` | `HashMap<String, JoinRequest>` | 入队申请 |

**预置数据：**
- 3 个团队成员：张三（admin）、李四（member）、王五（member）
- 3 个已审核题目：二叉树的最大深度（Easy）、回文子串（Medium）、最优括号匹配（Hard）

---

## 8. 构建与部署

### 8.1 前端构建

```bash
cd web
bun run build      # 产物输出到 dist/
bun run preview    # 本地预览构建产物
```

### 8.2 后端构建

```bash
cd server
cargo build --release  # 产物在 target/release/mcguffin-server
```

### 8.3 运行测试

```bash
# 后端
cd server && cargo test

# 前端
cd web && bun run test
cd web && bun run test:watch  # 监听模式
```

---

## 9. UI 设计规范

根据 DEMAND.md 的要求：

- **主色调：** 灰白色系
- **辅助色：** 低饱和度素色，避免过多渐变色
- **圆角策略：** 使用直角（`border-radius: 0`）
- **风格：** 简洁、专业、中性

前端使用 Tailwind CSS 实现，主要使用 `gray` 色阶体系。

---

## 10. 已知限制与待改进项

### 10.1 安全性

- [ ] Client Secret 硬编码在代码中，需迁移至环境变量
- [ ] Token 存储在 localStorage 中，存在 XSS 风险，建议使用 HttpOnly Cookie
- [ ] CORS 配置为允许所有来源，生产环境需限制为前端域名
- [ ] 题目投稿接口需增加后端权限校验
- [ ] 缺少 CSRF 防护

### 10.2 数据持久化

- [ ] 所有数据存储在内存中，需接入数据库
- [ ] 建议使用 PostgreSQL 或 SQLite

### 10.3 功能完善

- [ ] 前端登录页面使用模拟 OAuth 流程，需接入真实 CP OAuth
- [ ] 前端部分页面仍在使用 mock 数据，需对接后端 API
- [ ] 缺少题目详情查看页面
- [ ] 缺少用户个人资料页面
- [ ] 缺少非成员投稿功能
- [ ] 踢出成员的后端 API 尚未实现

### 10.4 工程质量

- [ ] 缺少单元测试和集成测试
- [ ] 前端所有组件集中在 App.tsx，需拆分
- [ ] 缺少错误边界和全局错误处理
- [ ] 缺少国际化支持
