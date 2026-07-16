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

## 0. 关于本项目

McGuffin 是一个面向**算法竞赛出题团队**的协作工具，提供从题目创作、审核发布到成果展示的完整工作流。

项目名称来源于电影术语 "MacGuffin"（麦高芬）—— 推动剧情发展的关键物件。在这里，每一道题目都是推动算法竞赛发展的"麦高芬"。

主要功能包含：

- 团队管理
- 题目和赛事工作流
- 社区讨论
- 较细粒度的权限管理
- 等等……

---

## 1. 技术栈

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

### 前端

| 技术                                                         | 用途              |
| ------------------------------------------------------------ | ----------------- |
| [React](https://react.dev/)                                  | UI 框架 (18.x)    |
| [TypeScript](https://www.typescriptlang.org/)                | 类型安全          |
| [Tailwind CSS](https://tailwindcss.com/)                     | 样式框架 (3.x)    |
| [react-markdown](https://github.com/remarkjs/react-markdown) | Markdown 渲染     |
| [KaTeX](https://katex.org/)                                  | 数学公式渲染      |
| [react-router-dom](https://reactrouter.com/)                 | 前端路由          |
| [Bun](https://bun.sh/)                                       | 包管理 + 运行环境 |

---

## 2. 安装部署

### 2.1 容器化部署（推荐）

最简单无脑，强烈推荐。

#### 2.1.1 Docker 一行命令部署（最快）

一行命令启动，无需克隆仓库：

```bash
docker run -d \
  --name mcguffin \
  -p 3000:3000 \
  -e SITE_URL=http://localhost:3000 \
  -e ADMIN_PASSWORD=change_me \ # 记得改
  -v mcguffin_data:/app/data \
  ghcr.io/deng2010/mcguffin
```

#### 2.1.2 Apple Container 部署

在 macOS 上，一种更 Apple 的启动方式是使用 [Apple Container](https://github.com/apple/container)。

```bash
container run --rm -d \ # 如果不需要在容器停下后删除镜像，可以把 --rm 去掉
  --name mcguffin \
  -p 3000:3000 \
  -e SITE_URL=http://localhost:3000 \
  -e ADMIN_PASSWORD=change_me \ # 记得改
  -v mcguffin_data:/app/data \
  ghcr.io/deng2010/mcguffin:latest-linux-arm64
```

#### 2.1.3：Docker Compose

更加可定制的部署方法，仓库中已写好 `docker-compose.yaml`。

```bash
git clone https://github.com/Deng2010/mcguffin.git
cd mcguffin
docker compose up -d
# → http://localhost:3000
```

#### 2.1.4 配置方式

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
      - mcguffin_data:/app/data # 持久化数据卷
      - ./my-config.toml:/app/data/config.toml # 自定义配置（可选）
```

#### 2.1.5 管理操作

```bash
# 查看运行状态
docker compose ps

# 查看日志
docker compose logs -f

# 容器内执行 CLI 命令
docker exec mcguffin /app/mcguffin status
docker exec mcguffin /app/mcguffin backup create

# 健康检查
curl http://localhost:3000/api/health
# → {"status":"ok","version":"0.3.0"}
```

> **注意**：数据存储在 Docker volume `mcguffin_data` 中（映射到容器的 `/app/data`），
> 包含 SQLite 数据库文件和配置文件。删除容器不会丢失数据，但删除 volume 会导致数据丢失。

---

### 2.2 源码构建（需要 Rust + Bun）

如果需要二次开发或自定义构建，可以使用源码方式。

#### 2.2.1 前置要求

| 工具        | 版本       | 安装                                                              |
| ----------- | ---------- | ----------------------------------------------------------------- |
| Rust        | ≥ 1.70     | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun         | 最新稳定版 | `curl -fsSL https://bun.sh/install \| bash`                       |
| just (可选) | 最新稳定版 | `brew install just` / `cargo install just`                        |

#### 2.2.2 开发环境

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

#### 2.2.3 生产构建

```bash
# 使用 just（推荐）
just build          # 构建全部
sudo just install   # 安装到 /usr/local
mcguffin init       # 交互式配置向导
mcguffin start      # 启动服务

# 或手动构建
cd server && cargo build --release
cd web && bun install && bun run build
```

#### 2.2.4 使用 McGuffin CLI 管理服务端

```bash
mcguffin -h         # 查看 CLI 使用帮助
mcguffin init       # 交互式生成配置文件
mcguffin start      # 启动后台服务
mcguffin stop       # 停止服务
mcguffin status     # 查看服务状态
mcguffin config     # 查看/修改配置
mcguffin backup     # 数据备份管理
```

---

## 3 文档索引

| 文档                                         | 适用对象    | 说明                             |
| -------------------------------------------- | ----------- | -------------------------------- |
| [📖 文档首页](docs/README.md)                | 所有人      | 完整文档索引，含部署/管理/使用   |
| [📦 快速部署](docs/guide/quick-start.md)     | 运维 / 站长 | Docker 一键启动、源码编译安装    |
| [⚙️ 配置详解](docs/guide/configuration.md)   | 运维 / 站长 | 配置项说明、环境变量、运行时修改 |
| [🛠️ 开发环境搭建](docs/guide/development.md) | 开发者      | Rust + Bun 环境、本地开发        |
| [🚢 生产部署](docs/guide/deployment.md)      | 运维 / 站长 | 反向代理、HTTPS、系统服务        |
| [👥 管理后台总览](docs/admin/overview.md)    | 管理员      | 后台功能概览、导航说明           |
| [🔐 用户与权限管理](docs/admin/users.md)     | 管理员      | 角色、权限组、成员管理           |
| [🏆 赛事管理](docs/admin/contests.md)        | 管理员      | 赛事创建、状态流转、题目编排     |
| [💾 备份与恢复](docs/admin/backups.md)       | 管理员      | 自动备份、手动操作、导入导出     |
| [🚀 新手上路](docs/user/getting-started.md)  | 普通用户    | 注册登录、团队申请、基本操作     |
| [📝 题目系统](docs/user/problems.md)         | 出题人      | 投稿、审核流程、题目认领         |
| [💬 社区讨论](docs/user/community.md)        | 全体用户    | 帖子、标签、社区动态             |
| [AGENTS.md](AGENTS.md)                       | AI 编程助手 | 代码库结构、开发约定与操作指南   |
| [GUIDE.md](GUIDE.md)                         | 人类开发者  | 技术架构、API 文档、最佳实践     |

## 4 反馈与建议

如有问题或建议，欢迎提交 [Issues](https://github.com/your-org/mcguffin/issues) 或 Pull Requests。

---

## 5 开源协议

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
