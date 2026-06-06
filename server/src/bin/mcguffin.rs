use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;

use chrono::Local;
use clap::{Parser, Subcommand};
use toml_edit::{DocumentMut, Item, Value as TomlValue};

// ===================== CLI Definition =====================

#[derive(Parser)]
#[command(
    name = "mcguffin",
    version,
    about = "McGuffin 管理工具 - 配置、备份与服务控制"
)]
struct Cli {
    /// 配置文件路径（默认自动探测：平台路径 > CWD 的 config.toml / mcguffin.toml）
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 生成默认配置文件
    Init,
    /// 查看或修改配置
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// 数据备份管理
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 重启服务
    Restart,
    /// 查看服务状态
    Status,
    /// 转换 JSON 数据文件为 SQLite 数据库
    JsonToDb {
        /// JSON 数据文件路径
        input: String,
        /// 输出 SQLite 数据库文件路径（可选，默认同目录）
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// 查看当前配置
    Show,
    /// 修改配置项，格式: <section>.<field> <value>
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum BackupAction {
    /// 创建数据备份
    Create,
    /// 列出所有备份
    List,
    /// 恢复指定备份
    Restore { name: String },
    /// 删除指定备份
    Delete { name: String },
}

// ===================== Config Path Resolution =====================

fn default_config_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let system = PathBuf::from("/usr/share/mcguffin/config.toml");
        if system.exists() {
            return system;
        }
        if let Some(home) = std::env::var_os("HOME") {
            let user = PathBuf::from(home).join(".config/mcguffin/config.toml");
            if user.exists() {
                return user;
            }
        }
        system
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join("Library/Application Support/mcguffin/config.toml")
        } else {
            PathBuf::from("/usr/share/mcguffin/config.toml")
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata).join("mcguffin/config.toml")
        } else {
            PathBuf::from("C:/ProgramData/mcguffin/config.toml")
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/usr/share/mcguffin/config.toml")
    }
}

fn resolve_config_path(cli_config: Option<&str>) -> PathBuf {
    if let Some(path) = cli_config {
        return PathBuf::from(path);
    }
    for name in &["mcguffin.toml", "config.toml"] {
        let cwd_path = PathBuf::from(name);
        if cwd_path.exists() {
            return cwd_path;
        }
    }
    default_config_path()
}

fn runtime_dir(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn find_server_binary() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate = parent.join("mcguffin-server");
            if candidate.exists() {
                return candidate;
            }
            #[cfg(target_os = "windows")]
            {
                let candidate = parent.join("mcguffin-server.exe");
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }

    let candidate = PathBuf::from("mcguffin-server");
    if candidate.exists() {
        return candidate;
    }
    #[cfg(target_os = "windows")]
    {
        let candidate = PathBuf::from("mcguffin-server.exe");
        if candidate.exists() {
            return candidate;
        }
    }

    for prefix in &["/usr/local", "/opt/homebrew", "/usr"] {
        let candidate = PathBuf::from(format!("{}/lib/mcguffin/mcguffin-server", prefix));
        if candidate.exists() {
            return candidate;
        }
    }

    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = PathBuf::from(dir).join("mcguffin-server");
            if candidate.exists() {
                return candidate;
            }
        }
    }

    PathBuf::from("mcguffin-server")
}

