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
FROM rust:1.86-slim-bookworm AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked

FROM chef AS planner
WORKDIR /app/server
COPY server/ .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app/server
COPY --from=planner /app/server/recipe.json recipe.json
# 只编译依赖，后续改源码跳过这层
RUN cargo chef cook --release --recipe-path recipe.json
# 复制源码并正式构建
COPY server/ .
RUN cargo build --release

# ==================== Stage 3: Runtime ====================
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates tzdata sqlite3 wget gosu \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system mcguffin && useradd --system -g mcguffin mcguffin

WORKDIR /app

COPY --from=builder /app/server/target/release/mcguffin-server /app/
COPY --from=builder /app/server/target/release/mcguffin /app/
COPY --from=frontend /app/web/dist/ /app/web/dist/

COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 3000
VOLUME ["/app/data"]

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/mcguffin-server"]
