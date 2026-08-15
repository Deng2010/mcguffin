-- McGuffin 错误报告与处理系统
-- 所有表使用 IF NOT EXISTS 以支持重复迁移

CREATE TABLE IF NOT EXISTS error_reports (
    id          TEXT PRIMARY KEY,
    ts          TEXT NOT NULL,
    user_id     TEXT,
    source      TEXT NOT NULL DEFAULT 'frontend',
    code        TEXT NOT NULL,
    message     TEXT NOT NULL DEFAULT '',
    hint        TEXT NOT NULL DEFAULT '',
    suggestion  TEXT NOT NULL DEFAULT '',
    stack       TEXT NOT NULL DEFAULT '',
    url         TEXT NOT NULL DEFAULT '',
    route       TEXT NOT NULL DEFAULT '',
    method      TEXT NOT NULL DEFAULT '',
    http_status INTEGER,
    ua          TEXT NOT NULL DEFAULT '',
    plugin_id   TEXT NOT NULL DEFAULT '',
    fingerprint TEXT NOT NULL UNIQUE,
    count       INTEGER NOT NULL DEFAULT 1,
    status      TEXT NOT NULL DEFAULT 'open',
    resolved_by TEXT,
    resolved_at TEXT,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_error_reports_last_seen ON error_reports(last_seen);
CREATE INDEX IF NOT EXISTS idx_error_reports_code ON error_reports(code);
CREATE INDEX IF NOT EXISTS idx_error_reports_status ON error_reports(status);