/// 推断 mcguffin-server 的正确运行目录。
/// server 的 main.rs 硬编码 `../web/dist` 作为前端静态资源路径，
/// 因此运行目录必须是 `server/`（开发）或与 web/dist/ 同级（生产）。
fn server_work_dir(server_path: &Path) -> PathBuf {
    // 开发场景：二进制在 target/release/ 或 target/debug/ 下
    if let Some(parent) = server_path.parent() {
        if parent.ends_with("target/release") || parent.ends_with("target/debug") {
            if let Some(project_root) = parent.parent().and_then(|p| p.parent()) {
                // project_root 是 server/ 目录，server 从这儿运行可以找到 ../web/dist
                if project_root.join("../web/dist").exists()
                    || project_root.join("Cargo.toml").exists()
                {
                    return project_root.to_path_buf();
                }
            }
        }
    }
    // 生产场景：二进制所在目录即运行目录（如 /usr/local/lib/mcguffin/）
    server_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn server_pid_path(config_path: &Path) -> PathBuf {
    runtime_dir(config_path).join("mcguffin.pid")
}

fn read_pid(pid_path: &Path) -> Option<u32> {
    let content = fs::read_to_string(pid_path).ok()?;
    content.trim().parse::<u32>().ok()
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let result = Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        matches!(result, Ok(status) if status.success())
    }
    #[cfg(windows)]
    {
        let result = Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
}

fn kill_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        for _ in 0..30 {
            if !is_process_running(pid) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let result = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        matches!(result, Ok(status) if status.success())
    }
    #[cfg(windows)]
    {
        let result = Command::new("taskkill")
            .arg("/PID")
            .arg(pid.to_string())
            .arg("/F")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        matches!(result, Ok(status) if status.success())
    }
}

// ===================== Config Helpers =====================

fn read_config(config_path: &Path) -> DocumentMut {
    let content = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件 ({}): {}", config_path.display(), e);
        eprintln!("  请先运行 'mcguffin init' 生成配置文件");
        std::process::exit(1);
    });
    DocumentMut::from_str(&content).unwrap_or_else(|e| {
        eprintln!("错误: 配置文件格式无效: {}", e);
        std::process::exit(1);
    })
}

fn get_data_file(config_path: &Path) -> String {
    let doc = read_config(config_path);
    doc.get("server")
        .and_then(|s| s.get("data_file"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "mcguffin_data.json".to_string())
}

fn backup_dir(config_path: &Path) -> PathBuf {
    let data_file = get_data_file(config_path);
    let path = PathBuf::from(&data_file);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join("backups")
}

fn format_size(size: u64) -> String {
    if size > 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else if size > 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} B", size)
    }
}

// ===================== Interactive Input Helpers =====================

/// 读取一行用户输入，去除首尾空白
fn read_line() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap_or(0);
    input.trim().to_string()
}

/// 提示用户输入，返回输入值（可能为空）
fn prompt(msg: &str) -> String {
    print!("{}", msg);
    io::stdout().flush().unwrap();
    read_line()
}

/// 提示用户输入，若为空则返回默认值
fn prompt_default(msg: &str, default: &str) -> String {
    let val = prompt(&format!("{} [{}]: ", msg, default));
    if val.is_empty() {
        default.to_string()
    } else {
        val
    }
}

/// 提示用户输入，不允许为空
fn prompt_required(msg: &str, hint: &str) -> String {
    loop {
        let val = prompt(&format!("{}: ", msg));
        if !val.is_empty() {
            return val;
        }
        if !hint.is_empty() {
            eprintln!("  {} 不能为空，{}", msg, hint);
        }
    }
}

/// 提示 yes/no，默认否
fn prompt_yesno(msg: &str, default: bool) -> bool {
    let default_str = if default { "Y/n" } else { "y/N" };
    let val = prompt(&format!("{} [{}]: ", msg, default_str));
    match val.to_lowercase().as_str() {
        "y" | "yes" | "是" => true,
        "n" | "no" | "否" => false,
        _ => default,
    }
}

/// TOML 字符串值转义
fn toml_string(s: &str) -> String {
    if s.contains('"') || s.contains('\n') {
        // 使用多行字符串
        format!("\"\"\"\n{}\n\"\"\"", s)
    } else {
        format!("\"{}\"", s)
    }
}

// ===================== Commands: Init =====================

