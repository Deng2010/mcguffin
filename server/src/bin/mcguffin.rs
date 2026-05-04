use std::fs;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use chrono::Local;
use toml_edit::{DocumentMut, Item, Value as TomlValue};

const CONFIG_PATH: &str = "/usr/share/mcguffin/config.toml";
const SERVICE_NAME: &str = "mcguffin";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    match args[1].as_str() {
        "init" => cmd_init(),
        "config" => cmd_config(&args),
        "backup" => cmd_backup(&args),
        "start" | "stop" | "restart" => cmd_service(&args[1]),
        "status" => cmd_status(),
        "help" | "--help" | "-h" => print_help(),
        _ => {
            eprintln!("未知命令: {}", args[1]);
            println!();
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("McGuffin CLI v{}", env!("CARGO_PKG_VERSION"));
    println!("管理工具 - 配置、备份与服务控制");
    println!();
    println!("用法:");
    println!("  mcguffin init                    生成默认配置文件");
    println!("  mcguffin config show             查看当前配置");
    println!("  mcguffin config set <key> <val>  修改配置项");
    println!("  mcguffin backup create           创建数据备份");
    println!("  mcguffin backup list             列出所有备份");
    println!("  mcguffin backup restore <name>   恢复指定备份");
    println!("  mcguffin backup delete <name>    删除指定备份");
    println!("  mcguffin start                   启动服务");
    println!("  mcguffin stop                    停止服务");
    println!("  mcguffin restart                 重启服务");
    println!("  mcguffin status                  查看服务状态");
    println!();
    println!("配置项键格式: <section>.<field>");
    println!("  示例:");
    println!("    mcguffin config set server.site_url https://example.com");
    println!("    mcguffin config set admin.password mynewpass");
    println!("    mcguffin config set server.port 8080");
    println!("    mcguffin config set oauth.cp_client_id your_client_id");
    println!();
    println!("配置文件路径: {}", CONFIG_PATH);
}

// ============== Config Helpers ==============

/// Read data_file path from config.toml
fn get_data_file() -> String {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件: {}", e);
        std::process::exit(1);
    });
    let doc = DocumentMut::from_str(&content).unwrap_or_else(|e| {
        eprintln!("错误: 配置文件格式无效: {}", e);
        std::process::exit(1);
    });
    doc.get("server")
        .and_then(|s| s.get("data_file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "mcguffin_data.json".to_string())
}

fn backup_dir() -> std::path::PathBuf {
    let data_file = get_data_file();
    let path = std::path::PathBuf::from(&data_file);
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent.join("backups")
}

// ============== Init ==============

fn cmd_init() {
    if Path::new(CONFIG_PATH).exists() {
        eprintln!("错误: 配置文件已存在 -> {}", CONFIG_PATH);
        eprintln!("如需重新生成请先删除该文件");
        std::process::exit(1);
    }

    if let Some(parent) = Path::new(CONFIG_PATH).parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("错误: 无法创建目录 {:?}: {}", parent, e);
            std::process::exit(1);
        });
    }

    let default_config = r#"# =====================
# McGuffin 配置文件
# =====================

[server]
# 公开访问地址（用于 OAuth 回调、CORS、前端重定向）
site_url = "https://lba-oi.team"
# 监听端口
port = 3000
# 数据文件路径（相对于服务的工作目录）
data_file = "mcguffin_data.json"

[admin]
# 管理员登录密码
password = "admin123"
# 管理员显示名称
display_name = "管理员"

[site]
# 站点名称
name = "McGuffin"

[oauth]
# CP OAuth 客户端凭证
cp_client_id = ""
cp_client_secret = ""
"#;

    fs::write(CONFIG_PATH, default_config).unwrap_or_else(|e| {
        eprintln!("错误: 无法写入配置文件: {}", e);
        std::process::exit(1);
    });

    println!("✓ 配置文件已生成: {}", CONFIG_PATH);
    println!("  请设置以下字段后启动服务:");
    println!("    oauth.cp_client_id");
    println!("    oauth.cp_client_secret");
}

// ============== Config ==============

fn cmd_config(args: &[String]) {
    if args.len() < 3 {
        eprintln!("用法: mcguffin config <show|set>");
        std::process::exit(1);
    }

    match args[2].as_str() {
        "show" => cmd_config_show(),
        "set" => {
            if args.len() < 5 {
                eprintln!("用法: mcguffin config set <key> <value>");
                std::process::exit(1);
            }
            cmd_config_set(args[3].as_str(), args[4].as_str());
        }
        _ => {
            eprintln!("未知子命令: {}. 可用: show, set", args[2]);
            std::process::exit(1);
        }
    }
}

