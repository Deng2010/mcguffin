# =============================================================================
# McGuffin Docker 多架构构建（交叉编译，无需 QEMU）
# =============================================================================

# ==================== Stage 1: Frontend ====================
FROM oven/bun:1 AS frontend
WORKDIR /app/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

# ==================== Stage 2: Backend ====================
# BUILDPLATFORM 确保构建在原生架构跑（不用 QEMU 模拟 arm64）
FROM --platform=$BUILDPLATFORM rust:1.86-alpine AS backend
RUN apk add --no-cache musl-dev sqlite-dev pkgconfig build-base

# 安装交叉编译 target
RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

WORKDIR /app/server

# 依赖层缓存
COPY server/Cargo.toml server/Cargo.lock ./
COPY server/migrations/ ./migrations/
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release 2>/dev/null || true \
    && rm -rf src

# 正式构建（交叉编译到 TARGETARCH 指定的架构）
COPY server/ .
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then \
      cargo build --release --target aarch64-unknown-linux-musl \
        && mkdir -p /out && cp target/aarch64-unknown-linux-musl/release/mcguffin-server /out/ \
        && cp target/aarch64-unknown-linux-musl/release/mcguffin /out/; \
    else \
      cargo build --release --target x86_64-unknown-linux-musl \
        && mkdir -p /out && cp target/x86_64-unknown-linux-musl/release/mcguffin-server /out/ \
        && cp target/x86_64-unknown-linux-musl/release/mcguffin /out/; \
    fi

# ==================== Stage 3: Runtime ====================
FROM alpine:3.21
RUN apk add --no-cache ca-certificates tzdata sqlite wget \
    && addgroup -S mcguffin && adduser -S mcguffin -G mcguffin

WORKDIR /app

COPY --from=backend /out/mcguffin-server /app/
COPY --from=backend /out/mcguffin /app/
COPY --from=frontend /app/web/dist/ /app/web/dist/

COPY docker-entrypoint.sh /app/
RUN chmod +x /app/docker-entrypoint.sh

EXPOSE 3000
VOLUME ["/app/data"]

USER mcguffin

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/mcguffin-server"]