fn cmd_init(config_path: &Path) {
    if config_path.exists() {
        eprintln!("错误: 配置文件已存在 -> {}", config_path.display());
        eprintln!("如需重新生成请先删除该文件");
        std::process::exit(1);
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("错误: 无法创建目录 {:?}: {}", parent, e);
            std::process::exit(1);
        });
    }

    println!("========================================");
    println!("  McGuffin 配置向导");
    println!("========================================");
    println!();
    println!("将生成配置文件: {}", config_path.display());
    println!("直接回车使用方括号内的默认值。");
    println!();

    // ===== [server] section =====
    println!("--- 服务器配置 [server] ---");

    let site_url = prompt_required(
        "  公开访问地址（含端口，如 https://example.com:3000）",
        "例如 https://mcguffin.example.com:3000",
    );

    let port = prompt_default("  监听端口（若与URL中相同可回车跳过）", "3000");

    let data_file = prompt_default("  数据文件路径（相对服务工作目录）", "mcguffin_data.json");

    println!();

    // ===== [admin] section =====
    println!("--- 管理员配置 [admin] ---");

    let admin_password = prompt_required("  管理员登录密码", "请设置一个安全的密码");

    let admin_display_name = prompt_default("  管理员显示名称", "管理员");

    println!();

    // ===== [site] section =====
    println!("--- 站点配置 [site]（可选，直接回车跳过）---");

    let site_name = prompt_default("  站点名称", "McGuffin");

    let site_title = prompt_default("  浏览器标签标题（留空使用站点名称）", "");

    println!();

    // ===== [oauth] section =====
    println!("--- CP OAuth 配置 [oauth] ---");
    println!("  需要先在 https://www.cpoauth.com 注册应用获取凭证。");
    println!("  若暂时没有可留空，服务将以演示模式运行。");
    println!();

    let oauth_client_id = prompt("  Client ID (回车跳过): ");
    let oauth_client_secret = prompt("  Client Secret (回车跳过): ");

    println!();

    // ===== [difficulty] section =====
    println!("--- 题目难度等级 [difficulty]（可选）---");

    let mut difficulties: Vec<(String, String, String)> = Vec::new();

    if prompt_yesno("  是否自定义难度等级？", false) {
        println!("  每个难度需要: 标识符(如 Easy)、显示名称(如 简单)、颜色(如 #22c55e)");
        println!("  输入空标识符结束。");
        loop {
            let id = prompt(&format!(
                "  难度 {} 标识符 (回车结束): ",
                difficulties.len() + 1
            ));
            if id.is_empty() {
                break;
            }
            let label = prompt_default(&format!("  难度 {} 显示名称", difficulties.len() + 1), &id);
            let color = prompt_default(
                &format!("  难度 {} 颜色 (hex)", difficulties.len() + 1),
                "#888888",
            );
            difficulties.push((id, label, color));
        }
        if difficulties.is_empty() {
            println!("  （未添加自定义难度，将使用默认: Easy/Medium/Hard）");
        }
        println!();
    }

    // ===== [discussion_tags] section =====
    println!("--- 讨论标签 [discussion_tags]（可选）---");

    let mut tags: Vec<(String, String, String)> = Vec::new();
    if prompt_yesno("  是否配置讨论标签？", false) {
        println!("  每个标签需要: 标识符、颜色、描述");
        println!("  输入空标识符结束。");
        loop {
            let id = prompt(&format!("  标签 {} 标识符 (回车结束): ", tags.len() + 1));
            if id.is_empty() {
                break;
            }
            let color = prompt_default(&format!("  标签 {} 颜色", tags.len() + 1), "#3b82f6");
            let desc = prompt(&format!("  标签 {} 描述: ", tags.len() + 1));
            tags.push((id, color, desc));
        }
        if tags.is_empty() {
            println!("  （未添加讨论标签）");
        }
        println!();
    }

    // ===== [discussion_emojis] section =====
    println!("--- 讨论表情 [discussion_emojis]（可选）---");

    let mut emojis: Vec<(String, String)> = Vec::new();
    if prompt_yesno("  是否配置讨论表情？", false) {
        println!("  每个表情需要: 标识符 和 实际字符 (如 thumbsup, 👍)");
        println!("  输入空标识符结束。");
        loop {
            let id = prompt(&format!("  表情 {} 标识符 (回车结束): ", emojis.len() + 1));
            if id.is_empty() {
                break;
            }
            let ch = prompt_required(&format!("  表情 {} 字符 (如 👍)", emojis.len() + 1), "");
            emojis.push((id, ch));
        }
        if emojis.is_empty() {
            println!("  （未添加讨论表情）");
        }
        println!();
    }

    // ===== Permission groups =====
    println!("--- 权限组 [permissions.groups]（可选）---");

    let mut groups: Vec<(String, String, Vec<String>)> = Vec::new();
    if prompt_yesno("  是否配置权限组？", false) {
        println!("  每个权限组需要: 名称 和 权限列表 (逗号分隔)");
        println!("  可用权限: view_showcase, apply_join, view_team, manage_team,");
        println!("            manage_members, submit_problem, view_problems, approve_problem,");
        println!("            manage_contests, manage_site, edit_showcase, view_discussions,");
        println!("            manage_discussions, manage_tags, manage_notifications,");
        println!("            manage_backups, view_stats, manage_posts");
        println!("  输入空名称结束。");
        loop {
            let gname = prompt(&format!("  权限组 {} 名称 (回车结束): ", groups.len() + 1));
            if gname.is_empty() {
                break;
            }
            let perms_str = prompt("    权限列表 (逗号分隔): ");
            let perms: Vec<String> = perms_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if perms.is_empty() {
                println!("    （未添加权限，组将跳过）");
                continue;
            }
            // 生成一个简短的 UUID-like ID
            let id = format!("grp_{:x}", groups.len() + 1);
            groups.push((id, gname, perms));
        }
        if groups.is_empty() {
            println!("  （未配置权限组）");
        }
        println!();
    }

    // ===== Preview & confirm =====
    println!("========================================");
    println!("  配置预览");
    println!("========================================");
    println!();

    let config_content = build_config_toml(
        &site_url,
        &port,
        &data_file,
        &admin_password,
        &admin_display_name,
        &site_name,
        &site_title,
        &oauth_client_id,
        &oauth_client_secret,
        &difficulties,
        &tags,
        &emojis,
        &groups,
    );

    println!("{}", config_content);

    if !prompt_yesno("确认写入以上配置？", true) {
        println!("已取消，未写入任何文件。");
        return;
    }

    fs::write(config_path, &config_content).unwrap_or_else(|e| {
        eprintln!("错误: 无法写入配置文件 ({}): {}", config_path.display(), e);
        std::process::exit(1);
    });

    println!("✓ 配置文件已生成: {}", config_path.display());
    println!();
    println!("后续步骤:");
    println!("  1. 启动服务:  mcguffin start");
    println!("  2. 查看状态:  mcguffin status");
    println!("  3. 修改配置:  mcguffin config set <key> <value>");
}