fn cmd_config_show() {
    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件: {}", e);
        eprintln!("  请先运行 'mcguffin init' 生成配置文件");
        std::process::exit(1);
    });
    println!("{}", content.trim());
}

fn cmd_config_set(key: &str, value: &str) {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        eprintln!("错误: 键格式应为 <section>.<field>");
        eprintln!("  收到: {}", key);
        std::process::exit(1);
    }
    let section = parts[0];
    let field = parts[1];

    let content = fs::read_to_string(CONFIG_PATH).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件: {}", e);
        eprintln!("  请先运行 'mcguffin init' 生成配置文件");
        std::process::exit(1);
    });

    let mut doc: DocumentMut = DocumentMut::from_str(&content).unwrap_or_else(|e| {
        eprintln!("错误: 配置文件格式无效: {}", e);
        std::process::exit(1);
    });

    let new_val: TomlValue = if let Ok(n) = value.parse::<i64>() {
        n.into()
    } else if let Ok(f) = value.parse::<f64>() {
        f.into()
    } else if value.eq_ignore_ascii_case("true") {
        true.into()
    } else if value.eq_ignore_ascii_case("false") {
        false.into()
    } else {
        value.into()
    };

    let root = doc.as_table_mut();

    if !root.contains_key(section) {
        eprintln!("错误: 配置段 [{}] 不存在", section);
        eprintln!("  可用配置段: server, admin, site, oauth");
        std::process::exit(1);
    }

    let table = root
        .get_mut(section)
        .and_then(|item| item.as_table_mut())
        .unwrap_or_else(|| {
            eprintln!("错误: 配置段 [{}] 不是有效的表", section);
            std::process::exit(1);
        });
    if table.get(field).is_some() {
        let old_repr = table[field].to_string();
        table[field] = Item::Value(new_val.clone());
        fs::write(CONFIG_PATH, doc.to_string()).unwrap_or_else(|e| {
            eprintln!("错误: 无法写入配置文件: {}", e);
            std::process::exit(1);
        });
        println!("✓ {} = {}  (原值: {})", key, new_val, old_repr.trim());
    } else {
        eprintln!("错误: 配置项 {} 不存在于 [{}] 段", field, section);
        eprintln!("  可用字段:");
        if let Some(t) = root.get(section).and_then(|i| i.as_table()) {
            for (k, _) in t.iter() {
                eprintln!("    {}.{}", section, k);
            }
        }
        std::process::exit(1);
    }
}

// ============== Backup ==============

fn cmd_backup(args: &[String]) {
    if args.len() < 3 {
        eprintln!("用法: mcguffin backup <create|list|restore|delete>");
        println!();
        println!("子命令:");
        println!("  create              创建数据备份");
        println!("  list                列出所有备份");
        println!("  restore <name>      恢复指定备份");
        println!("  delete <name>       删除指定备份");
        std::process::exit(1);
    }

    match args[2].as_str() {
        "create" => cmd_backup_create(),
        "list" => cmd_backup_list(),
        "restore" => {
            if args.len() < 4 {
                eprintln!("用法: mcguffin backup restore <name>");
                std::process::exit(1);
            }
            cmd_backup_restore(&args[3]);
        }
        "delete" => {
            if args.len() < 4 {
                eprintln!("用法: mcguffin backup delete <name>");
                std::process::exit(1);
            }
            cmd_backup_delete(&args[3]);
        }
        _ => {
            eprintln!("未知子命令: {}. 可用: create, list, restore, delete", args[2]);
            std::process::exit(1);
        }
    }
}

fn cmd_backup_create() {
    let data_file = get_data_file();
    let data_path = std::path::PathBuf::from(&data_file);
    if !data_path.exists() {
        eprintln!("错误: 数据文件不存在: {}", data_file);
        std::process::exit(1);
    }

    let dir = backup_dir();
    fs::create_dir_all(&dir).unwrap_or_else(|e| {
        eprintln!("错误: 无法创建备份目录: {}", e);
        std::process::exit(1);
    });

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("mcguffin_data_{}.json", timestamp);
    let backup_path = dir.join(&backup_name);

    fs::copy(&data_path, &backup_path).unwrap_or_else(|e| {
        eprintln!("错误: 备份失败: {}", e);
        std::process::exit(1);
    });

    let size = fs::metadata(&backup_path).map(|m| m.len()).unwrap_or(0);
    let size_str = if size > 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else if size > 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    };
    println!("✓ 备份已创建: {}", backup_name);
    println!("  位置: {:?}", backup_path);
    println!("  大小: {}", size_str);
}

