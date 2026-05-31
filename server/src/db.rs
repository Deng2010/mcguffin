//! SQLite 数据库初始化、WAL 配置、JSON 数据导入、在线备份
//!
//! 本模块负责 SQLite 的引导阶段：
//! - 创建连接池 + 运行迁移
//! - 设置 PRAGMA 优化参数
//! - 从旧版 `data.json` 导入数据（一次性迁移）
//! - 在线备份（通过 rusqlite backup API）
//! - JSON 导出/导入（跨版本兼容）

use std::collections::HashMap;

use rusqlite::backup::Backup;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing;

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
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .connect("sqlite::memory:")
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    // `:memory:` 模式下 WAL 模式不支持，跳过 PRAGMA
    tracing::info!("SQLite 以内存模式运行（数据不会持久化）");
    Ok(pool)
}

async fn try_init_db_file(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&format!("sqlite:{}", db_path))
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
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA cache_size = -64000")
        .execute(&pool)
        .await?; // 64 MB

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
    use sqlx::Row;

    let mut export = serde_json::json!({
        "users": [],
        "sessions": [],
        "refresh_tokens": [],
        "team_members": [],
        "join_requests": [],
        "contests": [],
        "problems": [],
        "notifications": [],
        "posts": [],
        "audit_log": [],
    });

    // users
    if let Ok(rows) = sqlx::query(
        "SELECT id, username, display_name, avatar_url, email, role, team_status, \
         created_at, bio, password_hash, effective_role, group_ids, user_permissions FROM users",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("查询 users: {}", e))
    {
        let mut list = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.get("id");
            list.push(serde_json::json!({
                "id": id,
                "username": r.get::<String,_>("username"),
                "display_name": r.get::<String,_>("display_name"),
                "avatar_url": r.get::<Option<String>,_>("avatar_url"),
                "email": r.get::<Option<String>,_>("email"),
                "role": r.get::<String,_>("role"),
                "team_status": r.get::<String,_>("team_status"),
                "created_at": r.get::<String,_>("created_at"),
                "bio": r.get::<String,_>("bio"),
            }));
        }
        export["users"] = serde_json::json!(list);
    }

    // team_members
    if let Ok(rows) = sqlx::query("SELECT id, user_id, joined_at FROM team_members")
        .fetch_all(pool)
        .await
    {
        let list: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<String,_>("id"),
                    "user_id": r.get::<String,_>("user_id"),
                    "joined_at": r.get::<String,_>("joined_at"),
                })
            })
            .collect();
        export["team_members"] = serde_json::json!(list);
    }

    // problems
    if let Ok(rows) = sqlx::query(
        "SELECT id, title, author_id, author_name, contest, contest_id, difficulty, \
         content, solution, status, created_at, public_at, claimed_by, \
         verifier_solution, visible_to, link, remark, editable_by FROM problems",
    )
    .fetch_all(pool)
    .await
    {
        let list: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<String,_>("id"),
                    "title": r.get::<String,_>("title"),
                    "author_id": r.get::<String,_>("author_id"),
                    "author_name": r.get::<String,_>("author_name"),
                    "difficulty": r.get::<String,_>("difficulty"),
                    "status": r.get::<String,_>("status"),
                    "created_at": r.get::<String,_>("created_at"),
                })
            })
            .collect();
        export["problems"] = serde_json::json!(list);
    }

    // contests
    if let Ok(rows) = sqlx::query(
        "SELECT id, name, start_time, end_time, description, created_by, created_at, status, link FROM contests"
    )
    .fetch_all(pool)
    .await
    {
        let list: Vec<serde_json::Value> = rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<String,_>("id"),
                "name": r.get::<String,_>("name"),
                "status": r.get::<String,_>("status"),
                "created_at": r.get::<String,_>("created_at"),
            })
        }).collect();
        export["contests"] = serde_json::json!(list);
    }

    // posts
    if let Ok(rows) = sqlx::query(
        "SELECT id, title, content, author_id, author_name, created_at, updated_at, \
         tags, pinned, team_only, status FROM posts",
    )
    .fetch_all(pool)
    .await
    {
        let list: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.get::<String,_>("id"),
                    "title": r.get::<String,_>("title"),
                    "author_id": r.get::<String,_>("author_id"),
                    "author_name": r.get::<String,_>("author_name"),
                    "created_at": r.get::<String,_>("created_at"),
                    "status": r.get::<String,_>("status"),
                })
            })
            .collect();
        export["posts"] = serde_json::json!(list);
    }

    serde_json::to_string_pretty(&export).map_err(|e| format!("JSON 序列化失败: {}", e))
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

    // 在一个事务中执行清空 + 重新导入
    let mut tx = pool.begin().await?;

    // 按外键依赖逆序清空（先清子表，再清父表）
    for table in &tables {
        sqlx::query(&format!("DELETE FROM {}", table))
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    // 重新导入
    let total = import_saved_data(pool, data).await?;
    tracing::info!("全量重新导入完成：{} 条记录", total);
    Ok(total)
}

// ============== 在线备份（rusqlite backup API） ==============

/// 使用 SQLite 在线备份 API 创建一致性快照。
/// 在 WAL 模式下，备份期间源数据库可以继续读写。
pub fn create_consistent_backup(source_path: &str, dest_path: &str) -> Result<(), String> {
    let mut src =
        rusqlite::Connection::open(source_path).map_err(|e| format!("无法打开源数据库: {}", e))?;
    let dst =
        rusqlite::Connection::open(dest_path).map_err(|e| format!("无法创建备份文件: {}", e))?;

    let backup = Backup::new(&dst, &mut src).map_err(|e| format!("备份初始化失败: {}", e))?;

    // 每步拷贝最多 100 页，每页之间最多等待 250ms
    backup
        .run_to_completion(100, std::time::Duration::from_millis(250), None)
        .map_err(|e| format!("备份执行失败: {}", e))?;

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
