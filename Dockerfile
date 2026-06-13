# =============================================================================
# McGuffin Docker 多阶段构建
# =============================================================================

# ==================== Stage 1: Frontend ====================
FROM oven/bun:1 AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ .
RUN bun run build

# ==================== Stage 2: Backend ====================
FROM rust:1.85-alpine AS backend
RUN apk add --no-cache musl-dev
WORKDIR /app/server

# 缓存依赖层（利用 Docker layer caching 减少重复编译）
COPY server/Cargo.toml server/Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && mkdir -p migrations \
    && echo "CREATE TABLE IF NOT EXISTS dummy (id INTEGER PRIMARY KEY);" > migrations/0001_init.sql \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# 正式构建
COPY server/ .
RUN rm -f migrations/0001_init.sql && cargo build --release

# ==================== Stage 3: Runtime ====================
FROM alpine:3.21
RUN apk add --no-cache ca-certificates tzdata sqlite wget \
    && addgroup -S mcguffin && adduser -S mcguffin -G mcguffin

WORKDIR /app

# 复制二进制
COPY --from=backend /app/server/target/release/mcguffin-server /app/
COPY --from=backend /app/server/target/release/mcguffin /app/

# 复制前端产物
COPY --from=frontend /app/web/dist/ /app/web/dist/

# 初始化脚本
COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 3000
VOLUME ["/app/data"]

USER mcguffin

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/mcguffin-server"]
