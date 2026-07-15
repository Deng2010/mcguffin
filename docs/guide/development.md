# 🛠️ 开发环境搭建

## 依赖安装

| 工具 | 版本要求 | 安装 |
|------|----------|------|
| Rust | ≥ 1.70 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun | 最新稳定 | `curl -fsSL https://bun.sh/install \| bash` |
| just（可选） | 最新 | `brew install just` / `cargo install just` |

## 启动开发服务器

### 方式一：一键启动（推荐）

```bash
just dev
```

并行启动前后端：
- 后端：http://0.0.0.0:3000
- 前端：http://localhost:5173（自动代理 `/api` → `:3000`）

### 方式二：分终端启动

```bash
# 终端 1 — 后端
cd server && cargo run

# 终端 2 — 前端
cd web && bun install && bun run dev
```

## 项目结构

```
mcguffin/
├── justfile                # 构建命令
├── Dockerfile              # Docker 镜像构建
├── docker-compose.yml      # Docker Compose 编排
├── docker-entrypoint.sh    # 容器入口脚本
├── web/                    # 前端 SPA
│   ├── src/
│   │   ├── App.tsx         # 路由 + Layout + Context
│   │   ├── main.tsx        # 入口
│   │   ├── index.css       # Tailwind + 自定义样式
│   │   ├── components/     # 可复用组件
│   │   ├── features/       # 功能模块（按业务划分）
│   │   │   ├── admin/      # 管理后台页面
│   │   │   ├── auth/       # 认证页面
│   │   │   ├── community/  # 社区讨论
│   │   │   ├── contests/   # 赛事管理
│   │   │   ├── problems/   # 题目系统
│   │   │   ├── profile/    # 个人主页
│   │   │   ├── showcase/   # 成果展示
│   │   │   └── team/       # 团队管理
│   │   ├── stores/         # Zustand 状态管理
│   │   ├── services/       # API 客户端
│   │   ├── plugins/        # 插件注册系统
│   │   └── hooks/          # 自定义 Hooks
│   └── ...配置文件
├── server/                 # 后端
│   ├── Cargo.toml
│   ├── migrations/         # SQLite 迁移
│   ├── src/
│   │   ├── main.rs         # 入口 + 路由注册
│   │   ├── handlers/       # API handler 模块
│   │   ├── plugin/         # WASM 插件系统
│   │   ├── infra/          # 基础设施（配置/持久化）
│   │   ├── types.rs        # 数据模型
│   │   └── utils.rs        # 工具函数
│   └── tests/api.rs        # 集成测试
└── .github/workflows/
    ├── test.yml            # PR/Push 测试
    ├── release.yml         # Tag 触发发布
    └── docker.yml          # Docker 多架构构建
```

## 开发工作流

### 编码规范

- **后端**：Rust Edition 2021，`rustfmt` 格式化，`clippy` 检查
- **前端**：TypeScript strict 模式，`react-router-dom` 路由，Zustand 状态管理
- 日志使用 `tracing` 库，不用 `println!`
- 语言：中文注释、中文 commit message

### 代码检查

```bash
# 全部检查
just check

# 仅后端
cd server && cargo check --bins && cargo clippy --bins -- -D warnings

# 仅前端
cd web && bun run tsc --noEmit
```

### 运行测试

```bash
# 全部测试
just test

# 仅后端
cd server && cargo test

# 仅前端
cd web && bun run test
```

### 新增 API 接口

1. 在 `server/src/handlers/` 下对应模块新增 handler 函数
2. 在 `server/src/routes.rs` 注册路由
3. 如需鉴权，在 handler 中调用 `resolve_user()` + 权限检查
4. 如新增数据模型，在 `types.rs` 定义并添加 `Serialize` + `Deserialize`

### 新增前端页面

1. 在 `web/src/features/` 下创建对应功能目录 + 页面组件
2. 在 `web/src/app/routes.tsx` 的 `<Routes>` 中注册路由
3. 如需权限控制，使用 `ProtectedRoute` 包裹路由或 `hasPermission()` 条件渲染

## 构建生产版本

```bash
# 全部构建
just build

# 单独构建后端
cd server && cargo build --release

# 单独构建前端
cd web && bun run build
```

产物：
- 后端二进制：`server/target/release/mcguffin-server` + `mcguffin`
- 前端静态文件：`web/dist/`

## Docker 构建

```bash
# 开发测试
docker build -t mcguffin:dev .

# 生产多架构构建
docker buildx build --platform linux/amd64,linux/arm64 -t ghcr.io/deng2010/mcguffin:latest .
```

## 数据库

### SQLite 模式

- WAL 模式（`PRAGMA journal_mode=WAL` + `synchronous=NORMAL`）
- 连接池：`max_connections=5`
- 迁移：`server/migrations/`，启动时自动运行 `sqlx::migrate!()`

### 内存缓存架构

```
启动 → SQLite → 内存 HashMap（全量加载）
                            ↓
                   运行时操作内存（读/写）
                            ↓
                   定时 save_all_to_db() 写回 SQLite
```

### 查看数据

```bash
# 使用 sqlite3 CLI
sqlite3 mcguffin_data.db
.tables
SELECT * FROM users;
```
