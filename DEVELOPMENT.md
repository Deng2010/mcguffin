# McGuffin 项目开发文档

## 1. 项目概述

**McGuffin** 是一个面向算法竞赛出题团队的协作工具，旨在为出题团队提供成员管理、题目投稿与审核、成果展示等核心功能。系统支持通过 CP OAuth 进行用户认证，实现不同角色的权限控制。

- 项目名称：McGuffin（麦高芬）
- 项目定位：算法竞赛出题团队工具
- 当前版本：0.1.0（前端 0.0.1）

---

## 2. 技术架构

### 2.1 整体架构

项目采用前后端分离架构：

```
浏览器 ──→ web (React SPA) ──→ server (Rust/Axum) ──→ CP OAuth
```

### 2.2 前端技术栈

| 技术        | 版本              | 用途              |
|------------|-------------------|-------------------|
| React      | ^18.3.1           | UI 框架           |
| React DOM  | ^18.3.1           | DOM 渲染          |
| React Router DOM | ^6.28.0     | 客户端路由        |
| TypeScript | ~5.6.2            | 类型安全          |
| Vite       | ^6.0.5            | 构建工具          |
| Tailwind CSS | ^3.4.17        | 原子化 CSS 样式    |
| Bun        | -                 | 包管理与运行时     |

### 2.3 后端技术栈

| 技术              | 版本   | 用途                     |
|-------------------|--------|--------------------------|
| Rust              | 2021   | 编程语言                 |
| Axum              | 0.7    | Web 框架                 |
| Tokio             | 1      | 异步运行时 (full features)|
| Serde / Serde_json| 1      | 序列化/反序列化          |
| Tower / Tower-HTTP| 0.4/0.5| 中间件 (CORS 等)         |
| Tracing           | 0.1    | 日志                     |
| UUID              | 1      | 唯一 ID 生成             |
| Chrono            | 0.4    | 时间处理                 |
| Reqwest           | 0.12   | HTTP 客户端 (OAuth 通信) |
| SHA-2             | 0.10   | 哈希 (PKCE 支持)         |
| Base64            | 0.22   | 编码                     |

### 2.4 外部服务

| 服务       | 地址                        | 用途                  |
|-----------|-----------------------------|-----------------------|
| CP OAuth  | https://www.cpoauth.com     | 第三方 OAuth 认证服务  |

---

## 3. 项目结构

```
mcguffin/
├── DEMAND.md                  # 需求文档
├── DEVELOPMENT.md             # 开发文档（本文件）
├── web/              # 前端项目
│   ├── index.html             # HTML 入口
│   ├── package.json           # 依赖配置
│   ├── bun.lock               # Bun 锁文件
│   ├── vite.config.ts         # Vite 配置
│   ├── tailwind.config.js     # Tailwind 配置
│   ├── tsconfig.json          # TypeScript 配置
│   ├── dist/                  # 构建产物
│   └── src/
│       ├── App.tsx            # 主应用组件 (含所有页面与逻辑)
│       ├── oauthConfig.ts     # OAuth 配置与工具函数
│       └── main.tsx           # 应用入口
│
└── server/           # 后端项目
    ├── Cargo.toml             # Rust 依赖配置
    └── src/
        ├── main.rs            # 服务入口 (路由注册, 启动配置)
        └── lib.rs             # 核心业务逻辑 (数据模型, API 处理函数)
```

---

## 4. 数据模型

### 4.1 User (用户)

| 字段         | 类型             | 说明                              |
|-------------|------------------|-----------------------------------|
| id          | String           | 用户唯一标识 (来自 CP OAuth sub)  |
| username    | String           | 用户名                            |
| display_name| String           | 显示名称                          |
| avatar_url  | Option\<String\> | 头像 URL                          |
| email       | Option\<String\> | 邮箱                              |
| role        | String           | 角色: admin / member / guest / pending |
| team_status | String           | 团队状态: none / pending / joined |
| created_at  | DateTime\<Utc\>  | 创建时间                          |

### 4.2 TeamMember (团队成员)

| 字段       | 类型   | 说明                            |
|-----------|--------|---------------------------------|
| id        | String | 成员记录 ID                     |
| user_id   | String | 关联用户 ID                     |
| name      | String | 成员名称                        |
| avatar    | String | 头像标识                        |
| role      | String | 角色: admin / member            |
| joined_at | String | 加入日期 (YYYY-MM-DD)           |

