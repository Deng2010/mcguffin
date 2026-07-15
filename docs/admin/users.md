# 🔐 用户与权限管理

## 角色层级

| 角色 | 说明 | 不可执行操作 |
|------|------|-------------|
| **superadmin** | 系统所有者（user_id=admin），硬编码保护 | 不可删除、不可降级 |
| **admin** | 团队核心成员，拥有所有管理权限 | 不可操作 superadmin |
| **member** | 团队正式成员 | 不可管理站点/审核 |
| **guest** | 未加入团队的普通用户 | 仅浏览公开内容 |
| **pending** | 已申请但未通过 | 等待审核期间 |

> **`effective_role` 计算规则**：`role=member` 且 `team_status=joined` → `member`；`team_status=pending` → `pending`；`team_status=none` → `guest`。

## 20 种权限

| 权限标识 | 说明 |
|----------|------|
| `view_showcase` | 查看成果展示 |
| `apply_join` | 申请加入团队 |
| `view_team` | 查看团队成员 |
| `manage_team` | 审核入队申请 |
| `manage_members` | 踢出成员/变更角色 |
| `submit_problem` | 投稿题目 |
| `view_problems` | 查看题目 |
| `approve_problem` | 审核题目 |
| `manage_contests` | 管理赛事 |
| `view_all_contests` | 查看全部赛事 |
| `view_public_contests` | 查看公开赛事 |
| `manage_site` | 站点配置 |
| `edit_showcase` | 编辑展示页 |
| `view_discussions` | 查看讨论 |
| `manage_discussions` | 管理讨论 |
| `manage_tags` | 管理标签 |
| `manage_notifications` | 管理通知 |
| `manage_backups` | 管理备份 |
| `view_stats` | 查看统计 |
| `manage_posts` | 管理统一帖子 |

## 权限计算

**三级 OR 取并集**：

```
用户最终权限 = 角色默认权限 ∪ 所属组权限 ∪ 个人额外权限
```

1. **角色基础权限** — 根据 `effective_role` 从配置中读取映射
2. **成员组权限** — 用户所属的权限组（MemberGroup）中各组的权限并集
3. **个人额外权限** — 管理员单独为用户添加的额外权限

> superadmin 拥有通配符 `*`（全部权限），不受任何限制。

## 角色→权限默认映射

| 角色 | 权限 |
|------|------|
| superadmin | 全部（通配符 `*`） |
| admin | 全部（manage_site、approve_problem 等所有管理权限） |
| member | submit_problem、view_problems、view_discussions 等 |
| guest | view_showcase、apply_join、view_public_contests |
| pending | 仅 apply_join |

可通过 `config.toml` 的 `[permissions]` 段自定义覆盖。

## 操作指南

### 管理后台 → 用户管理

在 `/admin/users` 页面：

**变更用户角色**：
1. 找到目标用户
2. 点击「变更角色」
3. 选择新角色（admin/member/guest/pending）
4. 确认

> 不能将 superadmin 降级，不能将其他 admin 降级（需 superadmin 操作）。

**设置权限组**：
1. 点击用户的「设置组」
2. 勾选一个或多个权限组
3. 保存

**设置个人权限**：
1. 点击用户的「设置权限」
2. 勾选需要额外授予的权限
3. 保存

**移除用户**：
1. 点击「移除」
2. 确认操作
3. 用户被删除，关联的 session/notification 也会清理

### 权限组管理

在「用户管理」页面的**权限组**区域：

**创建组**：
1. 输入组名
2. 勾选该组拥有的权限
3. 点击创建

**编辑组**：
1. 点击组的编辑按钮
2. 修改组名或权限
3. 保存

**删除组**：
1. 点击删除
2. 确认（组删除后，原组成员失去该组权限）

## 团队管理

### 入队申请流程

1. 游客在 `/apply` 页面提交申请（姓名、邮箱、理由）
2. 管理员在 `/team` 页面查看待审核列表
3. 管理员点击「通过」或「拒绝」
4. 通过后用户的 `team_status` 变为 `joined`，`effective_role` 变为 `member`
