# McGuffin Plugin Development Guide

> 版本 0.1 | 面向 WASM 插件开发者

---

## 概述

McGuffin 支持通过 **WASM (WebAssembly)** 插件扩展功能。插件以 `.wasm` 文件形式分发，通过管理后台 API 安装和卸载。

**插件运行时**：McGuffin 使用 [wasmtime](https://wasmtime.dev/) 作为 WASM 运行时，为插件提供安全沙箱。

---

## 插件 ABI

WASM 插件需导出以下函数（通过函数名查找）：

### `_plugin_manifest() -> i32`

返回指向 JSON 字符串的指针（WASM 线性内存偏移）。调用者读取到 null 终止符为止。

**返回值格式：**
```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "一个示例插件",
  "author": "Your Name",
  "homepage": "https://github.com/you/my-plugin",
  "permissions_needed": []
}
```

| 字段 | 必填 | 说明 |
|---|---|---|
| `id` | ✅ | 唯一标识，小写字母/数字/连字符，最长 64 字符 |
| `name` | ✅ | 显示名称 |
| `version` | ✅ | 语义化版本号 |
| `description` | ❌ | 功能描述 |
| `author` | ❌ | 作者名 |
| `homepage` | ❌ | 项目主页 URL |
| `permissions_needed` | ❌ | 需要的权限列表（预留） |

### `_plugin_on_load(ctx: i32) -> i32`

插件加载时调用。`ctx` 指向 JSON 字符串的指针。

**ctx 参数格式：**
```json
{
  "base_url": "https://lba-oi.team",
  "plugin_data": {}
}
```

返回值：成功返回 0，失败返回错误信息指针。

### `_plugin_on_unload(ctx: i32) -> i32`

插件卸载时调用。参数和返回值同 `_plugin_on_load`。

### `_plugin_handle_request(ctx: i32, req: i32) -> i32`

处理 HTTP 请求时调用。`req` 指向请求 JSON 的指针。

**req 参数格式：**
```json
{
  "method": "GET",
  "path": "/api/plugins/my-plugin/hello",
  "query": "name=test",
  "body": null,
  "headers": [["content-type", "application/json"]]
}
```

**返回值：JSON 字符串指针**
```json
{
  "status": 200,
  "headers": [["content-type", "application/json"]],
  "body": "{\"message\":\"Hello from WASM plugin!\"}"
}
```

---

## 内存管理约定

1. 插件使用 WASM 导出的 `memory` 作为线性内存
2. 所有字符串以 **null 终止**（C 风格字符串）
3. 插件函数返回 `i32` 指针指向线性内存中的字符串
4. 字符串由 **插件负责管理内存**——在堆上分配（通过 WASM 的 `malloc` 或 Rust 的 `Box` 泄出指针）
5. 建议使用 Rust 编写插件，通过 `Box::into_raw` 泄漏指针，插件不需要手动释放

---

## 快速开始（Rust）

### 1. 创建项目

```bash
cargo new --lib mcguffin-plugin-hello
cd mcguffin-plugin-hello
```

### 2. 配置 `Cargo.toml`

```toml
[package]
name = "mcguffin-plugin-hello"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
lto = true
opt-level = "z"
strip = true
```

### 3. 编写插件代码 `src/lib.rs`

```rust
use std::ffi::CString;

/// WASM 导出函数：返回插件清单 JSON 的指针
#[no_mangle]
pub extern "C" fn _plugin_manifest() -> i32 {
    let json = r#"{
        "id": "hello",
        "name": "Hello Plugin",
        "version": "0.1.0",
        "description": "A demo WASM plugin for McGuffin",
        "author": "You",
        "homepage": "",
        "permissions_needed": []
    }"#;
    leak_string(json)
}

/// WASM 导出函数：加载时调用
#[no_mangle]
pub extern "C" fn _plugin_on_load(ctx: i32) -> i32 {
    // ctx 是 JSON 字符串的指针，可以用来读取配置
    // 返回 0 表示成功
    0
}

/// WASM 导出函数：卸载时调用
#[no_mangle]
pub extern "C" fn _plugin_on_unload(ctx: i32) -> i32 {
    0
}

/// WASM 导出函数：处理请求
#[no_mangle]
pub extern "C" fn _plugin_handle_request(ctx: i32, req: i32) -> i32 {
    let response = r#"{
        "status": 200,
        "headers": [["content-type", "application/json"]],
        "body": "{\"message\":\"Hello from WASM plugin!\"}"
    }"#;
    leak_string(response)
}

/// 将 Rust 字符串泄漏到 WASM 堆中，返回指针
fn leak_string(s: &str) -> i32 {
    let c_str = CString::new(s).unwrap();
    let ptr = c_str.into_raw();
    ptr as i32
}
```

### 4. 编译

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

产物在 `target/wasm32-unknown-unknown/release/mcguffin_plugin_hello.wasm`

### 5. 优化 WASM 文件（推荐）

```bash
cargo install wasm-opt
wasm-opt -Oz -o plugin.wasm target/wasm32-unknown-unknown/release/mcguffin_plugin_hello.wasm
```

---

## 安装插件

### 方式一：通过后台 API 上传

```bash
# 上传安装
curl -X POST "http://localhost:3000/api/admin/plugins/install?id=hello" \
  -H "Authorization: Bearer <admin_token>" \
  --data-binary @target/wasm32-unknown-unknown/release/mcguffin_plugin_hello.wasm
```

### 方式二：通过 URL 安装

```bash
# 从 URL 下载安装
curl -X POST "http://localhost:3000/api/admin/plugins/install-url?id=hello&url=https://example.com/plugins/hello.wasm" \
  -H "Authorization: Bearer <admin_token>"
```

### 方式三：直接放置文件

将 `.wasm` 文件放入 `$MCGUFFIN_DATA_DIR/plugins/` 目录，然后触发热重载：

```bash
curl -X POST "http://localhost:3000/api/plugins/reload" \
  -H "Authorization: Bearer <admin_token>"
```

### 查看已安装插件

```bash
curl http://localhost:3000/api/plugins \
  -H "Authorization: Bearer <admin_token>"
```

### 卸载插件

```bash
curl -X DELETE "http://localhost:3000/api/admin/plugins/hello" \
  -H "Authorization: Bearer <admin_token>"
```

---

## 最佳实践

### 安全性

- WASM 插件运行在沙箱中，**没有文件系统 / 网络访问权限**（当前实现）
- 插件只能通过 `_plugin_handle_request` 与 McGuffin 交互
- `permissions_needed` 字段预留用于未来细粒度权限控制

### 性能

- 使用 `wasm-opt -Oz` 压缩 WASM 二进制（从 2MB+ 压缩到 ~100KB）
- 编译时使用 `lto = true`、`opt-level = "z"`、`strip = true` 减小体积
- 避免在 `_plugin_handle_request` 中做阻塞操作

### 插件 ID 规范

- 唯一、简短、描述性强
- 仅允许 `a-z`、`0-9`、`-`
- 最大 64 字符
- 保留 ID：`routes`、`reload`、`install`、`install-url`

### 版本兼容性

| ABI 版本 | McGuffin 版本 | 变更 |
|---|---|---|
| v1 | ≥ 0.3.0 | 初始 ABI 定义 |

---

## 示例插件仓库

参考示例：[mcguffin-plugin-template](https://github.com/Deng2010/mcguffin-plugin-template) (TODO)
