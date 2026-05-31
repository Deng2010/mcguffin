-- McGuffin 初始数据库 Schema
-- 所有表使用 IF NOT EXISTS 以支持重复迁移

-- ===== 元数据 =====
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- ===== 用户 =====
CREATE TABLE IF NOT EXISTS users (
    id               TEXT PRIMARY KEY,
    username         TEXT NOT NULL UNIQUE,
    display_name     TEXT NOT NULL,
    avatar_url       TEXT,
    email            TEXT,
    role             TEXT NOT NULL DEFAULT 'guest',
    team_status      TEXT NOT NULL DEFAULT 'none',
    created_at       TEXT NOT NULL,
    bio              TEXT NOT NULL DEFAULT '',
    password_hash    TEXT,
    effective_role   TEXT NOT NULL DEFAULT 'guest',
    group_ids        TEXT NOT NULL DEFAULT '[]',
    user_permissions TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- ===== 会话 =====
CREATE TABLE IF NOT EXISTS sessions (
    token       TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_active TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);

-- ===== 刷新令牌 =====
CREATE TABLE IF NOT EXISTS refresh_tokens (
    token   TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);

-- ===== 团队成员 =====
CREATE TABLE IF NOT EXISTS team_members (
    id        TEXT PRIMARY KEY,
    user_id   TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    joined_at TEXT NOT NULL
);

-- ===== 入队申请 =====
CREATE TABLE IF NOT EXISTS join_requests (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_name  TEXT NOT NULL,
    user_email TEXT,
    reason     TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_join_requests_status ON join_requests(status);
CREATE INDEX IF NOT EXISTS idx_join_requests_user_id ON join_requests(user_id);

-- ===== 比赛 =====
CREATE TABLE IF NOT EXISTS contests (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    start_time    TEXT,
    end_time      TEXT,
    description   TEXT NOT NULL DEFAULT '',
    created_by    TEXT NOT NULL REFERENCES users(id),
    created_at    TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'draft',
    link          TEXT,
    problem_order TEXT NOT NULL DEFAULT '[]',
    visible_to    TEXT NOT NULL DEFAULT '[]',
    editable_by   TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_contests_status ON contests(status);

-- ===== 题目 =====
CREATE TABLE IF NOT EXISTS problems (
    id                TEXT PRIMARY KEY,
    title             TEXT NOT NULL,
    author_id         TEXT NOT NULL REFERENCES users(id),
    author_name       TEXT NOT NULL,
    contest           TEXT,
    contest_id        TEXT REFERENCES contests(id) ON DELETE SET NULL,
    difficulty        TEXT NOT NULL,
    content           TEXT NOT NULL,
    solution          TEXT,
    status            TEXT NOT NULL DEFAULT 'pending',
    created_at        TEXT NOT NULL,
    public_at         TEXT,
    claimed_by        TEXT REFERENCES users(id),
    verifier_solution TEXT,
    visible_to        TEXT NOT NULL DEFAULT '[]',
    link              TEXT,
    remark            TEXT,
    editable_by       TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_problems_status ON problems(status);
CREATE INDEX IF NOT EXISTS idx_problems_author_id ON problems(author_id);
CREATE INDEX IF NOT EXISTS idx_problems_contest_id ON problems(contest_id);
CREATE INDEX IF NOT EXISTS idx_problems_difficulty ON problems(difficulty);
CREATE INDEX IF NOT EXISTS idx_problems_created_at ON problems(created_at);

-- ===== 帖子（统一模型） =====
CREATE TABLE IF NOT EXISTS posts (
    id                 TEXT PRIMARY KEY,
    title              TEXT NOT NULL,
    content            TEXT NOT NULL,
    author_id          TEXT NOT NULL REFERENCES users(id),
    author_name        TEXT NOT NULL,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    tags               TEXT NOT NULL DEFAULT '[]',
    pinned             INTEGER NOT NULL DEFAULT 0,
    team_only          INTEGER NOT NULL DEFAULT 0,
    emoji              TEXT,
    reactions          TEXT NOT NULL DEFAULT '{}',
    replies            TEXT NOT NULL DEFAULT '[]',
    mentioned_user_ids TEXT NOT NULL DEFAULT '[]',
    status             TEXT NOT NULL DEFAULT '',
    visible_to         TEXT NOT NULL DEFAULT '[]',
    editable_by        TEXT NOT NULL DEFAULT '[]'
);
CREATE INDEX IF NOT EXISTS idx_posts_author_id ON posts(author_id);
CREATE INDEX IF NOT EXISTS idx_posts_created_at ON posts(created_at);
CREATE INDEX IF NOT EXISTS idx_posts_pinned ON posts(pinned);

-- ===== 通知 =====
CREATE TABLE IF NOT EXISTS notifications (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    body        TEXT NOT NULL,
    read        INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    link        TEXT
);
CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_unread ON notifications(user_id, read);

-- ===== 审计日志 =====
CREATE TABLE IF NOT EXISTS audit_log (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    user_id   TEXT NOT NULL,
    user_name TEXT NOT NULL,
    action    TEXT NOT NULL,
    resource  TEXT NOT NULL,
    result    TEXT NOT NULL,
    reason    TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp DESC);

-- ===== 初始元数据 =====
INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', '1');
INSERT OR IGNORE INTO meta (key, value) VALUES ('created_at', '2025-01-01T00:00:00Z');
