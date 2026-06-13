# ===================== McGuffin Justfile =====================
# 安装: cargo install just
# 用法: just <命令>
# =============================================================

# ---------- 全局设置 ----------
set positional-arguments

# ---------- 构建 ----------

# 构建全部（前端 + 后端 + CLI）
build: build-backend build-frontend
    @echo "✓ 全部构建完成"

# 仅构建后端 + CLI
build-backend:
    @echo "── 构建后端 (release) ──"
    cd server && cargo build --release --bins
    @echo "✓ 后端构建完成: server/target/release/mcguffin-server"
    @echo "                    server/target/release/mcguffin"

# 仅构建前端
build-frontend:
    @echo "── 构建前端 ──"
    rm -rf web/dist
    cd web && bun install && bun run build
    @echo "✓ 前端构建完成: web/dist/"

# release 构建（同 build）
release: build

# ---------- 开发 ----------

# 快速部署（debug 编译后端 + 前端，安装到系统并重启服务）
fast-deploy: fast-backend build-frontend
    @echo "── 部署前端 ──"
    rm -rf {{ web_dist_dir }}/dist && cp -r web/dist {{ web_dist_dir }}/
    @echo "── 重启服务 ──"
    -rc-service mcguffin restart 2>/dev/null || true
    @echo "✓ 快速部署完成"

# 仅快速构建后端（debug 模式，增量编译 5-15s）
fast-backend:
    @echo "── 构建后端 (debug) ──"
    cd server && cargo build --bins
    cp server/target/debug/mcguffin-server {{ lib_dir }}/
    cp server/target/debug/mcguffin {{ bin_dir }}/
    @echo "✓ 后端 debug 构建完成"

# 启动开发服务器（前后端并行）
dev:
    @echo "── 启动开发服务器 ──"
    @echo "  前端: http://localhost:5173"
    @echo "  后端: http://localhost:3000"
    @echo ""
    cd server && cargo run &
    cd web && bun run dev

# 仅启动后端开发服务器
dev-backend:
    cd server && cargo run

# 仅启动前端开发服务器
dev-frontend:
    cd web && bun run dev

# ---------- 检查 & 测试 ----------

# 运行全部检查（类型检查 + 测试 + lint）
check: check-backend check-frontend
    @echo "✓ 全部检查通过"

# 后端 Rust 检查
check-backend:
    @echo "── Rust 检查 ──"
    cd server && cargo check --bins
    cd server && cargo clippy --bins -- -D warnings
    @echo "✓ Rust 检查通过"

# 前端 TypeScript 类型检查
check-frontend:
    @echo "── TypeScript 类型检查 ──"
    cd web && bun run tsc --noEmit
    @echo "✓ TypeScript 检查通过"

# 运行全部测试
test: test-backend test-frontend
    @echo "✓ 全部测试通过"

# 后端测试
test-backend:
    @echo "── Rust 测试 ──"
    cd server && cargo test
    @echo "✓ Rust 测试通过"

# 前端测试
test-frontend:
    @echo "── Vitest ──"
    cd web && bun run test
    @echo "✓ 前端测试通过"

# Rust 格式化检查
fmt:
    cd server && cargo fmt --check

# Rust 格式化修复
fmt-fix:
    cd server && cargo fmt

# ---------- 安装 ----------

prefix := env("PREFIX", "/usr/local")
bin_dir := prefix + "/bin"
lib_dir := prefix + "/lib/mcguffin"
web_dist_dir := lib_dir + "/web"
home_dir := env("HOME")

