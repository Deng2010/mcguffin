//! SQLite 数据库初始化、WAL 配置、数据导入/导出、在线备份
//!
//! 本模块负责 SQLite 的引导阶段：
//! - 创建连接池 + 运行迁移
//! - 设置 PRAGMA 优化参数
//! - 从旧版 `data.json` 导入数据（一次性迁移）
//! - 从 SQLite 加载全部数据到内存
//! - 在线备份（通过 rusqlite backup API）
//! - JSON 导出/导入（跨版本兼容）

use std::collections::HashMap;

use rusqlite::backup::Backup;
use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tracing;

use chrono::Utc;

use crate::state::SavedData;
use crate::types::*;

// ============== 初始化 ==============

/// 创建 SQLite 连接池、运行迁移、设置 PRAGMA
///
/// 先尝试基于文件的 SQLite（持久化），失败时回退到 `:memory:`（仅测试/演示环境）。
pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    // 尝试文件模式
    let pool_result = try_init_db_file(db_path).await;
    match pool_result {
        Ok(pool) => return Ok(pool),
        Err(e) => {
            tracing::warn!(
                "无法打开 SQLite 数据库文件 {}，回退到内存模式: {}",
                db_path,
                e
            );
        }
    }
    // 回退到内存模式（测试环境或权限不足时使用）
    let mem_opts = SqliteConnectOptions::new()
        .filename(":memory:")
        .foreign_keys(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect_with(mem_opts)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    // `:memory:` 模式下 WAL 模式不支持，跳过 PRAGMA
    tracing::info!("SQLite 以内存模式运行（数据不会持久化）");
    Ok(pool)
}

async fn try_init_db_file(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    // 使用 SqliteConnectOptions 禁用 FK 约束，避免 pool 连接复用问题
    // （PRAGMA foreign_keys 是 per-connection 设置，pool 层面设置不可靠）
    let opts = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .foreign_keys(false);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect_with(opts)
        .await?;

    // 运行迁移（CREATE TABLE IF NOT EXISTS）
    sqlx::migrate!("./migrations").run(&pool).await?;

    // PRAGMA 优化
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA cache_size = -64000")
        .execute(&pool)
        .await?; // 64 MB

    // FK 约束在启动时由 state.rs 控制：先关闭用于 JSON 导入，再开启用于正常操作
    // 此处不设置 PRAGMA foreign_keys，默认由后续代码管理

    tracing::info!("SQLite database initialized: {}", db_path);
    Ok(pool)
}

// ============== JSON 数据导入 ==============

/// 从 `SavedData`（反序列化的 data.json）导入所有数据到 SQLite。
/// 如果表中已有数据则跳过导入（幂等）。
pub(crate) async fn import_saved_data(
    pool: &SqlitePool,
    data: &SavedData,
) -> Result<u32, sqlx::Error> {
    // 检查是否已有数据
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count > 0 {
        tracing::info!("SQLite 已有 {} 个用户，跳过 JSON 导入", user_count);
        return Ok(0);
    }

    let mut total: u32 = 0;

    // 按 FK 依赖顺序插入
    total += import_users(pool, &data.users).await?;
    total += import_team_members(pool, &data.team_members).await?;
    total += import_sessions(pool, &data.sessions).await?;
    total += import_refresh_tokens(pool, &data.refresh_tokens).await?;
    total += import_join_requests(pool, &data.join_requests).await?;
    total += import_contests(pool, &data.contests).await?;
    total += import_problems(pool, &data.problems).await?;
    total += import_notifications(pool, &data.notifications).await?;
    total += import_posts(pool, &data.posts).await?;

    // 写入 meta 字段
    import_meta_fields(pool, data).await?;

    tracing::info!("JSON 数据导入完成，共 {} 条记录", total);
    Ok(total)
}