### 4.3 Problem (题目)

| 字段        | 类型              | 说明                                 |
|------------|-------------------|--------------------------------------|
| id         | String            | 题目唯一标识                         |
| title      | String            | 题目标题                             |
| author_id  | String            | 作者用户 ID                          |
| author_name| String            | 作者显示名                           |
| contest    | String            | 所属赛事 / 比赛                      |
| difficulty | String            | 难度: Easy / Medium / Hard           |
| content    | String            | 题目内容 (Markdown 格式)             |
| status     | String            | 状态: pending / approved / rejected  |
| created_at | DateTime\<Utc\>   | 创建时间                             |
| public_at  | Option\<DateTime\<Utc\>\> | 公开时间 (审核通过时设置)     |

### 4.4 JoinRequest (入队申请)

| 字段        | 类型             | 说明                           |
|------------|------------------|--------------------------------|
| id         | String           | 申请 ID                        |
| user_id    | String           | 申请人用户 ID                   |
| user_name  | String           | 申请人名称                      |
| user_email | String           | 申请人邮箱                      |
| reason     | String           | 申请理由                       |
| status     | String           | 状态: pending / approved / rejected |
| created_at | DateTime\<Utc\>  | 申请时间                       |

---

## 5. API 接口文档

后端服务监听 `0.0.0.0:3000`。

### 5.1 页面路由

| 方法 | 路径       | 说明                       |
|------|-----------|----------------------------|
| GET  | /         | 根路径，重定向到 /portfolio |
| GET  | /login    | 登录页面 (服务端渲染 HTML)  |
| GET  | /portfolio| 成果展示页面 (服务端渲染)   |

### 5.2 OAuth 认证接口

| 方法 | 路径                      | 说明                                      |
|------|--------------------------|-------------------------------------------|
| GET  | /api/oauth/authorize     | 发起 OAuth 授权，重定向到 CP OAuth         |
| GET  | /api/oauth/callback      | OAuth 回调，交换 token 并创建会话          |
| POST | /api/oauth/token         | 使用 refresh_token 刷新 access_token       |

**OAuth 授权流程：**
1. 前端/后端将用户重定向至 `/api/oauth/authorize`
2. 后端构建 CP OAuth 授权 URL 并重定向（含 PKCE code_challenge）
3. 用户在 CP OAuth 完成授权后回调至 `/api/oauth/callback?code=xxx`
4. 后端使用 code 向 CP OAuth 交换 access_token
5. 后端使用 access_token 获取用户信息
6. 创建本地会话，将 session_token 通过重定向传给前端

### 5.3 用户接口

| 方法 | 路径             | 说明                           |
|------|-----------------|--------------------------------|
| GET  | /api/user/me    | 获取当前登录用户信息 (Bearer Token) |
| GET  | /api/user/verify| 验证 token 有效性              |

**请求头：** `Authorization: Bearer <session_token>`

### 5.4 团队接口

| 方法 | 路径                                  | 说明                          |
|------|--------------------------------------|-------------------------------|
| GET  | /api/team/members                    | 获取团队成员列表               |
| GET  | /api/team/requests                   | 获取待处理入队申请             |
| POST | /api/team/apply                      | 申请加入团队                   |
| POST | /api/team/review/:request_id/:action | 审核入队申请 (action: approve/reject) |

**申请加入请求体：**
```json
{ "reason": "申请理由" }
```

**权限要求：** 审核（approve/reject）需要 admin 角色。

### 5.5 题目接口

| 方法 | 路径                                  | 说明                          |
|------|--------------------------------------|-------------------------------|
| GET  | /api/problems                        | 获取题目列表 (?all=true 返回全部) |
| POST | /api/problems                        | 投稿题目                       |
| POST | /api/problems/review/:problem_id/:action | 审核题目 (action: approve/reject) |

**投稿题目请求体：**
```json
{
  "title": "题目标题",
  "contest": "赛事名称",
  "difficulty": "Easy|Medium|Hard",
  "content": "Markdown 格式的题目内容"
}
```

**权限要求：** 投稿需要团队成员身份；审核需要 admin 角色。

### 5.6 登出接口

| 方法 | 路径          | 说明                  |
|------|--------------|-----------------------|
| POST | /api/logout  | 登出，销毁当前会话    |

