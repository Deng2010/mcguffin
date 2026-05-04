<div align="center">

# McGuffin 麦高芬 🎯

**算法竞赛出题团队协作平台**

![License](https://img.shields.io/badge/license-Apache%202.0-blue)
![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange)
![Node](https://img.shields.io/badge/Node-20%2B-green)
![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen)

<br>

</div>

---

## 📖 什么是 McGuffin？

McGuffin 是一个面向算法竞赛出题团队的协作平台。它帮助团队管理成员、提交与审核题目、展示成果，同时支持团队内部流转与公开分享。

> 名字来源于电影术语 "MacGuffin"（麦高芬）—— 推动剧情发展的关键物件。在这里，每一道题目都是推动算法竞赛发展的"麦高芬"。

### ✨ 核心功能

- **👥 团队管理** — 申请加入、角色审批、成员管理（成员/管理员/SuperAdmin）
- **📝 题目工作流** — 提交 → 审核 → 发布，支持认领验题
- **🏆 比赛管理** — 创建比赛、编排题目顺序、控制公开/草稿状态
- **🔐 权限体系** — 游客 → 成员 → 管理员 → SuperAdmin，精细控制
- **🔑 CP OAuth 登录** — 集成 CP OAuth，一键登录
- **🔧 CLI 管理工具** — 配置管理、数据备份、服务控制
- **🛡️ 自动备份** — 定时备份、一键恢复
- **📦 跨平台发布** — CI 自动构建 Windows/macOS/Linux 发行包

---

## 🚀 快速部署

### 前置要求

| 工具 | 版本 | 用途 |
|------|------|------|
| Rust | ≥ 1.80 | 编译后端服务 |
| Node.js | ≥ 20 | 构建前端 SPA |
| npm | ≥ 10 | 前端依赖管理 |
| systemd | — | 服务管理（Linux） |

### 从源码构建

```bash
# 1. 克隆项目
git clone https://github.com/your-org/mcguffin.git
cd mcguffin

# 2. 构建后端（服务 + CLI）
cd server
cargo build --release
# 产物: target/release/mcguffin-server, target/release/mcguffin

# 3. 构建前端
cd ../web
npm ci
npm run build
# 产物: dist/

# 4. 生成配置文件
cd ../server
./target/release/mcguffin init
```

### 配置

配置文件位于 `/usr/share/mcguffin/config.toml`：

```toml
[server]
site_url = "https://your-domain.com"
port = 3000
data_file = "mcguffin_data.json"

[admin]
password = "your-admin-password"
display_name = "管理员"

[site]
name = "McGuffin"

[oauth]
cp_client_id = "your-cp-oauth-client-id"
cp_client_secret = "your-cp-oauth-client-secret"

[difficulty.Easy]
label = "简单"
color = "#22c55e"

[difficulty.Medium]
label = "中等"
color = "#f59e0b"

[difficulty.Hard]
label = "困难"
color = "#ef4444"
```

### 启动服务

```bash
# 使用 CLI 启动
sudo mcguffin start
sudo mcguffin status

# 或直接运行
./target/release/mcguffin-server
```

### 反向代理（Nginx）

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

> **提示**：生产环境建议配置 HTTPS（Let's Encrypt），否则 OAuth 登录会因浏览器安全策略失败。

---

## 🛠️ CLI 管理工具

McGuffin 内置了一个命令行工具 `mcguffin`，用于日常管理：

### 配置管理

```bash
# 查看当前配置
mcguffin config show

# 修改配置项
mcguffin config set server.site_url https://example.com
mcguffin config set admin.password newpass123
mcguffin config set server.port 8080
mcguffin config set oauth.cp_client_id your_client_id

# 注意：修改后需重启服务生效
```

### 数据备份

```bash
# 创建备份
mcguffin backup create

# 列出所有备份
mcguffin backup list

# 恢复指定备份
mcguffin backup restore mcguffin_data_20250101_120000.json

# 删除备份
mcguffin backup delete mcguffin_data_20250101_120000.json
```

### 服务控制

```bash
mcguffin start      # 启动服务
mcguffin stop       # 停止服务
mcguffin restart    # 重启服务
mcguffin status     # 查看状态
```

---

## 💻 技术栈

### 后端

| 技术 | 用途 |
|------|------|
| [Rust](https://www.rust-lang.org/) | 编程语言 |
| [Axum](https://github.com/tokio-rs/axum) | Web 框架 |
| [Tokio](https://tokio.rs/) | 异步运行时 |
| [Serde](https://serde.rs/) | 序列化 |
| [reqwest](https://docs.rs/reqwest/) | HTTP 客户端（OAuth） |
| [toml](https://docs.rs/toml/) / [toml_edit](https://docs.rs/toml_edit/) | 配置解析/编辑 |

### 前端

| 技术 | 用途 |
|------|------|
| [React](https://react.dev/) | UI 框架 |
| [TypeScript](https://www.typescriptlang.org/) | 类型安全 |
| [Vite](https://vitejs.dev/) | 构建工具 |
| [React Router](https://reactrouter.com/) | 客户端路由 |
| [Tailwind CSS](https://tailwindcss.com/) | 样式 |
| [KaTeX](https://katex.org/) | 数学公式渲染 |
| [react-markdown](https://remark.js.org/) | Markdown 渲染 |

### 测试

| 技术 | 用途 |
|------|------|
| [Vitest](https://vitest.dev/) | 前端测试框架 |
| [Testing Library](https://testing-library.com/) | 组件测试 |
| [cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html) | 后端测试（单元 + 集成） |

---

## 🧪 本地开发

### 启动开发服务器

```bash
# 终端 1：启动后端（支持热重载）
cd server
cargo watch -x run

# 终端 2：启动前端开发服务器（Vite HMR）
cd web
npm run dev
# 前端在 http://localhost:5173，自动代理 /api 到后端
```

### 运行测试

```bash
# 后端全部测试
cd server && cargo test

# 后端只跑单元测试（更快）
cd server && cargo test --lib

# 后端只跑集成测试
cd server && cargo test --test api

# 前端测试
cd web && npm test

# 前端测试（监听模式）
cd web && npm run test:watch
```

---

## 🔄 CI/CD

| 工作流 | 触发条件 | 内容 |
|--------|----------|------|
| **Test** (`.github/workflows/test.yml`) | 每次 push / PR | 三平台前端构建 + 全部测试 |
| **Build & Release** (`.github/workflows/release.yml`) | 打 `v*.*.*` 标签 | 测试 → 构建 → 打包 → 上传 Releases |

### 创建发布版本

```bash
git tag v0.1.0
git push origin v0.1.0
```

会自动在 GitHub Releases 生成：

| 平台 | 文件 |
|------|------|
| Linux | `mcguffin-v0.1.0-x86_64-unknown-linux-gnu.tar.gz` |
| macOS | `mcguffin-v0.1.0-x86_64-apple-darwin.tar.gz` |
| Windows | `mcguffin-v0.1.0-x86_64-pc-windows-msvc.zip` |

每个压缩包包含：`mcguffin-server` + `mcguffin` CLI + `web-dist/` 前端静态文件。

---

## 📁 项目结构

```
mcguffin/
├── .github/workflows/
│   ├── release.yml     # 发布工作流
│   └── test.yml        # PR/push 测试工作流
├── server/             # 后端 Rust 项目
│   ├── src/
│   │   ├── main.rs     # 入口 + 路由注册
│   │   ├── lib.rs      # 库入口（重导出所有模块）
│   │   ├── state.rs    # AppState + 配置加载 + 持久化
│   │   ├── types.rs    # 全部数据类型定义
│   │   ├── auth.rs     # OAuth 认证
│   │   ├── user.rs     # 用户管理
│   │   ├── team.rs     # 团队管理
│   │   ├── problems.rs # 题目 CRUD
│   │   ├── contests.rs # 比赛管理
│   │   ├── admin.rs    # 管理员接口
│   │   ├── info.rs     # 站点信息
│   │   ├── pages.rs    # 服务端渲染页面
│   │   ├── utils.rs    # 工具函数
│   │   └── bin/
│   │       └── mcguffin.rs  # CLI 工具
│   ├── tests/
│   │   └── api.rs      # 集成测试
│   └── Cargo.toml
├── web/                # 前端 React 项目
│   ├── src/
│   │   ├── main.tsx    # 入口
│   │   ├── App.tsx     # 路由 + 布局
│   │   ├── api.ts      # API 客户端
│   │   ├── types.ts    # 类型定义
│   │   ├── AuthContext.tsx  # 认证上下文
│   │   ├── hooks/      # 自定义 Hooks
│   │   ├── pages/      # 页面组件
│   │   └── test/       # 测试文件
│   ├── package.json
│   └── vite.config.ts
└── LICENSE
```

---

## 👥 用户角色

| 角色 | 权限 |
|------|------|
| **SuperAdmin** | 全部权限，包括：配置修改、服务重启、备份管理、数据导出 |
| **管理员 (Admin)** | 题目审核、比赛编辑、团队成员管理（**看不到 SuperAdmin**） |
| **成员 (Member)** | 查看题目、提交题目、认领验题、查看团队 |
| **游客 (Guest)** | 查看 Showcase、申请加入团队 |

---

## 📬 联系方式

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