// ===================== Config Builder =====================

#[allow(clippy::too_many_arguments)]
fn build_config_toml(
    site_url: &str,
    port: &str,
    data_file: &str,
    admin_password: &str,
    admin_display_name: &str,
    site_name: &str,
    site_title: &str,
    oauth_client_id: &str,
    oauth_client_secret: &str,
    difficulties: &[(String, String, String)],
    tags: &[(String, String, String)],
    emojis: &[(String, String)],
    groups: &[(String, String, Vec<String>)],
) -> String {
    let mut toml = String::new();

    toml.push_str("# =====================\n");
    toml.push_str("# McGuffin 配置文件\n");
    toml.push_str("# 由 mcguffin init 交互式生成\n");
    toml.push_str(&format!(
        "# 生成时间: {}\n",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    toml.push_str("# =====================\n\n");

    // [server]
    toml.push_str("[server]\n");
    toml.push_str("# 公开访问地址（用于 OAuth 回调、CORS、前端重定向）\n");
    toml.push_str(&format!("site_url = {}\n", toml_string(site_url)));
    toml.push_str("# 监听端口\n");
    toml.push_str(&format!("port = {}\n", port));
    toml.push_str("# 数据文件路径（相对于服务的工作目录）\n");
    toml.push_str(&format!("data_file = {}\n\n", toml_string(data_file)));

    // [admin]
    toml.push_str("[admin]\n");
    toml.push_str("# 管理员登录密码\n");
    toml.push_str(&format!("password = {}\n", toml_string(admin_password)));
    toml.push_str("# 管理员显示名称\n");
    toml.push_str(&format!(
        "display_name = {}\n\n",
        toml_string(admin_display_name)
    ));

    // [site]
    toml.push_str("[site]\n");
    toml.push_str("# 站点导航栏显示名称\n");
    toml.push_str(&format!("name = {}\n", toml_string(site_name)));
    if !site_title.is_empty() {
        toml.push_str("# 浏览器标签标题\n");
        toml.push_str(&format!("title = {}\n", toml_string(site_title)));
    }
    toml.push('\n');

    // [oauth]
    toml.push_str("[oauth]\n");
    toml.push_str("# CP OAuth 客户端凭证\n");
    toml.push_str(&format!(
        "cp_client_id = {}\n",
        toml_string(oauth_client_id)
    ));
    toml.push_str(&format!(
        "cp_client_secret = {}\n\n",
        toml_string(oauth_client_secret)
    ));

    // [difficulty]
    if !difficulties.is_empty() {
        toml.push_str("# 题目难度等级\n");
        for (id, label, color) in difficulties {
            toml.push_str(&format!("\n[difficulty.{}]\n", id));
            toml.push_str(&format!("label = {}\n", toml_string(label)));
            toml.push_str(&format!("color = {}\n", toml_string(color)));
        }
        toml.push('\n');
    }

    // [discussion_tags] — 默认标签
    if !tags.is_empty() {
        toml.push_str("# 讨论标签（用户自定义）\n");
        for (id, color, desc) in tags {
            toml.push_str(&format!("\n[discussion_tags.{}]\n", id));
            toml.push_str(&format!("color = {}\n", toml_string(color)));
            if !desc.is_empty() {
                toml.push_str(&format!("description = {}\n", toml_string(desc)));
            }
        }
    } else {
        // 默认标签
        toml.push_str("# 默认讨论标签\n");
        toml.push_str("\n[discussion_tags.公告]\n");
        toml.push_str(&format!("color = {}\n", toml_string("#ef4444")));
        toml.push_str(&format!("description = {}\n", toml_string("官方公告")));
        toml.push_str("\n[discussion_tags.建议]\n");
        toml.push_str(&format!("color = {}\n", toml_string("#3b82f6")));
        toml.push_str(&format!("description = {}\n", toml_string("功能建议")));
        toml.push_str("\n[discussion_tags.标签]\n");
        toml.push_str(&format!("color = {}\n", toml_string("#22c55e")));
        toml.push_str(&format!("description = {}\n", toml_string("一般讨论")));
    }
    toml.push('\n');

    // [discussion_emojis]
    if !emojis.is_empty() {
        toml.push_str("# 讨论表情\n");
        for (id, ch) in emojis {
            toml.push_str(&format!("\n[discussion_emojis.{}]\n", id));
            toml.push_str(&format!("char = {}\n", toml_string(ch)));
        }
        toml.push('\n');
    }

    // [permissions.roles] — 默认角色权限（注释掉 = 使用代码内置默认值）
    toml.push_str("# =====================\n");
    toml.push_str("# 权限配置 [permissions]\n");
    toml.push_str("# 取消注释即可覆盖默认角色权限\n");
    toml.push_str("# =====================\n");
    toml.push_str("# [permissions]\n");
    toml.push_str("# admin = [\"view_team\", \"manage_team\", \"manage_members\", \"submit_problem\", \"view_problems\", \"approve_problem\", \"manage_contests\", \"view_all_contests\", \"view_public_contests\", \"manage_site\", \"edit_showcase\", \"view_discussions\", \"manage_posts\", \"manage_tags\", \"manage_notifications\", \"view_stats\"]\n");
    toml.push_str("# member = [\"view_showcase\", \"view_team\", \"submit_problem\", \"view_problems\", \"view_all_contests\", \"view_public_contests\", \"view_discussions\"]\n");
    toml.push_str("# guest = [\"view_showcase\", \"apply_join\", \"view_public_contests\", \"view_discussions\"]\n");
    toml.push('\n');

    // [permissions.groups]
    if !groups.is_empty() {
        toml.push_str("# 权限组（通过 UUID 标识）\n");
        for (id, gname, perms) in groups {
            toml.push_str(&format!("[permissions.groups.\"{}\"]\n", id));
            toml.push_str(&format!("name = {}\n", toml_string(gname)));
            let perms_str: Vec<String> = perms.iter().map(|p| toml_string(p)).collect();
            toml.push_str(&format!("permissions = [{}]\n", perms_str.join(", ")));
            toml.push('\n');
        }
    }

    toml
}

// ===================== Commands: Config =====================

fn cmd_config_show(config_path: &Path) {
    let content = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件 ({}): {}", config_path.display(), e);
        eprintln!("  请先运行 'mcguffin init' 生成配置文件");
        std::process::exit(1);
    });
    println!("{}", content.trim());
}