---

## 6. 前端页面与路由

### 6.1 路由表

| 路径                          | 组件                 | 权限要求              |
|-------------------------------|---------------------|-----------------------|
| /                             | HomePage (→/portfolio) | 无                   |
| /login                        | LoginPage           | 无                    |
| /portfolio                    | PortfolioPage       | 无 (公开)             |
| /oauth/cpoauth/authorize      | OAuthAuthorizePage  | 无 (模拟授权页)       |
| /oauth/cpoauth/callback       | → /portfolio        | 无 (回调处理)         |
| /team                         | TeamPage            | view_team             |
| /add                          | AddProblemPage      | submit_problem        |
| /apply                        | ApplyPage           | apply_join            |

### 6.2 权限体系

前端定义了 7 种权限：

| 权限              | 说明                  | admin | member | guest | pending |
|-------------------|----------------------|-------|--------|-------|---------|
| view_portfolio    | 查看成果展示          | Y     | Y      | Y     | Y       |
| apply_join        | 申请加入团队          | -     | Y      | Y     | Y       |
| view_team         | 查看团队成员          | Y     | Y      | -     | -       |
| manage_team       | 团队管理（审核/踢人） | Y     | -      | -     | -       |
| submit_problem    | 投稿题目              | Y     | Y      | -     | -       |
| view_all_problems | 查看所有题目(含待审核)| Y     | Y      | -     | -       |
| approve_problem   | 审核题目              | Y     | -      | -     | -       |

未登录用户仅拥有 `view_portfolio` 权限。

### 6.3 认证流程

1. 用户点击"通过 CP OAuth 登录"
2. 前端生成 CSRF state，存储在 sessionStorage
3. 重定向至 CP OAuth 授权页面（或后端 /api/oauth/authorize）
4. 用户授权后回调至前端 /oauth/cpoauth/callback
5. 前端使用 code 换取 token，用 token 获取用户信息
6. 如果 OAuth 请求失败，回退到演示模式（模拟登录）
7. 用户信息存入 localStorage，权限根据团队成员列表判定

**演示模式账号：**
- admin_token → 张三 (管理员)
- member_token → 赵六 (成员)
- pending_token → 申请者 (待审核)
- new_user_token → 新用户 (游客)

---

## 7. 开发指南

### 7.1 环境准备

**前端：**
```bash
# 安装 Bun (如未安装)
curl -fsSL https://bun.sh/install | bash

# 进入前端目录
cd web

# 安装依赖
bun install

# 启动开发服务器
bun run dev
```

**后端：**
```bash
# 安装 Rust (如未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 进入后端目录
cd server

# 开发模式运行
cargo run

# Release 构建
cargo build --release
```

### 7.2 构建与部署

**前端构建：**
```bash
cd web
bun run build    # 产物输出到 dist/
bun run preview  # 本地预览构建产物
```

**后端构建：**
```bash
cd server
cargo build --release  # 产物在 target/release/mcguffin-server
```

### 7.3 环境变量

**前端 (web)：**

| 变量                           | 默认值                    | 说明                  |
|-------------------------------|---------------------------|-----------------------|
| VITE_CPOAUTH_BASE_URL        | https://cpoauth.com       | CP OAuth 服务地址      |
| VITE_CPOAUTH_CLIENT_ID       | your-client-id            | 应用 Client ID        |
| VITE_CPOAUTH_CLIENT_SECRET   | your-client-secret        | 应用 Client Secret    |
| VITE_CPOAUTH_REDIRECT_URI    | {origin}/oauth/cpoauth/callback | 回调地址        |

**后端 (server)：**

当前 OAuth 配置硬编码在 `AppState::new()` 中，建议后续迁移到环境变量：
- cpoauth_client_id: "mcguffin_app"
- cpoauth_client_secret: "your_client_secret_here"
- cpoauth_redirect_uri: "http://localhost:3000/api/oauth/callback"

---

## 8. 数据存储

**当前状态：** 全部使用内存存储 (HashMap + Arc\<RwLock\>)，服务重启后数据丢失。