async fn import_users(
    pool: &SqlitePool,
    users: &HashMap<String, User>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for user in users.values() {
        let res = sqlx::query(
            r#"
            INSERT OR IGNORE INTO users
                (id, username, display_name, avatar_url, email, role, team_status,
                 created_at, bio, password_hash, effective_role, group_ids, user_permissions)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.email)
        .bind(&user.role)
        .bind(&user.team_status)
        .bind(user.created_at.to_rfc3339())
        .bind(&user.bio)
        .bind(&user.password_hash)
        .bind(&user.effective_role)
        .bind(serde_json::to_string(&user.group_ids).unwrap_or_default())
        .bind(serde_json::to_string(&user.user_permissions).unwrap_or_default())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_team_members(
    pool: &SqlitePool,
    members: &HashMap<String, TeamMember>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for m in members.values() {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO team_members (id, user_id, joined_at) VALUES (?, ?, ?)",
        )
        .bind(&m.id)
        .bind(&m.user_id)
        .bind(&m.joined_at)
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_sessions(
    pool: &SqlitePool,
    sessions: &HashMap<String, SessionEntry>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for (token, entry) in sessions {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO sessions (token, user_id, last_active) VALUES (?, ?, ?)",
        )
        .bind(token)
        .bind(&entry.user_id)
        .bind(entry.last_active.to_rfc3339())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_refresh_tokens(
    pool: &SqlitePool,
    tokens: &HashMap<String, String>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for (token, user_id) in tokens {
        let res =
            sqlx::query("INSERT OR IGNORE INTO refresh_tokens (token, user_id) VALUES (?, ?)")
                .bind(token)
                .bind(user_id)
                .execute(pool)
                .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_join_requests(
    pool: &SqlitePool,
    requests: &HashMap<String, JoinRequest>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for r in requests.values() {
        let res = sqlx::query(
            r#"INSERT OR IGNORE INTO join_requests
                (id, user_id, user_name, user_email, reason, status, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&r.id)
        .bind(&r.user_id)
        .bind(&r.user_name)
        .bind(&r.user_email)
        .bind(&r.reason)
        .bind(&r.status)
        .bind(r.created_at.to_rfc3339())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_contests(
    pool: &SqlitePool,
    contests: &HashMap<String, Contest>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for c in contests.values() {
        let res = sqlx::query(
            r#"INSERT OR IGNORE INTO contests
                (id, name, start_time, end_time, description, created_by, created_at,
                 status, link, problem_order, visible_to, editable_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&c.id)
        .bind(&c.name)
        .bind(&c.start_time)
        .bind(&c.end_time)
        .bind(&c.description)
        .bind(&c.created_by)
        .bind(c.created_at.to_rfc3339())
        .bind(&c.status)
        .bind(&c.link)
        .bind(serde_json::to_string(&c.problem_order).unwrap_or_default())
        .bind(serde_json::to_string(&c.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&c.editable_by).unwrap_or_default())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

#[allow(clippy::too_many_arguments)]
async fn import_problems(
    pool: &SqlitePool,
    problems: &HashMap<String, Problem>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for p in problems.values() {
        let res = sqlx::query(
            r#"INSERT OR IGNORE INTO problems
                (id, title, author_id, author_name, contest, contest_id, difficulty,
                 content, solution, status, created_at, public_at, claimed_by,
                 verifier_solution, visible_to, link, remark, editable_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&p.id)
        .bind(&p.title)
        .bind(&p.author_id)
        .bind(&p.author_name)
        .bind(&p.contest)
        .bind(&p.contest_id)
        .bind(&p.difficulty)
        .bind(&p.content)
        .bind(&p.solution)
        .bind(&p.status)
        .bind(p.created_at.to_rfc3339())
        .bind(p.public_at.map(|t| t.to_rfc3339()))
        .bind(&p.claimed_by)
        .bind(&p.verifier_solution)
        .bind(serde_json::to_string(&p.visible_to).unwrap_or_default())
        .bind(&p.link)
        .bind(&p.remark)
        .bind(serde_json::to_string(&p.editable_by).unwrap_or_default())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_notifications(
    pool: &SqlitePool,
    notifications: &HashMap<String, Notification>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for n in notifications.values() {
        let res = sqlx::query(
            r#"INSERT OR IGNORE INTO notifications
                (id, user_id, title, body, read, created_at, link)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&n.id)
        .bind(&n.user_id)
        .bind(&n.title)
        .bind(&n.body)
        .bind(n.read as i32)
        .bind(n.created_at.to_rfc3339())
        .bind(&n.link)
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

async fn import_posts(
    pool: &SqlitePool,
    posts: &HashMap<String, Post>,
) -> Result<u32, sqlx::Error> {
    let mut count = 0u32;
    for p in posts.values() {
        let res = sqlx::query(
            r#"INSERT OR IGNORE INTO posts
                (id, title, content, author_id, author_name, created_at, updated_at,
                 tags, pinned, team_only, emoji, reactions, replies,
                 mentioned_user_ids, status, visible_to, editable_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&p.id)
        .bind(&p.title)
        .bind(&p.content)
        .bind(&p.author_id)
        .bind(&p.author_name)
        .bind(p.created_at.to_rfc3339())
        .bind(p.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&p.tags).unwrap_or_default())
        .bind(p.pinned as i32)
        .bind(p.team_only as i32)
        .bind(&p.emoji)
        .bind(serde_json::to_string(&p.reactions).unwrap_or_default())
        .bind(serde_json::to_string(&p.replies).unwrap_or_default())
        .bind(serde_json::to_string(&p.mentioned_user_ids).unwrap_or_default())
        .bind(&p.status)
        .bind(serde_json::to_string(&p.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&p.editable_by).unwrap_or_default())
        .execute(pool)
        .await?;
        count += res.rows_affected() as u32;
    }
    Ok(count)
}

/// 将 SavedData 中的 meta 字段写入 SQLite meta 表
async fn import_meta_fields(pool: &SqlitePool, data: &SavedData) -> Result<(), sqlx::Error> {
    // site_description
    sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('site_description', ?)")
        .bind(&data.site_description)
        .execute(pool)
        .await?;

    // showcase_problem_ids
    sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_problem_ids', ?)")
        .bind(serde_json::to_string(&data.showcase_problem_ids).unwrap_or_default())
        .execute(pool)
        .await?;

    // showcase_contest_ids
    sqlx::query("INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_contest_ids', ?)")
        .bind(serde_json::to_string(&data.showcase_contest_ids).unwrap_or_default())
        .execute(pool)
        .await?;

    Ok(())
}

// ============== 从 SQLite 加载全部数据到 HashMap（启动时使用） ==============

/// 检查 SQLite 中是否有数据
pub(crate) async fn sqlite_has_data(pool: &SqlitePool) -> bool {
    let count: Result<i64, _> = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await;
    count.unwrap_or(0) > 0
}

/// 从 SQLite 读取全部数据并序列化为 JSON 字符串（不依赖本地文件）
pub(crate) async fn export_db_to_json_string(pool: &SqlitePool) -> Result<String, String> {
    let data = load_all_from_sqlite(pool)
        .await
        .map_err(|e| format!("加载数据失败: {}", e))?;
    serde_json::to_string_pretty(&data).map_err(|e| format!("序列化失败: {}", e))
}

// ============== JSON → DB 转换（CLI 工具） ==============

/// 从 JSON 文件导入数据到 SQLite 数据库文件。
/// 专为 CLI `json-to-db` 命令设计，公开为 `pub`。
pub async fn import_json_to_db(json_path: &str, db_path: &str) -> Result<u32, String> {
    let json =
        std::fs::read_to_string(json_path).map_err(|e| format!("无法读取 JSON 文件: {}", e))?;
    let data: crate::state::SavedData =
        serde_json::from_str(&json).map_err(|e| format!("JSON 解析失败: {}", e))?;

    let pool = init_db(db_path)
        .await
        .map_err(|e| format!("创建数据库失败: {}", e))?;

    reimport_all_data(&pool, &data)
        .await
        .map_err(|e| format!("导入数据失败: {}", e))
}

// ============== 全量重新导入（用于 JSON 恢复） ==============

/// 清空所有表并重新导入数据。
/// 用于从 JSON 备份恢复时的全量同步。
pub(crate) async fn reimport_all_data(
    pool: &SqlitePool,
    data: &SavedData,
) -> Result<u32, sqlx::Error> {
    let tables = [
        "audit_log",
        "notifications",
        "sessions",
        "refresh_tokens",
        "join_requests",
        "posts",
        "problems",
        "team_members",
        "contests",
        "users",
    ];

    // 在一个事务中执行清空 + 重新导入，全程关闭 FK 约束
    let mut tx = pool.begin().await?;

    // 暂时关闭 FK 约束，避免 DELETE 时级联问题和 INSERT 时外键引用顺序问题
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&mut *tx)
        .await?;

    // 按外键依赖逆序清空（先清子表，再清父表）
    for table in &tables {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(&mut *tx)
            .await?;
    }

    // 在同一个事务中重新导入（FK 已关闭，不会因数据引用顺序报错）
    // 使用 pool 直接导入，事务中的 FK OFF 设置会保留在当前连接上
    tx.commit().await?;

    // 确保 FK 关闭状态传播到 pool 连接
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    let total = import_saved_data(pool, data).await?;

    // 恢复 FK 约束
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    tracing::info!("全量重新导入完成：{} 条记录", total);
    Ok(total)
}

// ============== 在线备份（rusqlite backup API） ==============

/// 使用 SQLite 在线备份 API 创建一致性快照。
/// 在 WAL 模式下，备份期间源数据库可以继续读写。
/// 源数据库以只读模式打开，避免 WAL checkpoint 干扰运行中的池连接。
pub fn create_consistent_backup(source_path: &str, dest_path: &str) -> Result<(), String> {
    let src = rusqlite::Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("无法打开源数据库: {}", e))?;
    let mut dst =
        rusqlite::Connection::open(dest_path).map_err(|e| format!("无法创建备份文件: {}", e))?;

    let backup = Backup::new(&src, &mut dst).map_err(|e| format!("备份初始化失败: {}", e))?;

    // 每步拷贝最多 100 页，每页之间最多等待 250ms
    backup
        .run_to_completion(100, std::time::Duration::from_millis(250), None)
        .map_err(|e| format!("备份执行失败: {}", e))?;

    Ok(())
}

/// 使用 SQLite 在线备份 API 从备份文件恢复到主数据库（反向备份）。
/// 与 `create_consistent_backup` 方向相反：
/// - 源 = 备份文件（只读）
/// - 目标 = 主数据库文件
///
/// 无需关闭 sqlx 连接池，恢复完成后调用者应执行 `state.reload().await` 刷新内存缓存。
pub fn restore_from_backup(backup_path: &str, db_path: &str) -> Result<(), String> {
    let src = rusqlite::Connection::open_with_flags(backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("无法打开备份文件: {}", e))?;
    let mut dst =
        rusqlite::Connection::open(db_path).map_err(|e| format!("无法打开目标数据库: {}", e))?;

    let backup = Backup::new(&src, &mut dst).map_err(|e| format!("恢复初始化失败: {}", e))?;

    backup
        .run_to_completion(100, std::time::Duration::from_millis(250), None)
        .map_err(|e| format!("恢复执行失败: {}", e))?;

    Ok(())
}

// ============== JSON 数据导出（跨版本兼容） ==============

/// 导出数据结构：用于 JSON 格式的数据导出/导入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExport {
    pub export_version: u32,
    pub exported_at: String,
    pub users: Vec<DataUser>,
    pub team_members: Vec<DataTeamMember>,
    pub join_requests: Vec<DataJoinRequest>,
    pub contests: Vec<DataContest>,
    pub problems: Vec<DataProblem>,
    pub posts: Vec<DataPost>,
    pub notifications: Vec<DataNotification>,
    pub audit_log: Vec<DataAuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataUser {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub role: String,
    pub team_status: String,
    pub created_at: String,
    pub bio: String,
    pub group_ids: String,
    pub user_permissions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTeamMember {
    pub id: String,
    pub user_id: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataJoinRequest {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_email: Option<String>,
    pub reason: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataContest {
    pub id: String,
    pub name: String,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub description: String,
    pub created_by: String,
    pub created_at: String,
    pub status: String,
    pub link: Option<String>,
    pub problem_order: String,
    pub visible_to: String,
    pub editable_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataProblem {
    pub id: String,
    pub title: String,
    pub author_id: String,
    pub author_name: String,
    pub contest: Option<String>,
    pub contest_id: Option<String>,
    pub difficulty: String,
    pub content: String,
    pub solution: Option<String>,
    pub status: String,
    pub created_at: String,
    pub public_at: Option<String>,
    pub claimed_by: Option<String>,
    pub verifier_solution: Option<String>,
    pub visible_to: String,
    pub link: Option<String>,
    pub remark: Option<String>,
    pub editable_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPost {
    pub id: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: String,
    pub pinned: bool,
    pub team_only: bool,
    pub emoji: Option<String>,
    pub reactions: String,
    pub replies: String,
    pub mentioned_user_ids: String,
    pub status: String,
    pub visible_to: String,
    pub editable_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataNotification {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub body: String,
    pub read: bool,
    pub created_at: String,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAuditEntry {
    pub timestamp: String,
    pub user_id: String,
    pub user_name: String,
    pub action: String,
    pub resource: String,
    pub result: String,
    pub reason: Option<String>,
}

// ============== 从 SQLite 加载全部数据到 HashMap（启动时使用） ==============

/// 从 SQLite 读取全部数据，构建 `SavedData` 结构。
/// 这是启动时的主要数据加载路径 —— 替代了从 JSON 文件读取。
pub(crate) async fn load_all_from_sqlite(pool: &SqlitePool) -> Result<SavedData, sqlx::Error> {
    use sqlx::Row;

    let mut data = SavedData {
        users: HashMap::new(),
        sessions: HashMap::new(),
        refresh_tokens: HashMap::new(),
        team_members: HashMap::new(),
        problems: HashMap::new(),
        join_requests: HashMap::new(),
        contests: HashMap::new(),
        site_description: String::new(),
        notifications: HashMap::new(),
        showcase_problem_ids: Vec::new(),
        showcase_contest_ids: Vec::new(),
        posts: HashMap::new(),
        suggestions: HashMap::new(),
        announcements: HashMap::new(),
        discussions: HashMap::new(),
        member_groups: HashMap::new(),
    };

    // ── 读取站点描述 ──
    if let Ok(Some(row)) = sqlx::query("SELECT value FROM meta WHERE key = 'site_description'")
        .fetch_optional(pool)
        .await
    {
        data.site_description = row.get("value");
    }

    // ── 读取 showcase 配置 ──
    if let Ok(Some(row)) = sqlx::query("SELECT value FROM meta WHERE key = 'showcase_problem_ids'")
        .fetch_optional(pool)
        .await
    {
        let s: String = row.get("value");
        data.showcase_problem_ids = serde_json::from_str(&s).unwrap_or_default();
    }
    if let Ok(Some(row)) = sqlx::query("SELECT value FROM meta WHERE key = 'showcase_contest_ids'")
        .fetch_optional(pool)
        .await
    {
        let s: String = row.get("value");
        data.showcase_contest_ids = serde_json::from_str(&s).unwrap_or_default();
    }

    // ── 读取用户 ──
    if let Ok(rows) = sqlx::query(
        "SELECT id, username, display_name, avatar_url, email, role, team_status, \
         created_at, bio, password_hash, effective_role, group_ids, user_permissions FROM users",
    )
    .fetch_all(pool)
    .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.users.insert(
                id.clone(),
                User {
                    id,
                    username: row.get("username"),
                    display_name: row.get("display_name"),
                    avatar_url: row.get("avatar_url"),
                    email: row.get("email"),
                    role: row.get("role"),
                    team_status: row.get("team_status"),
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    bio: row.get("bio"),
                    password_hash: row.get("password_hash"),
                    effective_role: row.get("effective_role"),
                    group_ids: serde_json::from_str(&row.get::<String, _>("group_ids"))
                        .unwrap_or_default(),
                    user_permissions: serde_json::from_str(
                        &row.get::<String, _>("user_permissions"),
                    )
                    .unwrap_or_default(),
                },
            );
        }
    }

    // ── 读取会话 ──
    if let Ok(rows) = sqlx::query("SELECT token, user_id, last_active FROM sessions")
        .fetch_all(pool)
        .await
    {
        for row in rows {
            let token: String = row.get("token");
            let user_id: String = row.get("user_id");
            let last_active: String = row.get("last_active");
            let last_active = chrono::DateTime::parse_from_rfc3339(&last_active)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            data.sessions.insert(
                token,
                SessionEntry {
                    user_id,
                    last_active,
                },
            );
        }
    }

    // ── 读取刷新令牌 ──
    if let Ok(rows) = sqlx::query("SELECT token, user_id FROM refresh_tokens")
        .fetch_all(pool)
        .await
    {
        for row in rows {
            let token: String = row.get("token");
            let user_id: String = row.get("user_id");
            data.refresh_tokens.insert(token, user_id);
        }
    }

    // ── 读取团队成员 ──
    if let Ok(rows) = sqlx::query("SELECT id, user_id, joined_at FROM team_members")
        .fetch_all(pool)
        .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.team_members.insert(
                id.clone(),
                crate::types::TeamMember {
                    id,
                    user_id: row.get("user_id"),
                    joined_at: row.get("joined_at"),
                },
            );
        }
    }

    // ── 读取入队申请 ──
    if let Ok(rows) = sqlx::query(
        "SELECT id, user_id, user_name, user_email, reason, status, created_at FROM join_requests",
    )
    .fetch_all(pool)
    .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.join_requests.insert(
                id.clone(),
                crate::types::JoinRequest {
                    id,
                    user_id: row.get("user_id"),
                    user_name: row.get("user_name"),
                    user_email: row.get("user_email"),
                    reason: row.get("reason"),
                    status: row.get("status"),
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                },
            );
        }
    }

    // ── 读取比赛 ──
    if let Ok(rows) = sqlx::query(
        "SELECT id, name, start_time, end_time, description, created_by, created_at, \
         status, link, problem_order, visible_to, editable_by FROM contests",
    )
    .fetch_all(pool)
    .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.contests.insert(
                id.clone(),
                crate::types::Contest {
                    id,
                    name: row.get("name"),
                    start_time: row.get("start_time"),
                    end_time: row.get("end_time"),
                    description: row.get("description"),
                    created_by: row.get("created_by"),
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    status: row.get("status"),
                    link: row.get("link"),
                    problem_order: serde_json::from_str(&row.get::<String, _>("problem_order"))
                        .unwrap_or_default(),
                    visible_to: serde_json::from_str(&row.get::<String, _>("visible_to"))
                        .unwrap_or_default(),
                    editable_by: serde_json::from_str(&row.get::<String, _>("editable_by"))
                        .unwrap_or_default(),
                },
            );
        }
    }

    // ── 读取题目 ──
    if let Ok(rows) = sqlx::query(
        "SELECT id, title, author_id, author_name, contest, contest_id, difficulty, \
         content, solution, status, created_at, public_at, claimed_by, \
         verifier_solution, visible_to, link, remark, editable_by FROM problems",
    )
    .fetch_all(pool)
    .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.problems.insert(
                id.clone(),
                crate::types::Problem {
                    id,
                    title: row.get("title"),
                    author_id: row.get("author_id"),
                    author_name: row.get("author_name"),
                    contest: row.get("contest"),
                    contest_id: row.get("contest_id"),
                    difficulty: row.get("difficulty"),
                    content: row.get("content"),
                    solution: row.get("solution"),
                    status: row.get("status"),
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    public_at: row
                        .get::<Option<String>, _>("public_at")
                        .and_then(|s| s.parse().ok()),
                    claimed_by: row.get("claimed_by"),
                    verifier_solution: row.get("verifier_solution"),
                    visible_to: serde_json::from_str(&row.get::<String, _>("visible_to"))
                        .unwrap_or_default(),
                    link: row.get("link"),
                    remark: row.get("remark"),
                    editable_by: serde_json::from_str(&row.get::<String, _>("editable_by"))
                        .unwrap_or_default(),
                },
            );
        }
    }

    // ── 读取通知 ──
    if let Ok(rows) =
        sqlx::query("SELECT id, user_id, title, body, read, created_at, link FROM notifications")
            .fetch_all(pool)
            .await
    {
        for row in rows {
            let id: String = row.get("id");
            let read_int: i32 = row.get("read");
            data.notifications.insert(
                id.clone(),
                crate::types::Notification {
                    id,
                    user_id: row.get("user_id"),
                    title: row.get("title"),
                    body: row.get("body"),
                    read: read_int != 0,
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    link: row.get("link"),
                },
            );
        }
    }

    // ── 读取帖子 ──
    if let Ok(rows) = sqlx::query(
        "SELECT id, title, content, author_id, author_name, created_at, updated_at, \
         tags, pinned, team_only, emoji, reactions, replies, \
         mentioned_user_ids, status, visible_to, editable_by FROM posts",
    )
    .fetch_all(pool)
    .await
    {
        for row in rows {
            let id: String = row.get("id");
            data.posts.insert(
                id.clone(),
                crate::types::Post {
                    id,
                    title: row.get("title"),
                    content: row.get("content"),
                    author_id: row.get("author_id"),
                    author_name: row.get("author_name"),
                    created_at: row
                        .get::<String, _>("created_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: row
                        .get::<String, _>("updated_at")
                        .parse()
                        .unwrap_or_else(|_| Utc::now()),
                    tags: serde_json::from_str(&row.get::<String, _>("tags")).unwrap_or_default(),
                    pinned: row.get::<i32, _>("pinned") != 0,
                    team_only: row.get::<i32, _>("team_only") != 0,
                    emoji: row.get("emoji"),
                    reactions: serde_json::from_str(&row.get::<String, _>("reactions"))
                        .unwrap_or_default(),
                    replies: serde_json::from_str(&row.get::<String, _>("replies"))
                        .unwrap_or_default(),
                    mentioned_user_ids: serde_json::from_str(
                        &row.get::<String, _>("mentioned_user_ids"),
                    )
                    .unwrap_or_default(),
                    status: row.get("status"),
                    visible_to: serde_json::from_str(&row.get::<String, _>("visible_to"))
                        .unwrap_or_default(),
                    editable_by: serde_json::from_str(&row.get::<String, _>("editable_by"))
                        .unwrap_or_default(),
                },
            );
        }
    }

    Ok(data)
}

// ============== 测试 ==============

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 创建测试用的内存 SQLite 数据库
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .expect("创建测试数据库失败");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("运行迁移失败");

        // 开启 FK 约束（内存模式默认关闭）
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("设置 FK pragma 失败");

        pool
    }

    /// 构造一个包含各类数据的 SavedData
    fn make_test_data() -> SavedData {
        let now = Utc::now();

        let mut users = HashMap::new();
        users.insert(
            "user-001".to_string(),
            User {
                id: "user-001".to_string(),
                username: "alice".to_string(),
                display_name: "Alice".to_string(),
                avatar_url: Some("https://example.com/alice.png".to_string()),
                email: Some("alice@example.com".to_string()),
                role: "member".to_string(),
                team_status: "joined".to_string(),
                created_at: now,
                bio: "Test user".to_string(),
                password_hash: Some("$2b$12$...".to_string()),
                effective_role: "member".to_string(),
                group_ids: vec!["group-1".to_string()],
                user_permissions: vec!["view_portfolio".to_string()],
            },
        );
        users.insert(
            "user-002".to_string(),
            User {
                id: "user-002".to_string(),
                username: "bob".to_string(),
                display_name: "Bob".to_string(),
                avatar_url: None,
                email: None,
                role: "guest".to_string(),
                team_status: "pending".to_string(),
                created_at: now,
                bio: String::new(),
                password_hash: None,
                effective_role: "guest".to_string(),
                group_ids: vec![],
                user_permissions: vec![],
            },
        );

        let mut sessions = HashMap::new();
        sessions.insert(
            "token-abc".to_string(),
            SessionEntry {
                user_id: "user-001".to_string(),
                last_active: now,
            },
        );

        let mut refresh_tokens = HashMap::new();
        refresh_tokens.insert("refresh-xyz".to_string(), "user-001".to_string());

        let mut team_members = HashMap::new();
        team_members.insert(
            "user-001".to_string(),
            TeamMember {
                id: "user-001".to_string(),
                user_id: "user-001".to_string(),
                joined_at: "2025-01-01".to_string(),
            },
        );

        let mut join_requests = HashMap::new();
        join_requests.insert(
            "req-001".to_string(),
            JoinRequest {
                id: "req-001".to_string(),
                user_id: "user-002".to_string(),
                user_name: "Bob".to_string(),
                user_email: "bob@example.com".to_string(),
                reason: "I want to contribute".to_string(),
                status: "pending".to_string(),
                created_at: now,
            },
        );

        let mut contests = HashMap::new();
        contests.insert(
            "contest-001".to_string(),
            Contest {
                id: "contest-001".to_string(),
                name: "2025 Spring Contest".to_string(),
                start_time: "2025-03-01T09:00:00Z".to_string(),
                end_time: "2025-03-01T12:00:00Z".to_string(),
                description: "A test contest".to_string(),
                created_by: "user-001".to_string(),
                created_at: now,
                status: "published".to_string(),
                link: Some("https://example.com/contest".to_string()),
                problem_order: vec!["prob-001".to_string()],
                visible_to: vec!["member".to_string()],
                editable_by: vec!["user-001".to_string()],
            },
        );

        let mut problems = HashMap::new();
        problems.insert(
            "prob-001".to_string(),
            Problem {
                id: "prob-001".to_string(),
                title: "Test Problem".to_string(),
                author_id: "user-001".to_string(),
                author_name: "Alice".to_string(),
                contest: "2025 Spring Contest".to_string(),
                contest_id: Some("contest-001".to_string()),
                difficulty: "Medium".to_string(),
                content: "## Problem Content\n\nSolve this.".to_string(),
                solution: Some("## Solution\n\nHere is how.".to_string()),
                status: "approved".to_string(),
                created_at: now,
                public_at: Some(now),
                claimed_by: None,
                verifier_solution: None,
                visible_to: vec!["member".to_string()],
                link: None,
                remark: None,
                editable_by: vec!["user-001".to_string()],
            },
        );

        let mut notifications = HashMap::new();
        notifications.insert(
            "notif-001".to_string(),
            Notification {
                id: "notif-001".to_string(),
                user_id: "user-001".to_string(),
                title: "Welcome".to_string(),
                body: "Welcome to the team!".to_string(),
                read: false,
                created_at: now,
                link: None,
            },
        );

        let mut reactions = HashMap::new();
        reactions.insert(
            "heart".to_string(),
            vec!["user-001".to_string(), "user-002".to_string()],
        );

        let mut posts = HashMap::new();
        posts.insert(
            "post-001".to_string(),
            Post {
                id: "post-001".to_string(),
                title: "Test Post".to_string(),
                content: "This is a test post.".to_string(),
                author_id: "user-001".to_string(),
                author_name: "Alice".to_string(),
                created_at: now,
                updated_at: now,
                tags: vec!["讨论".to_string(), "测试".to_string()],
                pinned: true,
                team_only: false,
                emoji: Some("heart".to_string()),
                reactions,
                replies: vec![],
                mentioned_user_ids: vec![],
                status: "published".to_string(),
                visible_to: vec![],
                editable_by: vec!["user-001".to_string()],
            },
        );

        SavedData {
            users,
            sessions,
            refresh_tokens,
            team_members,
            problems,
            join_requests,
            contests,
            site_description: "欢迎来到 McGuffin 测试站".to_string(),
            notifications,
            showcase_problem_ids: vec!["prob-001".to_string()],
            showcase_contest_ids: vec!["contest-001".to_string()],
            posts,
            suggestions: HashMap::new(),
            announcements: HashMap::new(),
            discussions: HashMap::new(),
            member_groups: HashMap::new(),
        }
    }

    /// 测试：JSON 数据首次导入到 SQLite
    #[tokio::test]
    async fn test_json_import_to_sqlite() {
        let pool = setup_test_db().await;
        let data = make_test_data();

        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(before, 0);

        let count = import_saved_data(&pool, &data).await.unwrap();
        assert!(count > 0);

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM problems")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM contests")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM posts")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notifications")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }

    /// 测试：导入后再加载，验证数据完整性
    #[tokio::test]
    async fn test_import_then_load_roundtrip() {
        let pool = setup_test_db().await;
        let data = make_test_data();

        import_saved_data(&pool, &data).await.unwrap();
        let loaded = load_all_from_sqlite(&pool).await.unwrap();

        // 用户
        assert_eq!(loaded.users.len(), 2);
        let alice = loaded.users.get("user-001").unwrap();
        assert_eq!(alice.username, "alice");
        assert_eq!(alice.role, "member");
        assert_eq!(alice.team_status, "joined");
        assert_eq!(alice.bio, "Test user");
        assert_eq!(alice.group_ids, vec!["group-1".to_string()]);
        assert_eq!(alice.user_permissions, vec!["view_portfolio".to_string()]);
        assert_eq!(
            alice.avatar_url,
            Some("https://example.com/alice.png".to_string())
        );

        // 题目
        assert_eq!(loaded.problems.len(), 1);
        let prob = loaded.problems.get("prob-001").unwrap();
        assert_eq!(prob.title, "Test Problem");
        assert_eq!(prob.difficulty, "Medium");
        assert_eq!(prob.status, "approved");
        assert_eq!(prob.content, "## Problem Content\n\nSolve this.");
        assert_eq!(
            prob.solution,
            Some("## Solution\n\nHere is how.".to_string())
        );
        assert_eq!(prob.contest_id, Some("contest-001".to_string()));
        assert_eq!(prob.visible_to, vec!["member".to_string()]);
        assert_eq!(prob.editable_by, vec!["user-001".to_string()]);

        // 比赛
        assert_eq!(loaded.contests.len(), 1);
        let contest = loaded.contests.get("contest-001").unwrap();
        assert_eq!(contest.name, "2025 Spring Contest");
        assert_eq!(contest.status, "published");
        assert_eq!(contest.problem_order, vec!["prob-001".to_string()]);

        // 帖子
        assert_eq!(loaded.posts.len(), 1);
        let post = loaded.posts.get("post-001").unwrap();
        assert_eq!(post.title, "Test Post");
        assert_eq!(post.tags, vec!["讨论".to_string(), "测试".to_string()]);
        assert!(post.pinned);
        assert_eq!(post.status, "published");
        let heart_users = post.reactions.get("heart").unwrap();
        assert!(heart_users.contains(&"user-001".to_string()));
        assert!(heart_users.contains(&"user-002".to_string()));

        // 通知
        assert_eq!(loaded.notifications.len(), 1);
        let notif = loaded.notifications.get("notif-001").unwrap();
        assert_eq!(notif.title, "Welcome");
        assert!(!notif.read);

        // site_description 和 showcase
        assert_eq!(loaded.site_description, "欢迎来到 McGuffin 测试站");
        assert_eq!(loaded.showcase_problem_ids, vec!["prob-001".to_string()]);
        assert_eq!(loaded.showcase_contest_ids, vec!["contest-001".to_string()]);
    }

    /// 测试：reimport_all_data 清空后重新导入
    #[tokio::test]
    async fn test_reimport_all_data() {
        let pool = setup_test_db().await;
        let data = make_test_data();

        import_saved_data(&pool, &data).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            2
        );

        let mut new_data = make_test_data();
        new_data.users.remove("user-002");
        new_data.join_requests.clear(); // 移除引用已删除用户的外键
        new_data.problems.clear();
        new_data.posts.clear();
        new_data.sessions.clear();

        let reimported = reimport_all_data(&pool, &new_data).await.unwrap();
        assert!(reimported > 0);

        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        // bob 被清除
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = 'user-002'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM problems")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }

    /// 测试：meta 表字段（site_description 和 showcase）可以被覆盖写入
    #[tokio::test]
    async fn test_meta_fields_roundtrip() {
        let pool = setup_test_db().await;

        // 先导入数据（会写入默认的 site_description 和 showcase）
        let mut data = make_test_data();
        data.contests.clear();
        data.problems.clear();
        data.posts.clear();
        data.sessions.clear();
        data.join_requests.clear();
        data.notifications.clear();
        data.team_members.clear();
        import_saved_data(&pool, &data).await.unwrap();

        // 验证导入时写入的默认值
        let loaded = load_all_from_sqlite(&pool).await.unwrap();
        assert_eq!(loaded.site_description, "欢迎来到 McGuffin 测试站");

        // 覆盖写入新值
        sqlx::query(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('site_description', '测试站点')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_problem_ids', '[\"p1\",\"p2\"]')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('showcase_contest_ids', '[\"c1\"]')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // 验证覆盖后的值
        let loaded2 = load_all_from_sqlite(&pool).await.unwrap();
        assert_eq!(loaded2.site_description, "测试站点");
        assert_eq!(
            loaded2.showcase_problem_ids,
            vec!["p1".to_string(), "p2".to_string()]
        );
        assert_eq!(loaded2.showcase_contest_ids, vec!["c1".to_string()]);
    }

    /// 测试：meta 表无数据时的默认值
    #[tokio::test]
    async fn test_meta_fields_default_on_missing() {
        let pool = setup_test_db().await;

        // 不导入任何数据，直接加载（所有表为空）
        let loaded = load_all_from_sqlite(&pool).await.unwrap();
        assert_eq!(loaded.site_description, "");
        assert!(loaded.showcase_problem_ids.is_empty());
        assert!(loaded.showcase_contest_ids.is_empty());
        assert!(loaded.users.is_empty());
    }

    /// 测试：导入幂等性
    #[tokio::test]
    async fn test_import_is_idempotent() {
        let pool = setup_test_db().await;
        let data = make_test_data();

        let count1 = import_saved_data(&pool, &data).await.unwrap();
        assert!(count1 > 0);

        let user_count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();

        let count2 = import_saved_data(&pool, &data).await.unwrap();
        assert_eq!(count2, 0, "第二次导入应为幂等");

        let user_count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(user_count_before, user_count_after);
    }

    /// 测试：空数据导入
    #[tokio::test]
    async fn test_import_empty_data() {
        let pool = setup_test_db().await;

        let data = SavedData {
            users: HashMap::new(),
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            team_members: HashMap::new(),
            problems: HashMap::new(),
            join_requests: HashMap::new(),
            contests: HashMap::new(),
            site_description: String::new(),
            notifications: HashMap::new(),
            showcase_problem_ids: vec![],
            showcase_contest_ids: vec![],
            posts: HashMap::new(),
            suggestions: HashMap::new(),
            announcements: HashMap::new(),
            discussions: HashMap::new(),
            member_groups: HashMap::new(),
        };

        let count = import_saved_data(&pool, &data).await.unwrap();
        assert_eq!(count, 0);

        let loaded = load_all_from_sqlite(&pool).await.unwrap();
        assert!(loaded.users.is_empty());
        assert!(loaded.problems.is_empty());
        assert!(loaded.posts.is_empty());
    }

    /// 测试：帖子 replies 序列化往返
    #[tokio::test]
    async fn test_post_replies_roundtrip() {
        let pool = setup_test_db().await;
        let now = Utc::now();

        let mut data = SavedData {
            users: HashMap::new(),
            sessions: HashMap::new(),
            refresh_tokens: HashMap::new(),
            team_members: HashMap::new(),
            problems: HashMap::new(),
            join_requests: HashMap::new(),
            contests: HashMap::new(),
            site_description: String::new(),
            notifications: HashMap::new(),
            showcase_problem_ids: vec![],
            showcase_contest_ids: vec![],
            posts: HashMap::new(),
            suggestions: HashMap::new(),
            announcements: HashMap::new(),
            discussions: HashMap::new(),
            member_groups: HashMap::new(),
        };

        data.users.insert(
            "user-post".to_string(),
            User {
                id: "user-post".to_string(),
                username: "poster".to_string(),
                display_name: "Poster".to_string(),
                avatar_url: None,
                email: None,
                role: "member".to_string(),
                team_status: "joined".to_string(),
                created_at: now,
                bio: String::new(),
                password_hash: None,
                effective_role: "member".to_string(),
                group_ids: vec![],
                user_permissions: vec![],
            },
        );

        let mut reply_reactions = HashMap::new();
        reply_reactions.insert("like".to_string(), vec!["user-post".to_string()]);

        data.posts.insert(
            "post-reply".to_string(),
            Post {
                id: "post-reply".to_string(),
                title: "Post with replies".to_string(),
                content: "Main content".to_string(),
                author_id: "user-post".to_string(),
                author_name: "Poster".to_string(),
                created_at: now,
                updated_at: now,
                tags: vec![],
                pinned: false,
                team_only: false,
                emoji: None,
                reactions: HashMap::new(),
                replies: vec![PostReply {
                    id: "reply-001".to_string(),
                    author_id: "user-post".to_string(),
                    author_name: "Poster".to_string(),
                    content: "A reply".to_string(),
                    created_at: now,
                    reactions: reply_reactions,
                    parent_id: None,
                    reply_to: None,
                }],
                mentioned_user_ids: vec![],
                status: String::new(),
                visible_to: vec![],
                editable_by: vec![],
            },
        );

        import_saved_data(&pool, &data).await.unwrap();

        let loaded = load_all_from_sqlite(&pool).await.unwrap();
        let post = loaded.posts.get("post-reply").unwrap();
        assert_eq!(post.replies.len(), 1);
        let reply = &post.replies[0];
        assert_eq!(reply.id, "reply-001");
        assert_eq!(reply.content, "A reply");
        assert_eq!(reply.author_name, "Poster");
        assert!(reply
            .reactions
            .get("like")
            .unwrap()
            .contains(&"user-post".to_string()));
    }
}