fn cmd_config_set(config_path: &Path, key: &str, value: &str) {
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    if parts.len() != 2 {
        eprintln!("错误: 键格式应为 <section>.<field>，收到: {}", key);
        std::process::exit(1);
    }
    let section = parts[0];
    let field = parts[1];

    let content = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("错误: 无法读取配置文件 ({}): {}", config_path.display(), e);
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
        eprintln!(
            "错误: 配置段 [{}] 不存在，可用段: server, admin, site, oauth",
            section
        );
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
        fs::write(config_path, doc.to_string()).unwrap_or_else(|e| {
            eprintln!("错误: 无法写入配置文件 ({}): {}", config_path.display(), e);
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

// ===================== Commands: Backup =====================

fn cmd_backup_create(config_path: &Path) {
    let data_file = get_data_file(config_path);
    let data_path = PathBuf::from(&data_file);
    if !data_path.exists() {
        eprintln!("错误: 数据文件不存在: {}", data_file);
        std::process::exit(1);
    }

    let dir = backup_dir(config_path);
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
    println!("✓ 备份已创建: {}", backup_name);
    println!("  位置: {}", backup_path.display());
    println!("  大小: {}", format_size(size));
}

fn cmd_backup_list(config_path: &Path) {
    let dir = backup_dir(config_path);
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
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        println!("暂无备份");
        return;
    }

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
        let modified = meta
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {:40} {:>8}  {}", name, format_size(size), modified);
    }
}

fn cmd_backup_restore(config_path: &Path, name: &str) {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        eprintln!("错误: 无效的备份文件名");
        std::process::exit(1);
    }
    if !name.ends_with(".json") {
        eprintln!("错误: 无效的备份文件格式");
        std::process::exit(1);
    }

    let dir = backup_dir(config_path);
    let backup_path = dir.join(name);
    if !backup_path.exists() {
        eprintln!("错误: 备份文件不存在: {}", name);
        eprintln!("  请先运行 'mcguffin backup list' 查看可用备份");
        std::process::exit(1);
    }

    let data_file = get_data_file(config_path);
    let data_path = PathBuf::from(&data_file);

    let safety_name = format!("pre_restore_{}.json", Local::now().format("%Y%m%d_%H%M%S"));
    let safety_path = dir.join(&safety_name);
    if data_path.exists() {
        fs::copy(&data_path, &safety_path).unwrap_or_else(|e| {
            eprintln!("错误: 无法创建安全备份: {}", e);
            std::process::exit(1);
        });
        println!("  → 已创建安全备份: {}", safety_name);
    }

    fs::copy(&backup_path, &data_path).unwrap_or_else(|e| {
        eprintln!("错误: 恢复失败: {}", e);
        std::process::exit(1);
    });

    println!("✓ 数据已从备份恢复: {}", name);
    println!("  数据文件: {}", data_file);
    println!("  → 请重启服务使更改生效");
}