| 存储项          | 数据结构                        | 说明         |
|----------------|---------------------------------|-------------|
| users          | HashMap\<String, User\>        | 用户信息     |
| sessions       | HashMap\<String, String\>      | session_token → user_id |
| refresh_tokens | HashMap\<String, String\>      | refresh_token → user_id |
| team_members   | HashMap\<String, TeamMember\>  | 团队成员     |
| problems       | HashMap\<String, Problem\>     | 题目列表     |
| join_requests  | HashMap\<String, JoinRequest\> | 入队申请     |

**预置数据：**
- 3 个团队成员：张三 (admin)、李四 (member)、王五 (member)
- 3 个已审核题目：二叉树的最大深度 (Easy)、回文子串 (Medium)、最优括号匹配 (Hard)

---

## 9. UI 设计规范

根据 DEMAND.md 的设计要求：

- **主色调：** 灰白色系
- **辅助色：** 低饱和度素色，避免过多渐变色
- **圆角策略：** 使用直角（border-radius: 0），不使用圆角
- **风格特征：** 简洁、专业、中性

前端使用 Tailwind CSS 实现，主要使用 gray 色阶体系。

---

## 10. 已知限制与待改进项

### 10.1 安全性

- [ ] Client Secret 硬编码在前端和后端代码中，需迁移至环境变量
- [ ] Token 存储在 localStorage 中，存在 XSS 风险，建议使用 HttpOnly Cookie
- [ ] CORS 配置为允许所有来源 (Any)，生产环境需限制为前端域名
- [ ] 题目投稿接口应增加后端权限校验（目前仅前端限制）
- [ ] 缺少 CSRF 防护（部分接口使用 GET 进行状态修改）

### 10.2 数据持久化

- [ ] 所有数据存储在内存中，服务重启即丢失，需接入数据库
- [ ] 建议使用 PostgreSQL 或 SQLite 作为持久化方案

### 10.3 功能完善

- [ ] 前端登录页面使用模拟 OAuth 流程，需接入真实 CP OAuth
- [ ] 前端部分页面仍在使用 mock 数据，需对接后端 API
- [ ] 缺少 Markdown 渲染功能（题目内容以 Markdown 保存但未渲染）
- [ ] 缺少题目详情查看页面
- [ ] 缺少用户个人资料页面
- [ ] 缺少非成员投稿功能（DEMAND.md 要求支持非成员投稿）
- [ ] 踢出成员的后端 API 尚未实现

### 10.4 工程质量

- [ ] 缺少单元测试和集成测试
- [ ] 前端所有组件集中在 App.tsx 单文件中，需拆分
- [ ] 缺少错误边界和全局错误处理
- [ ] 缺少国际化支持
- [ ] 缺少日志收集与监控

---

## 11. API 请求/响应示例

### 获取当前用户

```
GET /api/user/me
Authorization: Bearer <session_token>

Response 200:
{
  "id": "1",
  "username": "zhangsan",
  "display_name": "张三",
  "avatar_url": null,
  "email": "zhangsan@example.com",
  "role": "admin",
  "team_status": "joined",
  "created_at": "2024-01-15T08:00:00Z"
}
```

### 获取题目列表

```
GET /api/problems
GET /api/problems?all=true

Response 200:
[
  {
    "id": "1",
    "title": "二叉树的最大深度",
    "author_id": "1",
    "author_name": "张三",
    "contest": "LeetCode周赛",
    "difficulty": "Easy",
    "content": "# 二叉树的最大深度\n\n给定一个二叉树，找出其最大深度...",
    "status": "approved",
    "created_at": "2024-06-01T00:00:00Z",
    "public_at": "2024-06-01T00:00:00Z"
  }
]
```

### 投稿题目

```
POST /api/problems
Authorization: Bearer <session_token>
Content-Type: application/json

{
  "title": "新题目",
  "contest": "Codeforces Round",
  "difficulty": "Medium",
  "content": "# 题目描述\n\n内容..."
}

Response 200:
{
  "success": true,
  "message": "提交成功，等待审核",
  "problem_id": null
}
```

### 审核入队申请

```
POST /api/team/review/<request_id>/approve
POST /api/team/review/<request_id>/reject
Authorization: Bearer <session_token>

Response 200:
{
  "success": true,
  "message": "已批准申请"
}
```

---

## 12. 版本历史

| 版本    | 日期       | 变更说明                 |
|---------|-----------|--------------------------|
| 0.1.0   | -         | 初始版本，核心功能原型    |
| 0.0.1   | -         | 前端初始版本             |
