<div align="center">

# McGuffin 麦高芬

**算法竞赛出题团队协作工具**

![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![Rust](https://img.shields.io/badge/Rust-2021-orange)
![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue)
![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)

<br>

</div>

---

## 📖 什么是 McGuffin？

McGuffin 是一个面向算法竞赛出题团队的协作工具。名字来源于电影术语 "MacGuffin"（麦高芬）—— 推动剧情发展的关键物件。在这里，每一道题目都是推动算法竞赛发展的"麦高芬"。

它帮助出题团队完成以下工作：

- **团队管理** — 成员申请加入、管理员审核、角色分配
- **题目投稿** — 支持 Markdown 格式编写题目，管理员审核发布
- **成果展示** — 展示团队已公开的出题成果

---

## ✨ 核心功能

### 👥 团队管理

- 游客可申请加入团队，填写申请理由
- 管理员可审核（批准/拒绝）入队申请
- 支持多种角色：管理员、成员、游客、待审核用户
- 不同角色拥有不同操作权限

### 📝 题目工作流

- 团队成员可以 Markdown 格式投稿题目
- 题目包含：标题、所属赛事、难度（Easy/Medium/Hard）、内容
- 管理员审核题目：批准后可公开展示，或拒绝退回
- 支持查看所有题目（含待审核）与仅查看已公开题目

### 🏆 成果展示

- 公开展示已通过审核的题目
- 访客无需登录即可浏览团队的出题成果
- 简洁专业的灰白色系界面，清晰呈现题目信息

### 🔐 CP OAuth 登录

- 集成 [CP OAuth](https://www.cpoauth.com/about) 认证服务
- 支持 PKCE 授权码模式，安全便捷
- 首次登录自动创建用户档案

---

## 👥 用户角色

| 角色 | 说明 | 主要权限 |
|------|------|----------|
| **管理员 (Admin)** | 团队核心成员 | 审核入队申请、审核题目、管理团队 |
| **成员 (Member)** | 团队正式成员 | 投稿题目、查看团队、查看内部题目 |
| **游客 (Guest)** | 非团队用户 | 查看成果展示 |
| **待审核 (Pending)** | 已申请但未通过 | 等待管理员审核 |

---

## 🛠️ 技术栈

### 后端

| 技术 | 用途 |
|------|------|
| [Rust](https://www.rust-lang.org/) | 编程语言 |
| [Axum](https://github.com/tokio-rs/axum) | Web 框架 |
| [Tokio](https://tokio.rs/) | 异步运行时 |
| [reqwest](https://docs.rs/reqwest/) | HTTP 客户端（OAuth 通信） |

### 前端

| 技术 | 用途 |
|------|------|
| [React](https://react.dev/) | UI 框架 |
| [TypeScript](https://www.typescriptlang.org/) | 类型安全 |
| [Vite](https://vitejs.dev/) | 构建工具 |
| [Tailwind CSS](https://tailwindcss.com/) | 样式框架 |
| [react-markdown](https://github.com/remarkjs/react-markdown) | Markdown 渲染 |
| [KaTeX](https://katex.org/) | 数学公式渲染 |
| [Bun](https://bun.sh/) | 包管理与运行时 |

---

## 🚀 快速开始

### 前置要求

| 工具 | 版本 |
|------|------|
| Rust | ≥ 1.70 |
| Bun | 最新稳定版 |

### 启动开发环境

```bash
# 克隆项目
git clone https://github.com/your-org/mcguffin.git
cd mcguffin

# 启动后端（终端 1）
cd server
cargo run

# 启动前端（终端 2）
cd web
bun install
bun run dev
```

前端默认运行在 `http://localhost:5173`，后端运行在 `http://localhost:3000`。

### 演示模式

如果尚未配置 CP OAuth，系统支持演示模式登录：

| Token | 角色 |
|-------|------|
| `admin_token` | 管理员 |
| `member_token` | 团队成员 |
| `pending_token` | 待审核用户 |
| `new_user_token` | 游客（新用户） |

---

## 📁 项目结构

```
mcguffin/
├── DEMAND.md          # 需求文档
├── DEVELOPMENT.md     # 开发文档
├── web/               # 前端项目（React + TypeScript + Vite）
│   ├── src/
│   │   ├── App.tsx    # 主应用组件
│   │   ├── main.tsx   # 应用入口
│   │   └── oauthConfig.ts  # OAuth 配置
│   └── package.json
└── server/            # 后端项目（Rust + Axum）
    ├── src/
    │   ├── main.rs    # 服务入口
    │   └── lib.rs     # 核心业务逻辑
    └── Cargo.toml
```

---

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