fn cmd_backup_delete(config_path: &Path, name: &str) {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        eprintln!("错误: 无效的备份文件名");
        std::process::exit(1);
    }

    let dir = backup_dir(config_path);
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

    println!("✓ 备份已删除: {} ({})", name, format_size(size));
}

// ===================== Commands: Service =====================

fn cmd_service_start(config_path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .arg("start")
            .arg("mcguffin.service")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                println!("✓ 服务已启动: mcguffin.service (systemd)");
                return;
            }
        }
    }

    let server_bin = find_server_binary();
    if !server_bin.exists() {
        eprintln!("错误: 找不到 mcguffin-server 二进制文件");
        eprintln!("  查找位置: {}", server_bin.display());
        eprintln!("  请先运行 'just install' 或 'just install-cli' 安装");
        std::process::exit(1);
    }

    let pid_path = server_pid_path(config_path);

    if let Some(pid) = read_pid(&pid_path) {
        if is_process_running(pid) {
            eprintln!("错误: 服务已在运行中 (PID: {})", pid);
            eprintln!("  如需重启请先运行: mcguffin stop");
            std::process::exit(1);
        } else {
            let _ = fs::remove_file(&pid_path);
        }
    }

    let server_path = std::path::absolute(&server_bin).unwrap_or(server_bin.clone());
    let work_dir = server_work_dir(&server_path);
    let log_path = runtime_dir(config_path).join("mcguffin.log");

    if let Some(log_parent) = log_path.parent() {
        fs::create_dir_all(log_parent).unwrap_or_else(|e| {
            eprintln!("错误: 无法创建日志目录: {}", e);
            std::process::exit(1);
        });
    }

    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .unwrap_or_else(|e| {
            eprintln!("错误: 无法打开日志文件 ({}): {}", log_path.display(), e);
            std::process::exit(1);
        });

    let child: Child = match Command::new(&server_path)
        .current_dir(work_dir)
        .stdout(Stdio::from(log_file.try_clone().unwrap_or_else(|e| {
            eprintln!("错误: 无法复制日志句柄: {}", e);
            std::process::exit(1);
        })))
        .stderr(Stdio::from(log_file))
        .stdin(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("错误: 启动服务失败: {}", e);
            std::process::exit(1);
        }
    };

    let pid = child.id();
    fs::write(&pid_path, pid.to_string()).unwrap_or_else(|e| {
        eprintln!("错误: 无法写入 PID 文件: {}", e);
        std::process::exit(1);
    });

    let doc = read_config(config_path);
    let site_url = doc
        .get("server")
        .and_then(|s| s.get("site_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("http://localhost:3000")
        .to_string();

    println!("✓ 服务已启动");
    println!("  地址:   {}", site_url);
    println!("  PID:    {}", pid);
    println!("  日志:   {}", log_path.display());
}

fn cmd_service_stop(config_path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .arg("stop")
            .arg("mcguffin.service")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                println!("✓ 服务已停止: mcguffin.service (systemd)");
                return;
            }
        }
    }

    let pid_path = server_pid_path(config_path);
    let pid = match read_pid(&pid_path) {
        Some(pid) => pid,
        None => {
            eprintln!("错误: 服务未在运行（找不到 PID 文件）");
            std::process::exit(1);
        }
    };

    if !is_process_running(pid) {
        let _ = fs::remove_file(&pid_path);
        eprintln!("注意: 服务已停止（PID {} 不存在）", pid);
        return;
    }

    println!("正在停止服务 (PID: {})...", pid);
    if kill_process(pid) {
        let _ = fs::remove_file(&pid_path);
        println!("✓ 服务已停止");
    } else {
        eprintln!("错误: 无法停止服务 (PID: {})", pid);
        std::process::exit(1);
    }
}

