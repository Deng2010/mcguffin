#!/bin/sh
set -e

DATA_DIR="${MCGUFFIN_DATA_DIR:-/app/data}"
CONFIG_FILE="${DATA_DIR}/config.toml"

echo "→ McGuffin Docker Entrypoint"
echo "   Data dir: ${DATA_DIR}"
echo "   Config:   ${CONFIG_FILE}"

# 修正数据目录权限（容器以 mcguffin 用户运行，volume 可能 root 持有）
if [ "$(id -u)" = "0" ]; then
    chown -R mcguffin:mcguffin "${DATA_DIR}" 2>/dev/null || true
fi

# 确保数据目录存在（mcguffin 用户可写）
mkdir -p "${DATA_DIR}"

# 如果配置文件不存在，用环境变量生成最小配置
if [ ! -f "${CONFIG_FILE}" ]; then
    echo "→ 首次启动，生成默认配置..."
    cat > "${CONFIG_FILE}" <<EOF
# McGuffin 容器配置 — 由 docker-entrypoint 自动生成
# 通过环境变量覆盖这些值

[server]
site_url = "${SITE_URL:-http://localhost:3000}"
port = 3000

[admin]
password = "${ADMIN_PASSWORD:-admin123}"
display_name = "${ADMIN_DISPLAY_NAME:-管理员}"

[site]
name = "${SITE_NAME:-McGuffin}"

[oauth]
cp_client_id = "${CPOAUTH_CLIENT_ID:-}"
cp_client_secret = "${CPOAUTH_CLIENT_SECRET:-}"
EOF
    echo "✓ 配置文件已生成: ${CONFIG_FILE}"
fi

# 设置环境变量供 mcguffin-server 使用（export 必须带上值）
export MCGUFFIN_DATA_DIR="${DATA_DIR}"
export MCGUFFIN_PLUGINS_DIR="${DATA_DIR}/plugins"
export MCGUFFIN_WEB_DIST="${MCGUFFIN_WEB_DIST:-/app/web/dist}"

# 确保插件目录存在且可写
mkdir -p "${MCGUFFIN_PLUGINS_DIR}"
if [ "$(id -u)" = "0" ]; then
    chown -R mcguffin:mcguffin "${MCGUFFIN_PLUGINS_DIR}" 2>/dev/null || true
fi

echo "→ 启动服务..."
# 以 mcguffin 用户运行服务（非 root）
exec su-exec mcguffin "$@"
