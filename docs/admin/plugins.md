# 🧩 插件管理

## 概述

McGuffin 支持通过 WASM（WebAssembly）插件扩展功能。插件以 `.wasm` 文件形式分发，通过管理后台 API 安装和卸载。

插件在沙箱中运行（使用 `wasmtime` 运行时），没有文件系统或网络访问权限。

## 管理后台操作

在 `/admin/plugins` 页面：

### 安装插件

**方式一：上传文件**
1. 点击「上传 .wasm 文件」
2. 选择编译好的 `.wasm` 文件
3. 文件名（不含扩展名）自动作为插件 ID
4. 系统验证并加载插件

**方式二：URL 安装**
1. 输入插件 ID（如 `my-plugin`）
2. 输入 WASM 文件下载 URL
3. 点击「安装」

### 查看已安装插件

插件列表显示：
- 名称、版本、作者
- 描述
- 插件 ID

### 卸载插件

1. 找到目标插件
2. 点击「卸载」
3. 确认操作
4. `.wasm` 文件被删除，插件从内存中移除

### 扫描刷新

点击「扫描目录刷新」会：
1. 扫描数据目录下的 `plugins/` 文件夹
2. 加载新增的 `.wasm` 文件
3. 移除已删除文件的插件记录

## API 方式

### 安装

```bash
# 上传安装
curl -X POST "http://localhost:3000/api/admin/plugins/install?id=hello" \
  -H "Authorization: Bearer <token>" \
  --data-binary @plugin.wasm

# URL 安装
curl -X POST "http://localhost:3000/api/admin/plugins/install-url?id=hello&url=https://example.com/hello.wasm" \
  -H "Authorization: Bearer <token>"
```

### 列表与卸载

```bash
# 列表
curl -H "Authorization: Bearer <token>" \
  http://localhost:3000/api/plugins

# 卸载
curl -X DELETE "http://localhost:3000/api/admin/plugins/hello" \
  -H "Authorization: Bearer <token>"
```

## 开发插件

参考 [PLUGIN_DEV.md](../../PLUGIN_DEV.md) 获取完整的 WASM 插件开发标准，包括：

- ABI 规范（4 个导出函数）
- 插件 manifest JSON 格式
- Rust 快速开始模板
- 编译和优化指南
- 安装部署流程