fn cmd_service_restart(config_path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let status = Command::new("systemctl")
            .arg("restart")
            .arg("mcguffin.service")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                println!("✓ 服务已重启: mcguffin.service (systemd)");
                return;
            }
        }
    }

    cmd_service_stop(config_path);
    cmd_service_start(config_path);
    println!("✓ 服务已重启");
}

fn cmd_service_status(config_path: &Path) {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("systemctl")
            .arg("status")
            .arg("mcguffin.service")
            .arg("--no-pager")
            .output();
        if let Ok(o) = output {
            if o.status.success() {
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
                return;
            }
        }
    }

    let pid_path = server_pid_path(config_path);

    match read_pid(&pid_path) {
        Some(pid) if is_process_running(pid) => {
            println!("● mcguffin 服务正在运行");
            println!("  PID:     {}", pid);
            println!("  PID 文件: {}", pid_path.display());

            #[cfg(unix)]
            {
                let output = Command::new("ps")
                    .arg("-o")
                    .arg("etime=")
                    .arg("-p")
                    .arg(pid.to_string())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .output();
                if let Ok(o) = output {
                    let uptime = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if !uptime.is_empty() {
                        println!("  运行时长: {}", uptime);
                    }
                }
            }
        }
        Some(_) => {
            println!("● mcguffin 服务未运行 (PID 文件已过期)");
            let _ = fs::remove_file(&pid_path);
            std::process::exit(1);
        }
        None => {
            println!("● mcguffin 服务未运行");
            std::process::exit(1);
        }
    }
}

