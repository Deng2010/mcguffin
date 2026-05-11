# AGENTS.md — McGuffin AI Agent 指南

> 面向 AI 编程助手（如 Claude、GitHub Copilot、opencode 等），提供代码库结构、开发约定与操作指南。

---

## 1. 项目概览

**McGuffin** 是面向算法竞赛出题团队的协作工具，采用前后端分离架构。

- 前端：React + TypeScript + Vite + Tailwind CSS（包管理器：Bun）
- 后端：Rust + Axum（HTTP 框架）+ Tokio（异步运行时）
- 认证：CP OAuth（支持 PKCE）
- 存储：内存存储（HashMap + Arc\<RwLock\>），服务重启后数据丢失

---

## 2. 关键文件索引

### 2.1 项目根目录

| 文件 | 用途 |
|------|------|
| `DEMAND.md` | 需求文档（功能定义、UI 规范） |
| `DEVELOPMENT.md` | 详细开发文档（数据模型、API、架构） |
| `GUIDE.md` | 面向人类开发者的指南 |
| `README.md` | 项目主页（面向用户） |

### 2.2 前端（`web/`）

| 文件 | 用途 |
|------|------|
| `web/src/App.tsx` | **核心文件**：所有页面组件、路由、权限逻辑集中在此 |
| `web/src/main.tsx` | 应用入口 |
| `web/src/oauthConfig.ts` | OAuth 配置、PKCE 工具函数、token 管理 |
| `web/package.json` | 依赖与脚本 |
| `web/vite.config.ts` | Vite 构建配置 |
| `web/tailwind.config.js` | Tailwind CSS 配置 |
| `web/tsconfig.json` | TypeScript 配置 |

### 2.3 后端（`server/`）

| 文件 | 用途 |
|------|------|
| `server/src/main.rs` | 服务入口：路由注册、中间件配置、启动逻辑 |
| `server/src/lib.rs` | **核心文件**：数据模型定义、AppState、所有 API handler |
| `server/Cargo.toml` | Rust 依赖配置 |

---

## 3. 开发约定

### 3.1 代码风格

- **前端**：
  - 使用 TypeScript（`~5.6.2`），严格模式
  - React 函数组件，使用 hooks
  - 不使用圆角（`border-radius: 0`）
  - 使用 Tailwind CSS 的 `gray` 色阶，避免渐变
  - 无 emoji，注释使用中文

- **后端**：
  - Rust 2021 edition
  - 使用 `tracing` 而非 `println!` 做日志
  - 错误处理使用 `Result`，API 返回 JSON 格式
  - 数据模型使用 `serde` 的 `Serialize`/`Deserialize`

### 3.2 命名规范

- **Rust**：snake_case（变量/函数）、PascalCase（类型/结构体）、UPPER_SNAKE_CASE（常量）
- **TypeScript**：camelCase（变量/函数）、PascalCase（组件/类型）
- **API 路径**：kebab-case（`/api/team-members`）或 snake_case（`/api/team/members`）

### 3.3 权限体系

前端定义了 7 种权限标识符：

```
view_portfolio     - 查看成果展示（所有人，含未登录）
apply_join         - 申请加入团队（登录用户，非 pending）
view_team          - 查看团队成员（member 及以上）
manage_team        - 团队管理（仅 admin）
submit_problem     - 投稿题目（member 及以上）
view_all_problems  - 查看所有题目含待审核（member 及以上）
approve_problem    - 审核题目（仅 admin）
```

---

## 4. 数据模型

### 4.1 User

```rust
// 后端 (lib.rs)
struct User {
    id: String,           // CP OAuth sub
    username: String,
    display_name: String,
    avatar_url: Option<String>,
    email: Option<String>,
    role: String,         // "admin" | "member" | "guest" | "pending"
    team_status: String,  // "none" | "pending" | "joined"
    created_at: DateTime<Utc>,
}
```

### 4.2 Problem

```rust
struct Problem {
    id: String,
    title: String,
    author_id: String,
    author_name: String,
    contest: String,
    difficulty: String,   // "Easy" | "Medium" | "Hard"
    content: String,      // Markdown 格式
    status: String,       // "pending" | "approved" | "rejected"
    created_at: DateTime<Utc>,
    public_at: Option<DateTime<Utc>>,
}
```

### 4.3 TeamMember

```rust
struct TeamMember {
    id: String,
    user_id: String,
    name: String,
    avatar: String,
    role: String,         // "admin" | "member"
    joined_at: String,    // YYYY-MM-DD
}
```

### 4.4 JoinRequest

