# 📦 快速部署

## Docker 部署（推荐）

### 前置要求

| 工具 | 安装 |
|------|------|
| Docker | https://docs.docker.com/engine/install/ |
| Docker Compose（可选） | 随 Docker Desktop 自带，Linux 需单独安装 |

镜像已支持 **多架构**（`linux/amd64` + `linux/arm64`）：
- Intel/AMD 设备 → Docker 自动选择 `amd64`
- Apple Silicon Mac（M 系列）→ Docker 自动选择 `arm64`，原生性能
- ARM 服务器（AWS Graviton、树莓派等）→ 同样使用 `arm64`

### 方式一：docker run（最快）

```bash
docker run -d \
  --name mcguffin \
  -p 3000:3000 \
  -e SITE_URL=http://localhost:3000 \
  -e ADMIN_PASSWORD=请修改此密码 \
  -v mcguffin_data:/app/data \
  ghcr.io/deng2010/mcguffin
```

打开 http://localhost:3000 即可使用。

> **Apple Silicon 用户注意**：如果 `container` 工具无法识别多架构 manifest，可使用单架构标签：
> ```
> ghcr.io/deng2010/mcguffin:latest-linux-arm64
> ```

### 方式二：docker compose（可定制）

```bash
git clone https://github.com/Deng2010/mcguffin.git
cd mcguffin
docker compose up -d
```

编辑 `docker-compose.yml` 可自定义端口、环境变量、卷挂载等。

### 方式三：Apple container（macOS 原生）

```bash
container pull ghcr.io/deng2010/mcguffin:latest-linux-arm64
container run --rm -d \
  --name mcguffin \
  --platform linux/arm64 \
  -p 3000:3000 \
  -e SITE_URL=http://localhost:3000 \
  -e ADMIN_PASSWORD=请修改此密码 \
  -v mcguffin_data:/app/data \
  ghcr.io/deng2010/mcguffin:latest-linux-arm64
```

### 环境变量

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `SITE_URL` | ✅ | — | 站点访问地址，影响 OAuth 回调 |
| `ADMIN_PASSWORD` | ✅ | — | 管理员登录密码 |
| `MCGUFFIN_DATA_DIR` | ❌ | `/app/data` | 数据目录（配置文件 + SQLite） |

---

## 源码编译部署

### 前置要求

| 工具 | 版本 | 安装 |
|------|------|------|
| Rust | ≥ 1.70 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Bun | 最新稳定 | `curl -fsSL https://bun.sh/install \| bash` |
| just（可选） | 最新 | `brew install just` / `cargo install just` |

### 构建并安装

```bash
# 构建全部
just build

# 安装到系统（需 sudo）
sudo just install

# 交互式配置向导
mcguffin init

# 启动服务
mcguffin start
# → http://localhost:3000
```

### 分步构建

```bash
# 后端
cd server
cargo build --release
# 产物: target/release/mcguffin-server + target/release/mcguffin

# 前端
cd web
bun install
bun run build
# 产物: dist/（由 mcguffin-server 静态服务）
```

### 开发机快速部署

```bash
just fast-deploy
# 1. debug 编译后端（5-15s 增量）
# 2. 构建前端
# 3. 复制到安装目录
# 4. 重启 mcguffin 服务
```

---

## 初始化配置

首次启动需设置管理员密码和站点信息：

```bash
mcguffin init
```

交互式向导会依次询问：
1. 管理员密码
2. 管理员显示名
3. 站点名称和标题
4. 站点 URL（用于 OAuth 回调）
5. 服务端口号

生成配置文件至 `/usr/share/mcguffin/config.toml`（或自定义路径）。

---

## 验证部署

```bash
curl http://localhost:3000/api/health
# → {"status":"ok"}

curl http://localhost:3000/api/site/info
# → {"name":"...","title":"...","description":"..."}
```
