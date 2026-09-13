-- 插件安装来源 URL：从 URL 安装 / 更新时记录，供管理后台「从原 URL 更新」使用。
-- 通过本地上传 .zip 安装的插件该列为 NULL。
ALTER TABLE plugins ADD COLUMN source_url TEXT;