```rust
struct JoinRequest {
    id: String,
    user_id: String,
    user_name: String,
    user_email: String,
    reason: String,
    status: String,       // "pending" | "approved" | "rejected"
    created_at: DateTime<Utc>,
}
```

---

## 5. API 速查表

### 5.1 认证

```
GET  /api/oauth/authorize              → 重定向到 CP OAuth
GET  /api/oauth/callback?code=xxx      → OAuth 回调
POST /api/oauth/token                  → 刷新 token
POST /api/logout                       → 登出
GET  /api/user/me                      → 当前用户信息 (Bearer Token)
GET  /api/user/verify                  → 验证 token
```

### 5.2 团队

```
GET  /api/team/members                 → 团队成员列表
GET  /api/team/requests                → 待处理入队申请
POST /api/team/apply                   → 申请加入 (body: {"reason": "..."})
POST /api/team/review/:id/:action      → 审核申请 (action: approve|reject)
```

### 5.3 题目

```
GET  /api/problems                     → 题目列表 (?all=true 返回全部)
POST /api/problems                     → 投稿题目
POST /api/problems/review/:id/:action  → 审核题目 (action: approve|reject)
```

---

## 6. 常用操作

### 6.1 安装与启动

```bash
# 前端
cd web && bun install && bun run dev

# 后端
cd server && cargo run

# 生产构建
cd web && bun run build          # → dist/
cd server && cargo build --release  # → target/release/mcguffin-server
```

### 6.2 运行测试

```bash
# 后端
cd server && cargo test

# 前端
cd web && bun run test
```

### 6.3 格式检查

```bash
# Rust
cd server && cargo fmt && cargo clippy

# TypeScript (类型检查已在 build 脚本中)
cd web && bun run build  # 包含 tsc --noEmit
```

---

## 7. 修改代码时的注意事项

### 7.1 前端修改

- 所有组件集中在 `web/src/App.tsx`，修改时注意保持文件结构清晰
- 新增页面需要在路由表中注册（`App.tsx` 中的 `<Routes>`）
- 新增权限需要在权限矩阵和角色权限映射中添加对应项
- OAuth 配置在 `web/src/oauthConfig.ts` 中修改

### 7.2 后端修改

- 数据模型定义在 `server/src/lib.rs` 顶部
- API handler 函数在 `server/src/lib.rs` 中
- 路由注册在 `server/src/main.rs` 的 `Router` 中
- 新增接口需要：
  1. 在 `lib.rs` 中定义 handler 函数
  2. 在 `main.rs` 中注册路由
  3. 考虑权限校验（检查 `role` 或 `team_status`）

### 7.3 新增数据模型

- 结构体需要实现 `Serialize`、`Deserialize`、`Clone`
- 使用 `uuid::Uuid::new_v4().to_string()` 生成 ID
- 时间字段使用 `chrono::Utc::now()`

### 7.4 数据存储

- 当前全部使用内存存储（`Arc<RwLock<HashMap<...>>>`）
- 所有状态在 `AppState` 中管理
- `AppState` 通过 `Axum` 的 `Extension` 中间件注入到 handler

---

## 8. 已知限制与待办

> 修改代码时请留意以下已知问题，避免引入新 bug。

### 8.1 必须注意的安全问题

- Client Secret 硬编码（前端 + 后端），生产环境需迁移到环境变量
- Token 存储在 localStorage（XSS 风险）
- CORS 配置为 `Any`（允许所有来源）
- 部分使用 GET 请求的接口实际修改了状态（应改为 POST/PUT/DELETE）

### 8.2 数据丢失

- 所有数据存储在内存中，服务重启后丢失

### 8.3 未实现功能

- 踢出成员的后端 API
- 非成员投稿功能
- 题目详情查看页面
- Markdown 渲染（内容已保存为 Markdown 但未在前端渲染）
- 用户个人资料页面

### 8.4 工程质量

- 前端所有组件集中在单一文件 `App.tsx`
- 缺少单元测试和集成测试
- 缺少错误边界和全局错误处理

---

## 9. UI 设计规范

- 灰白色系，低饱和度素色
- **不使用圆角**（所有 `border-radius` 应为 0）
- 不使用渐变色
- 使用 Tailwind 的 `gray` 色阶（gray-50 到 gray-900）
- 状态色：绿色（成功/通过）、黄色（待处理）、红色（失败/拒绝）

---

## 10. 演示模式

当 CP OAuth 未配置或请求失败时，系统回退到演示模式：

| 输入的 token | 模拟身份 |
|-------------|---------|
| `admin_token` | 张三，admin，已加入团队 |
| `member_token` | 赵六，member，已加入团队 |
| `pending_token` | 申请者，pending，待审核 |
| `new_user_token` | 新用户，guest，未加入团队 |

