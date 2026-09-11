-- 插件系统持久化：插件清单 + 插件 KV 数据
-- 全局开关（plugins_disabled）复用 meta 表存储，无需新表

CREATE TABLE IF NOT EXISTS plugins (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    author      TEXT NOT NULL DEFAULT '',
    permissions TEXT NOT NULL DEFAULT '[]',   -- JSON 数组字符串
    enabled     INTEGER NOT NULL DEFAULT 1,
    source      TEXT NOT NULL DEFAULT 'code', -- 'code'（前端代码注册）| 'zip'（上传安装）
    entry       TEXT                          -- zip 插件的入口文件（assets 内相对路径）
);

CREATE TABLE IF NOT EXISTS plugin_data (
    plugin_id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (plugin_id, namespace, key)
);

CREATE INDEX IF NOT EXISTS idx_plugin_data_plugin ON plugin_data(plugin_id);
