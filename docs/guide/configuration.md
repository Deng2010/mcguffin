# ⚙️ 配置详解

McGuffin 的配置集中在一个 TOML 文件中，可通过 CLI 或管理后台运行时修改。

## 配置文件路径

| 模式 | 默认路径 | 说明 |
|------|----------|------|
| Docker 部署 | `${MCGUFFIN_DATA_DIR}/config.toml` | 默认 `/app/data/config.toml` |
| 系统安装 | `/usr/share/mcguffin/config.toml` | `mcguffin init` 生成 |
| 环境变量覆盖 | — | 未找到配置文件时回退到环境变量 |

通过 `MCGUFFIN_DATA_DIR` 环境变量可自定义数据目录。

## 完整配置项

```toml
[server]
# 服务端口
port = 3000
# 站点 URL（影响 OAuth 回调）
site_url = "http://localhost:3000"
# SQLite 数据文件路径
data_file = "mcguffin_data.db"

[admin]
# 管理员密码（明文，首次启动后建议更改）
password = "your_password"
# 管理员显示名
display_name = "管理员"

[site]
# 站点名称（显示在导航栏 logo 位置）
name = "McGuffin"
# 站点标题（显示在浏览器标签页）
title = "McGuffin - 算法竞赛出题团队"

[oauth]
# CP OAuth 客户端 ID
cp_client_id = ""
# CP OAuth 客户端密钥
cp_client_secret = ""

[difficulty]
# 题目难度等级列表（可自定义）
levels = ["入门", "普及", "提高", "省选", "NOI"]

[backup]
# 自动备份间隔（分钟）
interval_minutes = 60
# 备份保留数量
retention_count = 48

[permissions]
# 角色→权限映射（覆盖默认值）
# 格式: role = ["perm1", "perm2"]
```

## 环境变量

当配置文件不存在时，回退到环境变量模式：

| 环境变量 | 对应配置项 | 说明 |
|----------|------------|------|
| `SITE_URL` | `server.site_url` | 站点 URL |
| `ADMIN_PASSWORD` | `admin.password` | 管理员密码 |
| `MCGUFFIN_DATA_DIR` | — | 数据目录路径 |
| `PORT` | `server.port` | 服务端口 |

Docker 部署时推荐仅通过环境变量配置，无需挂载配置文件。

## CLI 管理配置

```bash
# 查看当前配置
mcguffin config show

# 修改配置项
mcguffin config set server.port 8080
mcguffin config set site.name "My Team"
mcguffin config set admin.display_name "管理员"

# 导出配置
mcguffin config show > backup-config.toml
```

## 管理后台在线修改

管理后台 → **配置** 页面支持在线查看和修改以下配置：

| 配置组 | 可修改项 | 是否需要重启 |
|--------|----------|-------------|
| 站点信息 | 名称、标题、描述 | 否 |
| 难度等级 | 等级列表 | 否 |
| 权限映射 | 角色权限 | 否 |
| 自动备份 | 间隔、保留数 | 否 |
| 服务配置 | 端口、站点 URL | 是 |

> ⚠️ 需要重启的配置项修改后，需通过管理后台的「重启服务」按钮或 CLI 的 `mcguffin restart` 生效。

## 配置修改后生效时机

| 修改方式 | 生效时机 |
|----------|----------|
| CLI `config set` | 即时生效（部分需重启） |
| 管理后台在线修改 | 即时生效（部分需重启） |
| 直接编辑 config.toml | 需重启服务 |
| 环境变量 | 需重启服务 |
