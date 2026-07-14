use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json;
use sqlx::Row;
use tokio::sync::{Mutex, RwLock};
use toml_edit::{DocumentMut, Item, Value as TomlValue};

use crate::db::{import_saved_data, init_db, load_all_from_sqlite, sqlite_has_data};
use crate::infra::config::{
    load_config, load_difficulty_config, load_discussion_emojis, load_discussion_tags,
    load_member_groups,
};
use crate::plugin::builtins::SystemInfoPlugin;
use crate::plugin::{PluginContext, PluginManager};
use crate::state::{AppState, ADMIN_USER_ID, resolve_config_path};
use crate::types::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct SavedData {
    pub(crate) users: HashMap<String, User>,
    /// token → SessionEntry (contains user_id + last_active).
    /// Custom deserializer handles both old format (token→String) and new (token→SessionEntry).
    #[serde(deserialize_with = "deserialize_sessions", default)]
    pub(crate) sessions: HashMap<String, SessionEntry>,
    #[serde(default)]
    pub(crate) refresh_tokens: HashMap<String, String>,
    pub(crate) team_members: HashMap<String, TeamMember>,
    pub(crate) problems: HashMap<String, Problem>,
    pub(crate) join_requests: HashMap<String, JoinRequest>,
    #[serde(default)]
    pub(crate) contests: HashMap<String, Contest>,
    #[serde(default)]
    pub(crate) site_description: String,
    #[serde(default)]
    pub(crate) notifications: HashMap<String, Notification>,
    #[serde(default)]
    pub(crate) showcase_problem_ids: Vec<String>,
    #[serde(default)]
    pub(crate) showcase_contest_ids: Vec<String>,

    // ── Unified posts (primary storage) ──
    #[serde(default)]
    pub(crate) posts: HashMap<String, Post>,

    // ── Permission Groups ──
    #[serde(default)]
    pub(crate) member_groups: HashMap<String, MemberGroup>,
}

/// Custom deserializer for sessions that handles both old format
/// (`HashMap<String, String>` — just user_id) and new format
/// (`HashMap<String, SessionEntry>` — object with user_id + last_active).
fn deserialize_sessions<'de, D>(deserializer: D) -> Result<HashMap<String, SessionEntry>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value = serde_json::Value::deserialize(deserializer)?;

    match value {
        serde_json::Value::Object(map) => {
            let mut sessions = HashMap::new();
            for (token, val) in map {
                match val {
                    // Old format: "token" → "user_id" (string)
                    serde_json::Value::String(user_id) => {
                        sessions.insert(
                            token,
                            SessionEntry {
                                user_id,
                                last_active: Utc::now(),
                            },
                        );
                    }
                    // New format: "token" → {"user_id": "...", "last_active": "..."}
                    serde_json::Value::Object(obj) => {
                        let user_id = obj
                            .get("user_id")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .ok_or_else(|| D::Error::custom("missing user_id in session entry"))?;
                        let last_active = obj
                            .get("last_active")
                            .and_then(|v| v.as_str())
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(Utc::now);
                        sessions.insert(
                            token,
                            SessionEntry {
                                user_id,
                                last_active,
                            },
                        );
                    }
                    _ => {}
                }
            }
            Ok(sessions)
        }
        _ => Ok(HashMap::new()),
    }
}

