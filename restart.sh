#!/bin/bash
# Quick restart script for McGuffin server
# Usage: ./restart.sh [--backend-only] [--frontend-only]

SERVER_DIR="$(cd "$(dirname "$0")/server" && pwd)"
WEB_DIR="$(cd "$(dirname "$0")/web" && pwd)"
BINARY="$SERVER_DIR/target/release/mcguffin-server"

REBUILD_BACKEND=true
REBUILD_FRONTEND=true

for arg in "$@"; do
  case $arg in
    --backend-only) REBUILD_FRONTEND=false ;;
    --frontend-only) REBUILD_BACKEND=false ;;
  esac
done

echo "=== McGuffin Server Restart ==="

# Kill existing server
echo "[1/4] Stopping server..."
pkill -f mcguffin-server 2>/dev/null || true
sleep 1

# Rebuild backend
if $REBUILD_BACKEND; then
  echo "[2/4] Building backend..."
  cd "$SERVER_DIR"
  cargo build --release
else
  echo "[2/4] Skipping backend build"
fi

# Rebuild frontend
if $REBUILD_FRONTEND; then
  echo "[3/4] Building frontend..."
  cd "$WEB_DIR"
  bun run build
else
  echo "[3/4] Skipping frontend build"
fi

# Start server
echo "[4/4] Starting server..."
nohup "$BINARY" > /tmp/mcguffin.log 2>&1 &

sleep 2
if curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/ | grep -q 200; then
  echo "Server restarted successfully — HTTP 200"
else
  echo "Warning: server may not be healthy"
fi
