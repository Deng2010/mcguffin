<div align="center">

# McGuffin 麦高芬

**算法竞赛出题团队协作工具**

![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![Rust](https://img.shields.io/badge/Rust-2021-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue)
![Build](https://img.shields.io/github/actions/workflow/status/your-org/mcguffin/release.yml?branch=main&label=release)
![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)

<br>

</div>

---

## 📖 什么是 McGuffin？

McGuffin 是一个面向算法竞赛出题团队的协作工具，提供从题目创作、审核发布到成果展示的完整工作流。名字来源于电影术语 "MacGuffin"（麦高芬）—— 推动剧情发展的关键物件。在这里，每一道题目都是推动算法竞赛发展的"麦高芬"。

---

## ✨ 核心功能

### 👥 团队管理

- 游客申请加入 + 管理员审批 + 角色分配（admin/member/guest）
- 成员角色变更与移除（含 superadmin 保护机制）
- 权限组系统：可按成员组批量授予权限

### 📝 题目工作流

- Markdown 题目投稿（支持 LaTeX 公式、代码高亮、Luogu 风格 Callout）
- 三级审核：待审核 → 批准 → 发布
- 题目认领 + 验题人解决方案提交
- 可见性 ACL：可限制题目仅对特定成员可见

### 🏆 赛事管理

- 赛事 CRUD + 状态流转（draft → public）
- 题目排序 + 按赛事分组展示
- 公开展板可配置精选题目和赛事

### 💬 讨论与社区

- 统一帖子系统（讨论 + 公告 + 建议）
- 标签 + 表情反应 + 回复
- 公开社区动态流
- 管理员后台管理标签和表情

### 🔐 权限体系

- 三级权限：角色 → 成员组 → 个人（OR 关系取并集）
- 19 种细粒度权限标识，覆盖所有操作
- 超级管理员不受限（硬编码保护）
- 所有权限可通过配置文件自定义覆盖

### 🛠️ CLI 管理工具

- 跨平台（macOS / Linux / Windows）
- 交互式 `init` 配置向导
- `start` / `stop` / `restart` / `status` 服务管理
- `backup create / list / restore / delete` 数据备份
- `config show / set` 运行时配置查看与修改

---

## 👥 用户角色

| 角色                        | 说明                 | 主要权限             |
| --------------------------- | -------------------- | -------------------- |
| **超级管理员 (Superadmin)** | 系统所有者，不可删除 | 全部权限             |
| **管理员 (Admin)**          | 团队核心成员         | 审核、管理、配置站点 |
| **成员 (Member)**           | 团队正式成员         | 投稿、查看内部资源   |
| **游客 (Guest)**            | 非团队用户           | 浏览公开内容         |
| **待审核 (Pending)**        | 已申请但未通过       | 等待管理员审核       |

---

## 🛠️ 技术栈

### 后端

| 技术                                      | 用途                      |
| ----------------------------------------- | ------------------------- |
| [Rust](https://www.rust-lang.org/)        | 编程语言 (Edition 2021)   |
| [Axum](https://github.com/tokio-rs/axum)  | Web 框架 (0.8)            |
| [Tokio](https://tokio.rs/)                | 异步运行时                |
| [reqwest](https://docs.rs/reqwest/)       | HTTP 客户端（OAuth 通信） |
| [serde](https://serde.rs/)                | 序列化/反序列化           |
| [toml_edit](https://docs.rs/toml_edit/)   | TOML 编辑（配置持久化）   |
| [tower-http](https://docs.rs/tower-http/) | CORS、压缩、静态文件服务  |
| [chrono](https://docs.rs/chrono/)         | 时间处理                  |
| [clap](https://docs.rs/clap/)             | CLI 参数解析              |

### 前端

| 技术                                                         | 用途              |
| ------------------------------------------------------------ | ----------------- |
| [React](https://react.dev/)                                  | UI 框架 (18.x)    |
| [TypeScript](https://www.typescriptlang.org/)                | 类型安全          |
| [Vite](https://vitejs.dev/)                                  | 构建工具 (6.x)    |
| [Tailwind CSS](https://tailwindcss.com/)                     | 样式框架 (3.x)    |
| [react-markdown](https://github.com/remarkjs/react-markdown) | Markdown 渲染     |
| [KaTeX](https://katex.org/)                                  | 数学公式渲染      |
| [react-router-dom](https://reactrouter.com/)                 | 前端路由          |
| [Bun](https://bun.sh/)                                       | 包管理 + 运行环境 |

---

## 🚀 快速开始

### 前置要求

| 工具        | 版本       | 安装                                                              |
| ----------- | ---------- | ----------------------------------------------------------------- |
| Rust        | ≥ 1.70     | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun         | 最新稳定版 | `curl -fsSL https://bun.sh/install \| bash`                       |
| just (可选) | 最新稳定版 | `brew install just` / `cargo install just`                        |

### 开发环境

```bash
# 终端 1：后端
cd server
cargo run
# → http://localhost:3000

# 终端 2：前端
cd web
bun install
bun run dev
# → http://localhost:5173
```

### 生产构建

```bash
# 使用 just（推荐）
just build          # 构建全部
sudo just install   # 安装到 /usr/local
mcguffin init       # 交互式配置向导
mcguffin start      # 启动服务

# 或手动
cd server && cargo build --release
cd web && bun install && bun run build
```

### Docker 部署

#### 前置要求

| 工具           | 安装                                                        |
| -------------- | ----------------------------------------------------------- |
| Docker         | https://docs.docker.com/engine/install/                     |
| Docker Compose | 通常随 Docker Desktop 自带，或 `pip install docker-compose` |

#### 快速启动

```bash
# 方式一：从 ghcr 一键运行（无需克隆、无需构建）
docker run -d \
  --name mcguffin \
  -p 3000:3000 \
  -e SITE_URL=http://localhost:3000 \
  -e ADMIN_PASSWORD=change_me \
  -v mcguffin_data:/app/data \
  ghcr.io/deng2010/mcguffin
```

```bash
# 方式二：docker-compose（推荐，构建 + 启动一步完成）
git clone https://github.com/Deng2010/mcguffin.git
cd mcguffin
docker compose up -d
# → http://localhost:3000
```

```bash
# 方式三：本地构建后运行
 docker build -t mcguffin .
 docker run -d \
   --name mcguffin \
   -p 3000:3000 \
   -e SITE_URL=http://localhost:3000 \
   -e ADMIN_PASSWORD=change_me \
   -v mcguffin_data:/app/data \
   mcguffin
```

#### 配置方式

Docker 容器优先读取环境变量，也支持挂载自定义配置文件：

```yaml
# docker-compose.yml
services:
  mcguffin:
    environment:
      - SITE_URL=https://mcguffin.example.com
      - ADMIN_PASSWORD=your_password
      - SITE_NAME=My Team
      - TZ=Asia/Shanghai
    volumes:
      # 方式 A：持久化数据卷（自动生成配置）
      - mcguffin_data:/app/data
      # 方式 B：挂载自定义配置文件（覆盖自动生成）
      - ./my-config.toml:/app/data/config.toml
```

#### 管理操作

```bash
# 查看运行状态
docker compose ps

# 查看日志
docker compose logs -f

# 容器内执行 CLI 命令（备份、配置查看等）
docker exec mcguffin /app/mcguffin status
docker exec mcguffin /app/mcguffin backup create

# 健康检查
curl http://localhost:3000/api/health
# → {"status":"ok","version":"0.2.2"}
```

> **注意**：数据存储在 Docker volume `mcguffin_data` 中（映射到容器的 `/app/data`），
> 包含 SQLite 数据库文件和配置文件。删除容器不会丢失数据，但删除 volume 会导致数据丢失。

### CLI 命令

```bash
mcguffin init       # 交互式生成配置文件
mcguffin start      # 启动后台服务
mcguffin stop       # 停止服务
mcguffin status     # 查看服务状态
mcguffin config     # 查看/修改配置
mcguffin backup     # 数据备份管理
```

---

## 📁 项目结构

```
mcguffin/
├── justfile              # 构建命令（just）
├── AGENTS.md             # AI 编程助手指南
├── GUIDE.md              # 开发者指南
├── README.md             # 项目主页
├── Dockerfile            # Docker 构建
├── docker-compose.yml    # Docker Compose
├── docker-entrypoint.sh  # 容器入口
├── web/                  # 前端 SPA（React + Vite + Tailwind）
│   ├── src/
│   │   ├── App.tsx       # 路由 + 布局 + Context 提供者
│   │   ├── AuthContext.tsx  # 认证状态管理
│   │   ├── SiteContext.tsx  # 站点信息
│   │   ├── DarkModeContext.tsx # 暗色模式
│   │   ├── NotificationContext.tsx # 通知轮询
│   │   ├── api.ts        # API 客户端封装
│   │   ├── types.ts      # TypeScript 类型定义
│   │   ├── components/   # 9 个可复用组件
│   │   ├── pages/        # 24 个页面组件
│   │   ├── hooks/        # 自定义 Hooks
│   │   ├── utils/        # 工具函数
│   │   └── test/         # 前端测试
│   └── ...config files
├── server/               # 后端（Rust + Axum）
│   ├── Cargo.toml
│   ├── migrations/       # SQLite 迁移
│   ├── src/
│   │   ├── main.rs       # 服务入口，路由注册
│   │   ├── lib.rs        # 模块导出
│   │   ├── types.rs      # 数据模型 + 20 种权限常量
│   │   ├── state.rs      # AppState + SQLite 持久化 + 配置
│   │   ├── db.rs         # SQLite 初始化 + 数据导入/导出/备份
│   │   ├── auth.rs       # OAuth + 管理员登录
│   │   ├── user.rs       # 用户管理
│   │   ├── team.rs       # 团队管理 + 权限组
│   │   ├── problems.rs   # 题目 CRUD + 审核 + 认领
│   │   ├── contests.rs   # 赛事管理
│   │   ├── discussions.rs # 统一帖子系统
│   │   ├── community.rs  # 社区动态
│   │   ├── suggestions.rs # 建议系统
│   │   ├── announcements.rs # 公告系统
│   │   ├── notifications.rs # 通知系统
│   │   ├── info.rs       # 站点信息
│   │   ├── admin.rs      # 管理后台 API
│   │   ├── pages.rs      # SSR 页面（legacy）
│   │   ├── utils.rs      # 认证工具函数
│   │   └── bin/mcguffin.rs # CLI 工具
│   └── tests/api.rs      # 集成测试
└── .github/workflows/
    ├── test.yml          # PR/Push 测试
    ├── release.yml       # Tag 触发发布
    └── docker.yml        # Docker 构建
```

---

## 📚 文档索引

| 文档                   | 适用对象    | 说明                           |
| ---------------------- | ----------- | ------------------------------ |
| [AGENTS.md](AGENTS.md) | AI 编程助手 | 代码库结构、开发约定与操作指南 |
| [GUIDE.md](GUIDE.md)   | 人类开发者  | 技术架构、API 文档、最佳实践   |

|---

## 📬 反馈与建议

如有问题或建议，欢迎提交 [Issues](https://github.com/your-org/mcguffin/issues) 或 Pull Requests。

---

## 📄 开源协议

本项目基于 [Apache License 2.0](LICENSE) 开源。

```
Copyright 2026 LBA OI Team

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
