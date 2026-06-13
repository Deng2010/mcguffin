# =============================================================================
# McGuffin Docker 构建
# =============================================================================

# ==================== Stage 1: Frontend ====================
FROM oven/bun:1 AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

# ==================== Stage 2: Backend ====================
FROM rust:1.86-alpine AS backend
RUN apk add --no-cache musl-dev sqlite-dev pkgconfig build-base
WORKDIR /app/server

# 依赖层缓存
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/migrations/ ./migrations/
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# 正式构建
COPY server/ .
RUN cargo build --release

# ==================== Stage 3: Runtime ====================
FROM alpine:3.21
RUN apk add --no-cache ca-certificates tzdata sqlite wget \
    && addgroup -S mcguffin && adduser -S mcguffin -G mcguffin

WORKDIR /app

COPY --from=backend /app/server/target/release/mcguffin-server /app/
COPY --from=backend /app/server/target/release/mcguffin /app/
COPY --from=frontend /app/web/dist/ /app/web/dist/

COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 3000
VOLUME ["/app/data"]

USER mcguffin

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/mcguffin-server"]