// ===================== JSON → DB 转换 =====================

/// 将旧版 JSON 数据文件转换为 SQLite 数据库
fn cmd_json_to_db(input_path: &str, output_path: Option<&str>) {
    eprintln!("JSON → SQLite 数据转换工具");
    eprintln!("输入: {}", input_path);

    let json_path = std::path::Path::new(input_path);
    if !json_path.exists() {
        eprintln!("错误: 输入文件不存在: {}", input_path);
        std::process::exit(1);
    }

    let output = output_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| json_path.with_extension("db"));
    eprintln!("输出: {}", output.display());

    eprintln!("正在转换...");
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        match mcguffin_server_lib::import_json_to_db(input_path, &output.to_string_lossy()).await {
            Ok(n) => {
                eprintln!("成功! 已导入 {} 条记录到 {}", n, output.display());
            }
            Err(e) => {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        }
    });
}

// ===================== Entry Point =====================

fn main() {
    let cli = Cli::parse();
    let config_path = resolve_config_path(cli.config.as_deref());

    match &cli.command {
        Commands::Init => cmd_init(&config_path),
        Commands::Config { action } => match action {
            ConfigAction::Show => cmd_config_show(&config_path),
            ConfigAction::Set { key, value } => cmd_config_set(&config_path, key, value),
        },
        Commands::Backup { action } => match action {
            BackupAction::Create => cmd_backup_create(&config_path),
            BackupAction::List => cmd_backup_list(&config_path),
            BackupAction::Restore { name } => cmd_backup_restore(&config_path, name),
            BackupAction::Delete { name } => cmd_backup_delete(&config_path, name),
        },
        Commands::Start => cmd_service_start(&config_path),
        Commands::Stop => cmd_service_stop(&config_path),
        Commands::Restart => cmd_service_restart(&config_path),
        Commands::Status => cmd_service_status(&config_path),
        Commands::JsonToDb { input, output } => cmd_json_to_db(input, output.as_deref()),
    }
}
