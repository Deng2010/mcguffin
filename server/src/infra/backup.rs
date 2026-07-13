use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use chrono;
use tokio::time::interval;
use tracing;

use crate::db::create_consistent_backup;
use crate::state::AppState;

impl AppState {
    /// 启动后台自动备份任务。
    /// 每隔 `interval_secs` 秒创建一个 SQLite 备份，保留最多 `max_backups` 个。
    pub fn start_auto_backup(self: &Arc<Self>, interval_secs: u64, max_backups: usize) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            // 跳过第一次立即执行，等一个间隔
            interval.tick().await;

            loop {
                interval.tick().await;

                // 数据已实时写入 SQLite，直接创建备份即可
                // 1. 创建 SQLite 备份
                let db_path = Path::new(&state.db_path);
                if db_path.exists() {
                    let dir = db_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join("backups");
                    let _ = std::fs::create_dir_all(&dir);

                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let backup_name = format!("mcguffin_auto_{}.db", timestamp);
                    let dest = dir.join(&backup_name);

                    match create_consistent_backup(
                        &db_path.to_string_lossy(),
                        &dest.to_string_lossy(),
                    ) {
                        Ok(()) => tracing::debug!("Auto backup created: {:?}", dest),
                        Err(e) => tracing::warn!("Auto backup failed: {}", e),
                    }

                    // 2. 清理旧备份
                    if let Ok(entries) = std::fs::read_dir(&dir) {
                        let mut db_backups: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.path().extension().map(|ext| ext == "db").unwrap_or(false)
                            })
                            .collect();
                        // 按修改时间排序，最旧的在前面
                        db_backups
                            .sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

                        // 删除超出限制的旧备份
                        while db_backups.len() > max_backups {
                            if let Some(oldest) = db_backups.first() {
                                let path = oldest.path();
                                if std::fs::remove_file(&path).is_ok() {
                                    tracing::info!("Pruned old auto-backup: {:?}", path);
                                }
                            }
                            db_backups.remove(0);
                        }
                    }
                }
            }
        });
    }
}