fn cmd_backup_list() {
    let dir = backup_dir();
    if !dir.exists() {
        println!("暂无备份 (备份目录不存在)");
        return;
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            eprintln!("错误: 无法读取备份目录: {}", e);
            std::process::exit(1);
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();

    if entries.is_empty() {
        println!("暂无备份");
        return;
    }

    // Sort by creation time (newest first)
    entries.sort_by(|a, b| {
        let am = a.metadata().ok().and_then(|m| m.created().ok());
        let bm = b.metadata().ok().and_then(|m| m.created().ok());
        bm.cmp(&am)
    });

    println!("备份列表 ({})", entries.len());
    println!("{:-^72}", "");
    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = entry.metadata().ok();
        let size = meta.clone().map(|m| m.len()).unwrap_or(0);
        let size_str = if size > 1024 * 1024 {
            format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
        } else if size > 1024 {
            format!("{:.1}KB", size as f64 / 1024.0)
        } else {
            format!("{}B", size)
        };
        let modified = meta
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {:40} {:>8}  {}", name, size_str, modified);
    }
}

fn cmd_backup_restore(name: &str) {
    // Validate filename
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        eprintln!("错误: 无效的备份文件名");
        std::process::exit(1);
    }
    if !name.ends_with(".json") {
        eprintln!("错误: 无效的备份文件格式");
        std::process::exit(1);
    }

    let dir = backup_dir();
    let backup_path = dir.join(name);
    if !backup_path.exists() {
        eprintln!("错误: 备份文件不存在: {}", name);
        eprintln!("  请先运行 'mcguffin backup list' 查看可用备份");
        std::process::exit(1);
    }

    let data_file = get_data_file();
    let data_path = std::path::PathBuf::from(&data_file);
    let data_path_str = data_file.clone();

    // Create safety backup of current data
    let safety_name = format!("pre_restore_{}.json", Local::now().format("%Y%m%d_%H%M%S"));
    let safety_path = dir.join(&safety_name);
    if data_path.exists() {
        fs::copy(&data_path, &safety_path).unwrap_or_else(|e| {
            eprintln!("错误: 无法创建安全备份: {}", e);
            std::process::exit(1);
        });
        println!("  → 已创建安全备份: {}", safety_name);
    }

    // Restore
    fs::copy(&backup_path, &data_path).unwrap_or_else(|e| {
        eprintln!("错误: 恢复失败: {}", e);
        std::process::exit(1);
    });

    println!("✓ 数据已从备份恢复: {}", name);
    println!("  数据文件: {}", data_path_str);
    println!("  → 请重启服务使更改生效: mcguffin restart");
}

fn cmd_backup_delete(name: &str) {
    // Validate filename
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        eprintln!("错误: 无效的备份文件名");
        std::process::exit(1);
    }

    let dir = backup_dir();
    let backup_path = dir.join(name);
    if !backup_path.exists() {
        eprintln!("错误: 备份文件不存在: {}", name);
        eprintln!("  请先运行 'mcguffin backup list' 查看可用备份");
        std::process::exit(1);
    }

    let size = fs::metadata(&backup_path).map(|m| m.len()).unwrap_or(0);
    fs::remove_file(&backup_path).unwrap_or_else(|e| {
        eprintln!("错误: 删除失败: {}", e);
        std::process::exit(1);
    });

    let size_str = if size > 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else if size > 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    };
    println!("✓ 备份已删除: {} ({})", name, size_str);
}

// ============== Service ==============

fn cmd_service(action: &str) {
    let result = Command::new("systemctl")
        .arg(action)
        .arg(format!("{}.service", SERVICE_NAME))
        .status();

    match result {
        Ok(status) if status.success() => {
            let label = match action {
                "start" => "启动",
                "stop" => "停止",
                "restart" => "重启",
                _ => action,
            };
            println!("✓ 服务已{}: {}.service", label, SERVICE_NAME);
        }
        Ok(status) => {
            eprintln!("错误: 服务操作失败 (exit: {:?})", status.code());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("错误: 无法执行 systemctl: {}", e);
            eprintln!("  请确保 systemd 可用且具有管理员权限");
            std::process::exit(1);
        }
    }
}

fn cmd_status() {
    let output = Command::new("systemctl")
        .arg("status")
        .arg(format!("{}.service", SERVICE_NAME))
        .arg("--no-pager")
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("●")
                    || trimmed.starts_with("Active:")
                    || trimmed.starts_with("Main PID:")
                {
                    println!("{}", trimmed);
                }
            }
            if !o.status.success() {
                let stderr = String::from_utf8_lossy(&o.stderr);
                if !stderr.trim().is_empty() {
                    eprintln!("{}", stderr.trim());
                }
            }
        }
        Err(e) => {
            eprintln!("错误: 无法执行 systemctl: {}", e);
            std::process::exit(1);
        }
    }
}