其他任意 token 也会创建 guest 用户。

---

## 11. AI Agent Token 优化指南

> AI 编程助手（如 Claude Code、OpenCode、Codex、Copilot 等）按 token 计费。以下策略可显著减少 token 消耗，延长单次会话的有效上下文长度，降低 API 开销。

### 11.1 系统提示词精简

系统提示词是每轮对话中最先加载、最常重复的部分。

**关键原则：**
- **去掉废话** —— 删除"你是一个优秀的助手"、"尽力帮助用户"等无信息量的套话
- **提炼为极简指令** —— 用最少的词传达完整的约束
- **优先使用 context files（AGENTS.md）** —— 项目结构和约定写在 AGENTS.md 中，代理会自动读取，无需塞入系统提示词
- **技能按需加载** —— 不要一次性加载所有技能，只用 `/skill name` 按需注入

**Hermes 实践参考：**
```python
# prompt_builder.py 中的系统提示词拼装逻辑：
# - 身份描述 + 平台提示 → ~200 tokens
# - 技能索引 → 每个技能 ~30-50 tokens（仅描述 + 触发条件）
# - Context files → AGENTS.md / .cursorrules 按目录扫描注入
# 总控：minimal_system_prompt 模式下跳过大部分身份描述
```

**对比数据：**
| 策略 | 系统提示词大小 | 每轮节省 |
|------|--------------|---------|
| 完整模板 | ~2500 tokens | 基准 |
| 去掉套话 | ~1500 tokens | ~40% |
| 技能按需加载 | ~800 tokens | ~68% |
| Context files 外置 | ~600 tokens | ~76% |

### 11.2 上下文压缩

长时间对话中，历史消息会吞噬上下文窗口。

**核心策略：**

**1. 摘要压缩（Hermes 的 ContextCompressor）**
```
[CONTEXT COMPACTION — REFERENCE ONLY]
Earlier turns were compacted into the summary below...
```
- 使用辅助模型（便宜/快速）对历史回合做摘要，替代原文
- 保护上下文窗口的头（系统提示词）和尾（最新消息）
- 压缩比例为 20%（摘要 token 数 = 被压缩内容 × 0.20）
- 摘要上限 12,000 tokens，下限 2,000 tokens

**2. 工具输出裁剪**
- 旧的工具结果替换为占位符：`[Old tool output cleared to save context space]`
- 长文件读取结果如果已过时，优先裁剪

**3. 迭代式摘要更新**
- 多次压缩场景下，每次都复用上次的摘要并追加新内容
- 避免每次压缩丢失累积信息

**操作建议：**
```bash
# 手动触发压缩（Hermes）
/compress

# CLI 中设置自动压缩阈值
hermes config set compression.threshold 0.50   # 上下文使用超过 50% 时触发
hermes config set compression.target_ratio 0.20 # 压缩后保留 20%
```

### 11.3 工具调用优化

工具调用是 token 消耗的大头 —— 每次调用都有工具定义 + 参数 + 结果。

**1. 批量工具调用**
- 一次性返回多个工具调用，减少 LLM 往返次数
- 每轮往返 = 工具定义 tokens + 参数 tokens + 结果 tokens
- 合并独立操作为单次调用

**2. 只启用必要工具**
```bash
# 查看已启用的工具集
hermes tools list

# 只启用当前任务需要的工具集
hermes tools disable web      # 非研究任务不需要网页搜索
hermes tools disable browser  # 非前端调试不需要浏览器
hermes tools disable vision   # 非图片处理不需要视觉
```

**3. 简化工具定义**
- 工具 schema 中的 `description` 字段每次调用都会被发送
- 精简描述文本，去掉冗长的示例
- 不加代理不认识的语言注释

**4. 工具结果裁剪**
- 读取文件时只取需要的部分（`head -50` / `offset + limit`）
- `ls` 只列出需要的目录，用 `grep` 过滤
- 长命令结果用 `tail` 截断

**对比：**
| 优化 | 每轮消耗 | 
|------|---------|
| 12 个工具集（全开） | ~5000 tokens |
| 5 个必要工具集 | ~2000 tokens |
| 精简描述后 | ~1200 tokens |

### 11.4 Prompt Caching

主流 LLM 服务商（Anthropic、OpenAI、DeepSeek 等）都支持 Prompt Caching。

**原理：** 如果两次请求的系统提示词前缀相同，缓存命中后可节省 **50-90%** 的输入 token 费用。

**最佳实践：**