impl AppState {
    /// 将 HashMap 中所有数据同步写入 SQLite。
    /// 用于备份前确保 SQLite 包含最新数据。
    pub async fn sync_to_db(&self) {
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.db)
            .await;
    }

    const MAX_SESSIONS_PER_USER: usize = 3;

    /// 记录审计日志条目到 SQLite。
    pub async fn log_audit(&self, entry: AuditEntry) {
        let _ = sqlx::query(
            "INSERT INTO audit_log (timestamp, user_id, user_name, action, resource, result, reason) \
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.user_id)
        .bind(&entry.user_name)
        .bind(&entry.action)
        .bind(&entry.resource)
        .bind(&entry.result)
        .bind(&entry.reason)
        .execute(&self.db)
        .await;
    }

    /// 设置 refresh token（双写：HashMap + SQLite）
    pub async fn set_refresh_token(&self, token: String, user_id: String) {
        {
            let mut rt = self.refresh_tokens.write().await;
            rt.insert(token.clone(), user_id.clone());
        }
        let _ = sqlx::query("INSERT OR REPLACE INTO refresh_tokens (token, user_id) VALUES (?, ?)")
            .bind(&token)
            .bind(&user_id)
            .execute(&self.db)
            .await;
    }

    /// 删除 refresh token（双写）
    pub async fn remove_refresh_token(&self, token: &str) {
        {
            let mut rt = self.refresh_tokens.write().await;
            rt.remove(token);
        }
        let _ = sqlx::query("DELETE FROM refresh_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await;
    }

    /// 清除指定用户的所有 refresh token（双写）
    pub async fn clear_user_refresh_tokens(&self, user_id: &str) {
        {
            let mut rt = self.refresh_tokens.write().await;
            rt.retain(|_, uid| uid != user_id);
        }
        let _ = sqlx::query("DELETE FROM refresh_tokens WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// Create a session for the given user, automatically evicting the oldest
    /// session if they already have MAX_SESSIONS_PER_USER sessions.
    /// 双写：HashMap + SQLite
    pub async fn create_session(&self, user_id: &str) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_rfc = now.to_rfc3339();

        // HashMap 写入
        {
            let mut sessions = self.sessions.write().await;

            // Collect sessions for this user, sorted oldest-first
            let mut user_sessions: Vec<(String, chrono::DateTime<Utc>)> = sessions
                .iter()
                .filter(|(_, e)| e.user_id == user_id)
                .map(|(token, e)| (token.clone(), e.last_active))
                .collect();
            user_sessions.sort_by_key(|(_, t)| *t);

            // Evict oldest sessions beyond the limit
            while user_sessions.len() >= Self::MAX_SESSIONS_PER_USER {
                let (oldest_token, _) = user_sessions.remove(0);
                sessions.remove(&oldest_token);
            }

            sessions.insert(
                token.clone(),
                SessionEntry {
                    user_id: user_id.to_string(),
                    last_active: now,
                },
            );
        }

        // SQLite 写入（双写）
        let _ = sqlx::query("INSERT INTO sessions (token, user_id, last_active) VALUES (?, ?, ?)")
            .bind(&token)
            .bind(user_id)
            .bind(&now_rfc)
            .execute(&self.db)
            .await;

        // 清理旧 session（SQLite 端）
        let _ = sqlx::query(
            "DELETE FROM sessions WHERE token IN (\
             SELECT token FROM sessions WHERE user_id = ? \
             ORDER BY last_active ASC \
             LIMIT -1 OFFSET ?
            )",
        )
        .bind(user_id)
        .bind(Self::MAX_SESSIONS_PER_USER as i32 - 1)
        .execute(&self.db)
        .await;

        token
    }

    /// 删除 session（双写）
    pub async fn remove_session(&self, token: &str) {
        self.sessions.write().await.remove(token);
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&self.db)
            .await;
    }

    /// 插入通知（双写）
    pub async fn insert_notification(&self, notification: &Notification) {
        self.notifications
            .write()
            .await
            .insert(notification.id.clone(), notification.clone());
        let _ = sqlx::query(
            "INSERT INTO notifications (id, user_id, title, body, read, created_at, link) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&notification.id)
        .bind(&notification.user_id)
        .bind(&notification.title)
        .bind(&notification.body)
        .bind(notification.read as i32)
        .bind(notification.created_at.to_rfc3339())
        .bind(&notification.link)
        .execute(&self.db)
        .await;
    }

    /// 标记通知为已读（双写）
    pub async fn mark_notification_read(&self, notification_id: &str) {
        if let Some(n) = self.notifications.write().await.get_mut(notification_id) {
            n.read = true;
        }
        let _ = sqlx::query("UPDATE notifications SET read = 1 WHERE id = ?")
            .bind(notification_id)
            .execute(&self.db)
            .await;
    }

    /// 标记用户所有通知为已读（双写）
    pub async fn mark_all_user_notifications_read(&self, user_id: &str) {
        let mut notifications = self.notifications.write().await;
        for n in notifications.values_mut() {
            if n.user_id == user_id && !n.read {
                n.read = true;
            }
        }
        drop(notifications);
        let _ = sqlx::query("UPDATE notifications SET read = 1 WHERE user_id = ? AND read = 0")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    // ===== users 双写辅助方法 =====

    /// 插入或替换用户（双写）
    pub async fn upsert_user(&self, user: &User) {
        tracing::info!("upsert_user 开始: id={}", &user.id[..8.min(user.id.len())]);
        // 先写 HashMap，完成后立即释放写锁
        // tokio::sync::Mutex 的 lock() 保证排队公平性，不会写者饿死
        // 使用 try_lock 尝试避免阻塞
        let mut users = if let Ok(guard) = self.users.try_lock() {
            guard
        } else {
            tracing::warn!("upsert_user: 锁被占用，排队等待...");
            self.users.lock().await
        };
        users.insert(user.id.clone(), user.clone());
        drop(users);
        tracing::info!("upsert_user HashMap写入完成");
        // 再写 SQLite（不持有 HashMap 锁）
        tracing::info!("upsert_user 开始 sqlx query...");
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO users \
             (id, username, display_name, avatar_url, email, role, team_status, \
              created_at, bio, password_hash, effective_role, group_ids, user_permissions) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
        .execute(&self.db)
        .await;
    }

    /// 更新用户特定字段（双写）
    pub async fn update_user_field(&self, user_id: &str, field: &str, value: String) {
        // HashMap 更新
        {
            let mut users = self.users.lock().await;
            if let Some(u) = users.get_mut(user_id) {
                match field {
                    "role" => u.role = value.clone(),
                    "team_status" => u.team_status = value.clone(),
                    "display_name" => u.display_name = value.clone(),
                    "avatar_url" => u.avatar_url = Some(value.clone()),
                    "bio" => u.bio = value.clone(),
                    "email" => u.email = Some(value.clone()),
                    "username" => u.username = value.clone(),
                    "password_hash" => u.password_hash = Some(value.clone()),
                    _ => {}
                }
            }
        }
        // SQLite 更新（使用动态 SQL，字段已校验）
        let _ = sqlx::query(&format!("UPDATE users SET {} = ? WHERE id = ?", field))
            .bind(&value)
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 更新用户多个字段（双写）
    pub async fn update_user(&self, user: &User) {
        // 先写 HashMap
        {
            let mut users = self.users.lock().await;
            users.insert(user.id.clone(), user.clone());
        }
        let _ = sqlx::query(
            "UPDATE users SET username=?, display_name=?, avatar_url=?, email=?, \
             role=?, team_status=?, bio=?, password_hash=?, effective_role=?, \
             group_ids=?, user_permissions=? WHERE id=?",
        )
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.email)
        .bind(&user.role)
        .bind(&user.team_status)
        .bind(&user.bio)
        .bind(&user.password_hash)
        .bind(&user.effective_role)
        .bind(serde_json::to_string(&user.group_ids).unwrap_or_default())
        .bind(serde_json::to_string(&user.user_permissions).unwrap_or_default())
        .bind(&user.id)
        .execute(&self.db)
        .await;
    }

    /// 删除用户（双写）
    pub async fn delete_user(&self, user_id: &str) {
        self.users.lock().await.remove(user_id);
        let _ = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 从用户列表中移除指定 group_id（双写）
    pub async fn remove_group_from_all_users(&self, group_id: &str) {
        {
            let mut users = self.users.lock().await;
            for u in users.values_mut() {
                u.group_ids.retain(|g| g != group_id);
            }
        }
        // SQLite 端需要用 JSON 函数处理
        // 由于 SQLite JSON 操作较复杂，这里使用读取-修改-写入循环
        if let Ok(rows) =
            sqlx::query("SELECT id, group_ids FROM users WHERE json_array_length(group_ids) > 0")
                .fetch_all(&self.db)
                .await
        {
            for row in rows {
                let uid: String = row.get("id");
                let gids: String = row.get("group_ids");
                let mut list: Vec<String> = serde_json::from_str(&gids).unwrap_or_default();
                list.retain(|g| g != group_id);
                let _ = sqlx::query("UPDATE users SET group_ids = ? WHERE id = ?")
                    .bind(serde_json::to_string(&list).unwrap_or_default())
                    .bind(&uid)
                    .execute(&self.db)
                    .await;
            }
        }
    }

    // ===== posts 双写辅助方法 =====

    /// 插入或替换帖子（双写）
    pub async fn upsert_post(&self, post: &Post) {
        self.posts
            .write()
            .await
            .insert(post.id.clone(), post.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO posts \
             (id, title, content, author_id, author_name, created_at, updated_at, \
              tags, pinned, team_only, emoji, reactions, replies, \
              mentioned_user_ids, status, visible_to, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&post.id)
        .bind(&post.title)
        .bind(&post.content)
        .bind(&post.author_id)
        .bind(&post.author_name)
        .bind(post.created_at.to_rfc3339())
        .bind(post.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&post.tags).unwrap_or_default())
        .bind(post.pinned as i32)
        .bind(post.team_only as i32)
        .bind(&post.emoji)
        .bind(serde_json::to_string(&post.reactions).unwrap_or_default())
        .bind(serde_json::to_string(&post.replies).unwrap_or_default())
        .bind(serde_json::to_string(&post.mentioned_user_ids).unwrap_or_default())
        .bind(&post.status)
        .bind(serde_json::to_string(&post.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&post.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 删除帖子（双写）
    pub async fn delete_post_by_id(&self, post_id: &str) {
        self.posts.write().await.remove(post_id);
        let _ = sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(post_id)
            .execute(&self.db)
            .await;
    }

    // ===== problems 双写辅助方法 =====

    /// 插入题目（双写）
    pub async fn insert_problem(&self, problem: &Problem) {
        self.problems
            .write()
            .await
            .insert(problem.id.clone(), problem.clone());
        if let Err(e) = sqlx::query(
            "INSERT OR REPLACE INTO problems
             (id, title, author_id, author_name, contest, contest_id, difficulty, \
              content, solution, status, created_at, public_at, claimed_by, \
              verifier_solution, visible_to, link, remark, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&problem.id)
        .bind(&problem.title)
        .bind(&problem.author_id)
        .bind(&problem.author_name)
        .bind(&problem.contest)
        .bind(&problem.contest_id)
        .bind(&problem.difficulty)
        .bind(&problem.content)
        .bind(&problem.solution)
        .bind(&problem.status)
        .bind(problem.created_at.to_rfc3339())
        .bind(problem.public_at.map(|t| t.to_rfc3339()))
        .bind(&problem.claimed_by)
        .bind(&problem.verifier_solution)
        .bind(serde_json::to_string(&problem.visible_to).unwrap_or_default())
        .bind(&problem.link)
        .bind(&problem.remark)
        .bind(serde_json::to_string(&problem.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await
        {
            tracing::error!("Failed to insert problem {}: {}", problem.id, e);
        }
    }

    /// 更新题目单个字段（双写）
    pub async fn update_problem_field<T: ToString + Send>(
        &self,
        problem_id: &str,
        field: &str,
        value: T,
    ) {
        let val_str = value.to_string();
        {
            let mut problems = self.problems.write().await;
            if let Some(p) = problems.get_mut(problem_id) {
                match field {
                    "status" => p.status = val_str.clone(),
                    "claimed_by" => {
                        p.claimed_by = if val_str.is_empty() {
                            None
                        } else {
                            Some(val_str.clone())
                        }
                    }
                    "verifier_solution" => {
                        p.verifier_solution = if val_str.is_empty() {
                            None
                        } else {
                            Some(val_str.clone())
                        }
                    }
                    "public_at" => p.public_at = None, // handled separately
                    _ => {}
                }
            }
        }
        let _ = sqlx::query(&format!("UPDATE problems SET {} = ? WHERE id = ?", field))
            .bind(&val_str)
            .bind(problem_id)
            .execute(&self.db)
            .await;
    }

    /// 删除题目（双写）
    pub async fn delete_problem_by_id(&self, problem_id: &str) {
        self.problems.write().await.remove(problem_id);
        let _ = sqlx::query("DELETE FROM problems WHERE id = ?")
            .bind(problem_id)
            .execute(&self.db)
            .await;
    }

    /// 清除所有题目中指定 contest 的引用（双写）
    pub async fn clear_contest_from_problems(&self, contest_id: &str) {
        {
            let mut problems = self.problems.write().await;
            for p in problems.values_mut() {
                if p.contest_id.as_deref() == Some(contest_id) {
                    p.contest = String::new();
                    p.contest_id = None;
                }
            }
        }
        let _ =
            sqlx::query("UPDATE problems SET contest = '', contest_id = NULL WHERE contest_id = ?")
                .bind(contest_id)
                .execute(&self.db)
                .await;
    }

    // ===== contests 双写辅助方法 =====

    /// 插入比赛（双写）
    pub async fn insert_contest(&self, contest: &Contest) {
        self.contests
            .write()
            .await
            .insert(contest.id.clone(), contest.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO contests
             (id, name, start_time, end_time, description, created_by, created_at, \
              status, link, problem_order, visible_to, editable_by) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&contest.id)
        .bind(&contest.name)
        .bind(&contest.start_time)
        .bind(&contest.end_time)
        .bind(&contest.description)
        .bind(&contest.created_by)
        .bind(contest.created_at.to_rfc3339())
        .bind(&contest.status)
        .bind(&contest.link)
        .bind(serde_json::to_string(&contest.problem_order).unwrap_or_default())
        .bind(serde_json::to_string(&contest.visible_to).unwrap_or_default())
        .bind(serde_json::to_string(&contest.editable_by).unwrap_or_default())
        .execute(&self.db)
        .await;
    }

    /// 删除比赛（双写）
    pub async fn delete_contest_by_id(&self, contest_id: &str) {
        self.contests.write().await.remove(contest_id);
        let _ = sqlx::query("DELETE FROM contests WHERE id = ?")
            .bind(contest_id)
            .execute(&self.db)
            .await;
    }

    // ===== team_members 双写辅助方法 =====

    /// 插入团队成员（双写）
    pub async fn insert_team_member(&self, member: &TeamMember) {
        self.team_members
            .write()
            .await
            .insert(member.id.clone(), member.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO team_members (id, user_id, joined_at) VALUES (?, ?, ?)",
        )
        .bind(&member.id)
        .bind(&member.user_id)
        .bind(&member.joined_at)
        .execute(&self.db)
        .await;
    }

    /// 按成员 ID 删除团队成员（双写）
    pub async fn remove_team_member_by_id(&self, member_id: &str) {
        self.team_members.write().await.remove(member_id);
        let _ = sqlx::query("DELETE FROM team_members WHERE id = ?")
            .bind(member_id)
            .execute(&self.db)
            .await;
    }

    /// 按用户 ID 删除团队成员（双写）
    pub async fn remove_team_member_by_user(&self, user_id: &str) {
        // 先从 HashMap 移除
        {
            let mut members = self.team_members.write().await;
            members.retain(|_, m| m.user_id != user_id);
        }
        let _ = sqlx::query("DELETE FROM team_members WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.db)
            .await;
    }

    /// 判断用户是否为团队成员
    pub async fn is_team_member(&self, user_id: &str) -> bool {
        self.team_members
            .read()
            .await
            .values()
            .any(|m| m.user_id == user_id)
    }

    // ===== join_requests 双写辅助方法 =====

    /// 插入入队申请（双写）
    pub async fn insert_join_request(&self, request: &JoinRequest) {
        self.join_requests
            .write()
            .await
            .insert(request.id.clone(), request.clone());
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO join_requests \
             (id, user_id, user_name, user_email, reason, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&request.id)
        .bind(&request.user_id)
        .bind(&request.user_name)
        .bind(&request.user_email)
        .bind(&request.reason)
        .bind(&request.status)
        .bind(request.created_at.to_rfc3339())
        .execute(&self.db)
        .await;
    }

    /// 更新入队申请状态（双写）
    pub async fn update_join_request_status(&self, request_id: &str, status: &str) {
        if let Some(r) = self.join_requests.write().await.get_mut(request_id) {
            r.status = status.to_string();
        }
        let _ = sqlx::query("UPDATE join_requests SET status = ? WHERE id = ?")
            .bind(status)
            .bind(request_id)
            .execute(&self.db)
            .await;
    }

    /// 从 SQLite 重新加载所有内存状态。
    /// 用于备份恢复后同步内存数据。
    pub async fn reload(&self) {
        match crate::db::load_all_from_sqlite(&self.db).await {
            Ok(data) => {
                *self.users.lock().await = data.users;
                *self.sessions.write().await = data.sessions;
                *self.refresh_tokens.write().await = data.refresh_tokens;
                *self.team_members.write().await = data.team_members;
                *self.problems.write().await = data.problems;
                *self.join_requests.write().await = data.join_requests;
                *self.contests.write().await = data.contests;
                *self.site_description.write().await = data.site_description;
                *self.notifications.write().await = data.notifications;
                *self.showcase_problem_ids.write().await = data.showcase_problem_ids;
                *self.showcase_contest_ids.write().await = data.showcase_contest_ids;
                // member_groups come from config.toml, not SQLite
                let reloaded_config = crate::infra::config::load_config();
                *self.member_groups.write().await =
                    crate::infra::config::load_member_groups(&reloaded_config);

                *self.posts.write().await = data.posts;

                // discussion_tags and discussion_emojis stay from config.toml
                tracing::info!("内存状态已从 SQLite 重新加载");
            }
            Err(e) => {
                tracing::error!("从 SQLite 重新加载数据失败: {}", e);
            }
        }
    }

    pub(crate) fn default_team_members() -> HashMap<String, TeamMember> {
        HashMap::new()
    }

    pub(crate) fn default_problems() -> HashMap<String, Problem> {
        HashMap::new()
    }
}

impl AppState {
    pub async fn new() -> Self {
        // Load config from /usr/share/mcguffin/config.toml
        let mut config = load_config();
        let difficulty_config = load_difficulty_config(&config);

        // Migration: if config.toml has no [permissions] section, write default role permissions
        {
            let raw_config = std::fs::read_to_string(resolve_config_path()).unwrap_or_default();
            let has_permissions_section =
                raw_config.contains("\n[permissions]") || raw_config.starts_with("[permissions]");
            if !has_permissions_section {
                tracing::info!(
                    "No [permissions] section in config.toml, writing default permissions"
                );
                let defaults = default_role_permissions();
                if let Ok(raw) = std::fs::read_to_string(resolve_config_path()) {
                    if let Ok(mut doc) = DocumentMut::from_str(&raw) {
                        // Ensure [permissions] table exists
                        if doc.get("permissions").is_none() {
                            doc["permissions"] = Item::Table(toml_edit::Table::new());
                        }
                        // Write [permissions.roles]
                        doc["permissions"]["roles"] = Item::Table(toml_edit::Table::new());
                        if let Some(roles_t) = doc["permissions"]["roles"].as_table_mut() {
                            for (role, perms) in &defaults {
                                if !perms.is_empty() {
                                    let arr = toml_edit::Array::from_iter(
                                        perms.iter().map(|p| TomlValue::from(p.as_str())),
                                    );
                                    roles_t[role] = Item::Value(TomlValue::Array(arr));
                                }
                            }
                        }
                        let _ = std::fs::write(resolve_config_path(), doc.to_string());
                        tracing::info!("Default permissions written to config.toml");
                    }
                }
                // Reload config to pick up the newly written defaults
                config = load_config();
            }
        }

        let site_version = env!("CARGO_PKG_VERSION").to_string();

        // ── SQLite 初始化 ──
        let data_dir = std::env::var("MCGUFFIN_DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let db_path = std::path::PathBuf::from(&data_dir).join("mcguffin_data.db");
        let json_path = db_path.with_extension("json");
        let db_path_str = db_path.to_string_lossy().to_string();
        let db = init_db(&db_path_str)
            .await
            .expect("SQLite 初始化失败，请检查数据库文件路径和权限");

        // ── 数据加载策略：SQLite 为主，JSON 仅用于首次迁移 ──
        let mut saved: Option<SavedData> = None;
        let has_data = sqlite_has_data(&db).await;
        if has_data {
            // SQLite 已有数据 → 从数据库加载
            tracing::info!("从 SQLite 加载数据...");
            match load_all_from_sqlite(&db).await {
                Ok(data) => {
                    saved = Some(data);
                    tracing::info!("SQLite 数据加载成功");
                }
                Err(e) => {
                    tracing::warn!("从 SQLite 加载数据失败: {}", e);
                }
            }
        }

        // SQLite 为空 → 尝试从 JSON 迁移
        let json_path_str = json_path.to_string_lossy().to_string();
        if saved.is_none() {
            if let Ok(json) = std::fs::read_to_string(&json_path) {
                if let Ok(data) = serde_json::from_str::<SavedData>(&json) {
                    tracing::info!(
                        "从 JSON 文件 {} 加载数据，准备导入 SQLite...",
                        json_path_str
                    );
                    match import_saved_data(&db, &data).await {
                        Ok(n) if n > 0 => {
                            tracing::info!("已从 {} 导入 {} 条记录到 SQLite", json_path_str, n);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::warn!("从 JSON 导入 SQLite 失败: {}", e);
                        }
                    }
                    saved = Some(data);
                }
            }
        }

        // Discussion tags and emojis come from config.toml, not saved data
        let discussion_tags = load_discussion_tags(&config);
        let discussion_emojis = load_discussion_emojis(&config);

        let (
            mut users,
            sessions,
            refresh_tokens,
            mut team_members,
            problems,
            join_requests,
            contests,
            site_description,
            notifications,
            showcase_problem_ids,
            showcase_contest_ids,
            posts,
        ) = if let Some(data) = saved {
            tracing::info!("Loaded state from JSON: {}", json_path_str);

            let p = data.posts;

            // Migration: if data.json has member_groups, write to config.toml
            if !data.member_groups.is_empty() && config.permission_groups.is_empty() {
                tracing::info!(
                    "Migrating {} member groups from data.json to config.toml",
                    data.member_groups.len()
                );
                // Write to config.toml via the admin helper
                let groups_json: Vec<serde_json::Value> = data
                    .member_groups
                    .values()
                    .map(|g| {
                        serde_json::json!({
                            "id": g.id,
                            "name": g.name,
                            "permissions": g.permissions,
                        })
                    })
                    .collect();
                if let Ok(raw) = std::fs::read_to_string(resolve_config_path()) {
                    if let Ok(mut doc) = DocumentMut::from_str(&raw) {
                        if let Some(perms_root) =
                            doc.get_mut("permissions").and_then(|s| s.as_table_mut())
                        {
                            // Clear old groups section
                            if let Some(groups_t) =
                                perms_root.get_mut("groups").and_then(|s| s.as_table_mut())
                            {
                                let keys: Vec<String> =
                                    groups_t.iter().map(|(k, _)| k.to_string()).collect();
                                for k in keys {
                                    groups_t.remove(&k);
                                }
                            } else {
                                // Ensure groups sub-table exists
                                perms_root["groups"] = Item::Table(toml_edit::Table::new());
                            }
                            // Write migrated groups
                            if let Some(groups_t) =
                                perms_root.get_mut("groups").and_then(|s| s.as_table_mut())
                            {
                                for g in &groups_json {
                                    let id = g.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let name = g.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                    let perms = g.get("permissions").and_then(|v| v.as_array());
                                    if id.is_empty() || name.is_empty() {
                                        continue;
                                    }
                                    let mut it = toml_edit::InlineTable::new();
                                    it.insert("name", TomlValue::from(name));
                                    if let Some(arr) = perms {
                                        let t_arr = toml_edit::Array::from_iter(
                                            arr.iter()
                                                .filter_map(|v| v.as_str().map(TomlValue::from)),
                                        );
                                        it.insert("permissions", TomlValue::Array(t_arr));
                                    }
                                    groups_t[id] = Item::Value(TomlValue::InlineTable(it));
                                }
                            }
                        }
                        let _ = std::fs::write(resolve_config_path(), doc.to_string());
                    }
                }
                // Also update in-memory member_groups
                // (loaded from config.toml again below — handled by load_member_groups fallback)
            }

            (
                data.users,
                data.sessions,
                data.refresh_tokens,
                data.team_members,
                data.problems,
                data.join_requests,
                data.contests,
                data.site_description,
                data.notifications,
                data.showcase_problem_ids,
                data.showcase_contest_ids,
                p,
            )
        } else {
            tracing::info!("No saved state, using default seed data");
            (
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                Self::default_team_members(),
                Self::default_problems(),
                HashMap::new(),
                HashMap::new(),
                String::new(),
                HashMap::new(),
                Vec::new(),
                Vec::new(),
                HashMap::new(),
            )
        };

        // Member groups come from config.toml (already migrated from data.json if needed)
        // Reload config after potential migration
        let config = load_config();
        let member_groups = load_member_groups(&config);

        // Always ensure superadmin user exists AND has correct role
        let admin_display_name = &config.admin.display_name;
        users.entry(ADMIN_USER_ID.to_string()).or_insert(User {
            id: ADMIN_USER_ID.to_string(),
            username: "admin".to_string(),
            display_name: admin_display_name.clone(),
            avatar_url: None,
            email: None,
            role: "superadmin".to_string(),
            team_status: "joined".to_string(),
            created_at: Utc::now(),
            bio: String::new(),
            password_hash: None,
            effective_role: "superadmin".to_string(),
            group_ids: Vec::new(),
            user_permissions: Vec::new(),
        });
        // Force update role to superadmin (in case loaded from old data)
        if let Some(u) = users.get_mut(ADMIN_USER_ID) {
            u.role = "superadmin".to_string();
            u.display_name = admin_display_name.clone();
        }

        // Migration: convert role="pending" to role="guest" (pending role removed)
        let mut pending_count = 0;
        for u in users.values_mut() {
            if u.role == "pending" {
                u.role = "guest".to_string();
                pending_count += 1;
            }
        }
        if pending_count > 0 {
            tracing::info!(
                "Migrated {} users from role=pending to role=guest",
                pending_count
            );
        }

        // Always ensure superadmin is a team member AND has correct role
        team_members
            .entry(ADMIN_USER_ID.to_string())
            .or_insert(TeamMember {
                id: ADMIN_USER_ID.to_string(),
                user_id: ADMIN_USER_ID.to_string(),
                joined_at: DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc)
                    .format("%Y-%m-%d")
                    .to_string(),
            });

        let redirect_uri = format!("{}/api/oauth/callback", config.server.site_url);

        // Load role→permissions from config or use defaults
        let role_permissions: HashMap<String, Vec<String>> = if !config.permissions.is_empty() {
            // Validate permission names in config
            for (role, perms) in &config.permissions {
                for p in perms {
                    if p != PERM_WILDCARD && !perms::ALL.contains(&p.as_str()) {
                        tracing::warn!(
                            "配置中的权限名「{}」（角色: {}）不在已知权限列表中，将被忽略",
                            p,
                            role
                        );
                    }
                }
            }
            config.permissions.clone()
        } else {
            default_role_permissions()
        };

        let plugins_dir = std::env::var("MCGUFFIN_PLUGINS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("plugins"));
        let _ = std::fs::create_dir_all(&plugins_dir);

        let app_state = Self {
            users: Arc::new(Mutex::new(users)),
            sessions: Arc::new(RwLock::new(sessions)),
            refresh_tokens: Arc::new(RwLock::new(refresh_tokens)),
            team_members: Arc::new(RwLock::new(team_members)),
            problems: Arc::new(RwLock::new(problems)),
            join_requests: Arc::new(RwLock::new(join_requests)),
            contests: Arc::new(RwLock::new(contests)),
            cpoauth_client_id: config.oauth.cp_client_id,
            cpoauth_client_secret: config.oauth.cp_client_secret,
            cpoauth_redirect_uri: redirect_uri,
            admin_password: Arc::new(RwLock::new(config.admin.password)),
            site_name: config
                .site
                .name
                .clone()
                .unwrap_or_else(|| "McGuffin".to_string()),
            site_title: config
                .site
                .title
                .unwrap_or_else(|| config.site.name.unwrap_or_else(|| "McGuffin".to_string())),
            site_version,
            site_description: Arc::new(RwLock::new(site_description)),
            site_url: config.server.site_url,
            db_path: db_path_str,
            difficulty: Arc::new(RwLock::new(difficulty_config.clone())),
            posts: Arc::new(RwLock::new(posts)),
            notifications: Arc::new(RwLock::new(notifications)),
            showcase_problem_ids: Arc::new(RwLock::new(showcase_problem_ids)),
            showcase_contest_ids: Arc::new(RwLock::new(showcase_contest_ids)),
            difficulty_order: Arc::new(RwLock::new(
                config.site.difficulty_order.clone().unwrap_or_else(|| {
                    let mut keys: Vec<String> = difficulty_config.levels.keys().cloned().collect();
                    keys.sort();
                    keys
                }),
            )),
            discussion_tags: Arc::new(RwLock::new(discussion_tags)),
            discussion_emojis: Arc::new(RwLock::new(discussion_emojis)),
            role_permissions: Arc::new(RwLock::new(role_permissions)),
            member_groups: Arc::new(RwLock::new(member_groups)),
            db,
            backup_directory: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("创建 HTTP 客户端失败"),
            plugins: PluginManager::new(plugins_dir),
        };

        {
            let plugin_ctx = PluginContext {
                db: app_state.db.clone(),
                http_client: app_state.http_client.clone(),
                plugin_data: app_state.plugins.plugin_data.clone(),
                base_url: app_state.site_url.clone(),
            };
            if let Err(e) = app_state
                .plugins
                .register_builtin(Box::new(SystemInfoPlugin), plugin_ctx)
                .await
            {
                tracing::warn!("Failed to register SystemInfoPlugin: {}", e);
            }
            app_state.plugins.start_hot_reload_task().await;
        }

        // SQLite 是权威数据源，确保 admin 存在于数据库中
        {
            let admin_user = app_state.users.lock().await.get(ADMIN_USER_ID).cloned();
            if let Some(ref admin) = admin_user {
                let _ = sqlx::query(
                    "INSERT OR REPLACE INTO users (id, username, display_name, avatar_url, email, role, team_status, \
                     created_at, bio, password_hash, effective_role, group_ids, user_permissions) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(&admin.id)
                .bind(&admin.username)
                .bind(&admin.display_name)
                .bind(&admin.avatar_url)
                .bind(&admin.email)
                .bind(&admin.role)
                .bind(&admin.team_status)
                .bind(admin.created_at.to_rfc3339())
                .bind(&admin.bio)
                .bind(&admin.password_hash)
                .bind(&admin.effective_role)
                .bind(serde_json::to_string(&admin.group_ids).unwrap_or_default())
                .bind(serde_json::to_string(&admin.user_permissions).unwrap_or_default())
                .execute(&app_state.db)
                .await;
            }
        }
        {
            let admin_member = app_state
                .team_members
                .read()
                .await
                .get(ADMIN_USER_ID)
                .cloned();
            if let Some(ref m) = admin_member {
                let _ = sqlx::query(
                    "INSERT OR REPLACE INTO team_members (id, user_id, joined_at) VALUES (?, ?, ?)",
                )
                .bind(&m.id)
                .bind(&m.user_id)
                .bind(&m.joined_at)
                .execute(&app_state.db)
                .await;
            }
        }

        // 启动完成，启用 FK 约束以保障正常运行的引用完整性
        let _ = sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&app_state.db)
            .await;

        app_state
    }
}

impl Default for AppState {
    fn default() -> Self {
        // Default 仅供少数边缘场景使用（如某些推导 trait），
        // 正常情况下应使用 AppState::new().await
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(Self::new()))
    }
}