# 安装到系统（需 sudo）
install: build
    @echo "── 安装到 {{ prefix }} ──"
    mkdir -p {{ bin_dir }}
    mkdir -p {{ lib_dir }}
    mkdir -p {{ web_dist_dir }}/dist
    cp server/target/release/mcguffin-server {{ lib_dir }}/
    cp server/target/release/mcguffin {{ bin_dir }}/
    cp -r web/dist/* {{ web_dist_dir }}/dist/
    test -f {{ lib_dir }}/mcguffin_data.json && echo "  保留现有数据文件" || (test -f mcguffin_data.json && cp mcguffin_data.json {{ lib_dir }}/ && echo "  已复制数据文件" || echo "  （无数据文件，将创建新数据）")
    @echo "✓ 安装完成"
    @echo "  二进制:   {{ lib_dir }}/mcguffin-server"
    @echo "  CLI:      {{ bin_dir }}/mcguffin"
    @echo "  Web 静态: {{ web_dist_dir }}/dist/"
    @echo ""
    @echo "启动服务:  cd {{ lib_dir }} && ./mcguffin-server"
    @echo "配置:      mcguffin config set server.site_url <url>"

# 安装到用户目录（无需 root，仅 Linux/macOS）
install-user: build
    @echo "── 安装到 ~/.local ──"
    mkdir -p {{ home_dir }}/.local/bin
    mkdir -p {{ home_dir }}/.local/lib/mcguffin/web/dist
    cp server/target/release/mcguffin-server {{ home_dir }}/.local/lib/mcguffin/
    cp server/target/release/mcguffin {{ home_dir }}/.local/bin/
    cp -r web/dist/* {{ home_dir }}/.local/lib/mcguffin/web/dist/
    @echo "✓ 安装到 ~/.local 完成"
    @echo "  请确保 {{ home_dir }}/.local/bin 在 PATH 中"

# 安装 CLI 二进制到系统（无需前端构建）
install-cli: build-backend
    @echo "── 安装 CLI ──"
    mkdir -p {{ bin_dir }}
    cp server/target/release/mcguffin {{ bin_dir }}/
    @echo "✓ CLI 已安装: {{ bin_dir }}/mcguffin"

# 初始化配置文件（按平台自动选择路径）
init-config:
    @echo "── 生成配置文件 ──"
    cargo run --bin mcguffin -- init
    @echo "提示: 也可在当前目录放 config.toml 供开发使用"

# ---------- 清理 ----------

# 清理所有构建产物
clean:
    @echo "── 清理 ──"
    cd server && cargo clean
    cd web && rm -rf dist node_modules
    rm -rf {{ lib_dir }}
    @echo "✓ 清理完成"

# 仅清理前端
clean-frontend:
    cd web && rm -rf dist

# 仅清理后端
clean-backend:
    cd server && cargo clean

# ---------- 实用工具 ----------

# 查看当前版本
version:
    @echo "McGuffin 版本信息"
    @echo "  后端: $(cd server && cargo metadata --format-version 1 --no-deps | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)"
    @echo "  前端: $(cd web && cat package.json | grep '"version"' | cut -d'"' -f4)"

# 构建 nightly 包（模拟 CI）
dist: build
    @echo "── 打包 ──"
    mkdir -p target/dist
    cp server/target/release/mcguffin-server target/dist/
    cp server/target/release/mcguffin target/dist/
    cp -r web/dist target/dist/
    @echo "✓ 打包完成: target/dist/"
    ls -lh target/dist/

# 显示帮助
default:
    @echo "McGuffin 构建工具 (just)"
    @echo ""
    @echo "用法: just <命令>"
    @echo ""
    @echo "构建:"
    @echo "  build             构建全部 (前端 + 后端)"
    @echo "  build-backend     仅构建后端 + CLI"
    @echo "  build-frontend    仅构建前端"
    @echo ""
    @echo "开发:"
    @echo "  dev               启动前后端开发服务器（并行）"
    @echo "  dev-backend       仅启动后端"
    @echo "  dev-frontend      仅启动前端"
    @echo ""
    @echo "检查 & 测试:"
    @echo "  check             全部检查 (cargo check + tsc + clippy)"
    @echo "  test              全部测试 (cargo test + vitest)"
    @echo "  fmt               Rust 格式化检查"
    @echo ""
    @echo "安装:"
    @echo "  install           构建并安装到系统 (sudo, PREFIX=/usr/local)"
    @echo "  install-user      构建并安装到 ~/.local (无需 root)"
    @echo "  install-cli       仅安装 CLI 二进制"
    @echo "  init-config       生成默认配置文件"
    @echo ""
    @echo "清理:"
    @echo "  clean             清理所有构建产物"
    @echo ""
    @echo "其它:"
    @echo "  dist              打包所有构建产物到 target/dist/"
    @echo "  version           查看版本信息"