**1. 稳定系统提示词**
- 系统提示词放在消息列表最前面
- 避免在会话中动态修改系统提示词（破坏缓存）
- 技能描述等变化的内容放到系统提示词后面

**2. 缓存友好排序**
```
[缓存命中区域]          → 不变的：系统提示词 + AGENTS.md + skill 索引
[缓存不命中区域]         → 变化的：当前轮 user message + 最新 tool results
```

**3. 长上下文复用**
- 频繁切换会话会丢弃缓存
- 尽量在同一个会话中完成相关任务序列

**支持缓存的提供商：**
| 提供商 | 缓存起始长度 | 折扣 |
|--------|------------|------|
| Anthropic Claude | 1024 tokens | ~90% 输入费减免 |
| OpenAI GPT-4o | 1024 tokens | 50% 输入费减免 |
| DeepSeek | 自动 | 输入费 10% |
| Google Gemini | 32K tokens | 自动适用 |

### 11.5 消息序列优化

**不要做的事：**
- ❌ 连续发送空消息或纯标点消息
- ❌ 在同一轮中同时发送多条相同意图的消息
- ❌ 用多条消息分段发送同一个问题

**要做的：**
- ✅ 一次性发送完整的上下文和需求
- ✅ 需要修正时，用 /undo + 重发，避免累积无用的修正轮
- ✅ 阶段性完成后主动 `/compress` 压缩历史

### 11.6 迭代次数控制

每次工具调用往返 = 至少一轮 API 请求。限制迭代次数可以从根本上节省 token。

```yaml
# Hermes config
agent:
  max_turns: 90              # 最大迭代轮次（默认 90）
  iteration_budget: 30       # 单任务预算（超过则从摘要模式继续）
```

**策略：**
- 简单任务设置 `max_turns: 10-20`
- 复杂任务设置 `max_turns: 50-90`
- 使用 `delegate_task`（Hermes 子代理）将大任务拆分为多个小会话 —— 每个子代理独立清理上下文

### 11.7 委托与并行

将大任务拆分为子代理可以有效分摊上下文开销：

**Hermes delegate_task 的 token 优势：**
- 子代理有独立的上下文窗口，不共享父代理的历史
- 子代理可以只启用特定工具集（如 `['terminal', 'file']`）
- 每个子代理结束时只返回摘要，父代理上下文不会被中间结果污染

```python
# 坏：在一个会话中做所有事（很快占满上下文）
# 好：分散到子代理，每个只返回摘要
delegate_task(goal="实现X功能", toolsets=['terminal', 'file'])
```

### 11.8 模型选择

不同模型的价格差异可达 100 倍：

| 模型 | 输入价格（每百万 token） | 适合场景 |
|------|-----------------------|---------|
| Claude Sonnet 4 | $3 | 日常编码、代码审查 |
| DeepSeek V4 | $0.5 | 简单重构、文档生成 |
| Gemini 2.5 Flash | $0.15 | 摘要、分类、简单查询 |
| Claude Haiku 3.5 | $0.80 | 快速迭代的简单任务 |

**策略：**
- 复杂任务用强模型（Sonnet/Opus）
- 简单重复任务用便宜模型（Haiku/Gemini Flash/DeepSeek）
- 使用 `hermes model` 在会话中切换模型
- 辅助模型（压缩、摘要、搜索）可以独立配置便宜模型

### 11.9 监控与审计

```bash
# Hermes 内置
hermes insights --days 7      # 查看 7 天用量
/usage                        # 当前会话 token 统计

# API 级别
# OpenRouter 等提供商的 Dashboard 可查看实时 token 消耗
# 设置预算告警：OpenRouter → Credits → Usage Alerts
```

### 11.10 推荐工作流

```
[任务分析] → 选模型（复杂/简单）
    ↓
[极简系统提示词] → 只加载必要的技能 + AGENTS.md
    ↓
[一次性发送完整需求] → 避免多轮修正
    ↓
[启用必要的工具集] → 关闭不必要的 web/browser
    ↓
[定期 /compress] → 或设置自动压缩
    ↓
[阶段性完成 → /reset] → 或 delegate 给子代理
```

### 性能预期

对于 McGuffin 项目日常开发（Rust + TypeScript），采用上述优化后：

| 场景 | 优化前（每轮） | 优化后（每轮） |
|------|--------------|--------------|
| 简单代码修改 | ~8000 tokens | ~3000 tokens |
| 新增功能（5-10 轮） | ~80K tokens | ~25K tokens |
| 跨文件重构 | ~200K tokens | ~50K tokens |
| **预估节省** | — | **~60-70%** |
