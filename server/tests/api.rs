// ============== Integration Tests ==============
//
// These tests verify the core behavior of AppState, types,
// and utility functions in a realistic environment.

use mcguffin_server_lib::*;

use axum::body::Body;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Request, StatusCode};
use axum::Router;
use std::net::SocketAddr;
use tower::ServiceExt;

use mcguffin_server_lib::error;

/// 构建带固定客户端 IP 的测试路由（ConnectInfo 提取器需要）。
fn test_router(state: AppState) -> Router {
    build_router(state).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345))))
}

async fn clean_error_reports(state: &AppState) {
    sqlx::query("DELETE FROM error_reports")
        .execute(&state.db)
        .await
        .expect("clean error_reports");
}

async fn create_session(state: &AppState, user_id: &str) -> String {
    let token = format!(
        "test-session-{}-{}",
        user_id,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    state.sessions.write().await.insert(
        token.clone(),
        SessionEntry {
            user_id: user_id.to_string(),
            last_active: chrono::Utc::now(),
        },
    );
    token
}

fn auth_header(token: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {}", token))
}

/// 未匹配路由 → 统一 404 JSON。
#[tokio::test]
async fn test_api_fallback_returns_json_404() {
    let state = AppState::new().await;
    let app = test_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/nonexistent-route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["code"], "NOT_FOUND");
    assert!(v["message"].is_string());
}

/// 错误上报 → 去重计数 → 错误中心列表（含权限校验）。
#[tokio::test]
async fn test_error_report_dedupe_and_list() {
    let state = AppState::new().await;
    clean_error_reports(&state).await;
    let app = test_router(state.clone());

    // 免鉴权上报两次相同指纹 → 合并计数
    let payload = serde_json::json!({
        "code": "TEST_DEDUPE",
        "message": "测试错误消息",
        "stack": "line1\nline2\nline3",
        "route": "/problems/1",
        "source": "frontend",
    });
    for _ in 0..2 {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/errors/report")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    // 未登录访问错误中心 → 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/errors")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 普通用户 → 403
    let guest = User {
        id: "guest-user-1".to_string(),
        username: "guest1".to_string(),
        display_name: "访客1".to_string(),
        avatar_url: None,
        email: None,
        role: "guest".to_string(),
        team_status: "none".to_string(),
        created_at: chrono::Utc::now(),
        bio: String::new(),
        password_hash: None,
        effective_role: "guest".to_string(),
        group_ids: Vec::new(),
        user_permissions: Vec::new(),
    };
    state.users.write().await.insert(guest.id.clone(), guest);
    let guest_token = create_session(&state, "guest-user-1").await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/errors")
                .header(auth_header(&guest_token).0, auth_header(&guest_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // 管理员 → 200，去重后 count=2
    let admin_token = create_session(&state, "admin").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/errors")
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["total"], 1);
    assert_eq!(v["errors"][0]["code"], "TEST_DEDUPE");
    assert_eq!(v["errors"][0]["count"], 2);
    assert_eq!(v["errors"][0]["status"], "open");
}

/// 上报接口按 IP 限流（20 次/分钟）。
#[tokio::test]
async fn test_error_report_rate_limited() {
    let state = AppState::new().await;
    clean_error_reports(&state).await;
    let app = test_router(state);

    let payload = serde_json::json!({ "code": "TEST_RATE", "message": "限流测试" });
    let mut last_status = StatusCode::OK;
    for _ in 0..25 {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/errors/report")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        last_status = res.status();
        if last_status == StatusCode::TOO_MANY_REQUESTS {
            let body = axum::body::to_bytes(res.into_body(), 1_000_000)
                .await
                .unwrap();
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(v["code"], "RATE_LIMITED");
            break;
        }
    }
    assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS);
}

/// 错误中心状态流转 / 删除 / 清空。
#[tokio::test]
async fn test_error_status_flow_and_clear() {
    let state = AppState::new().await;
    clean_error_reports(&state).await;
    let app = test_router(state.clone());

    let payload = serde_json::json!({ "code": "TEST_FLOW", "message": "状态流转测试" });
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/errors/report")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let admin_token = create_session(&state, "admin").await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/errors")
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = v["errors"][0]["id"].as_str().unwrap().to_string();

    // PATCH → resolved
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/errors/{}", id))
                .header("content-type", "application/json")
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::from(r#"{"status":"resolved"}"#.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 无效状态 → 400
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/errors/{}", id))
                .header("content-type", "application/json")
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::from(r#"{"status":"bogus"}"#.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 删除单条
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/errors/{}", id))
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 清空
    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/errors")
                .header(auth_header(&admin_token).0, auth_header(&admin_token).1)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

/// panic 兜底 → 500 INTERNAL_ERROR JSON（含 request_id）。
#[tokio::test]
async fn test_panic_returns_json_500() {
    async fn boom() -> &'static str {
        panic!("test panic")
    }

    let app = Router::new()
        .route("/boom", axum::routing::get(boom))
        .layer(tower_http::catch_panic::CatchPanicLayer::custom(
            error::panic_to_response,
        ))
        .layer(axum::middleware::from_fn(
            error::log_errors_and_inject_request_id,
        ))
        .layer(tower_http::request_id::SetRequestIdLayer::x_request_id(
            tower_http::request_id::MakeRequestUuid,
        ));

    let res = app
        .oneshot(Request::builder().uri("/boom").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["code"], "INTERNAL_ERROR");
    assert!(v["request_id"].is_string());
    assert!(!v["request_id"].as_str().unwrap().is_empty());
}

/// 保留策略：超过 2000 条时裁剪。
#[tokio::test]
async fn test_error_report_retention_prunes_old_rows() {
    let state = AppState::new().await;
    clean_error_reports(&state).await;

    // 直接插入 2001 条旧数据（不同指纹）
    for i in 0..2001 {
        let ts = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO error_reports \
             (id, ts, user_id, source, code, message, hint, suggestion, stack, url, \
              route, method, http_status, ua, plugin_id, fingerprint, count, status, \
              resolved_by, resolved_at, first_seen, last_seen) \
             VALUES (?, ?, NULL, 'backend', 'TEST_OLD', 'old', '', '', '', '', '', '', NULL, \
                     '', '', ?, 1, 'open', NULL, NULL, ?, ?)",
        )
        .bind(format!("old-{}", i))
        .bind(&ts)
        .bind(&ts)
        .bind(&ts)
        .bind(&ts)
        .execute(&state.db)
        .await
        .unwrap();
    }

    // 新上报触发裁剪
    let app = test_router(state.clone());
    let payload = serde_json::json!({ "code": "TEST_NEW", "message": "新错误" });
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/errors/report")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap(),
    )
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM error_reports")
        .fetch_one(&state.db)
        .await
        .unwrap();
    assert!(count <= 2000, "error_reports 超过保留上限: {}", count);
}

/// Verify that creating a new AppState loads config correctly
#[tokio::test]
async fn test_app_state_initialization() {
    let state = AppState::new().await;

    // Admin user must exist
    let users = state.users.read().await;
    let admin = users.get("admin").expect("admin user must exist");
    assert_eq!(admin.role, "superadmin", "admin user must be superadmin");
    assert_eq!(admin.username, "admin");
    assert_eq!(admin.team_status, "joined");

    // Admin must be a team member
    drop(users);
    let members = state.team_members.read().await;
    let team_admin = members.get("admin").expect("admin must be a team member");
    assert_eq!(team_admin.user_id, "admin");
    assert_eq!(team_admin.joined_at, "2024-01-01");
}

/// Verify difficulty configuration is loaded
#[tokio::test]
async fn test_difficulty_config_loaded() {
    let state = AppState::new().await;
    let dc = state.difficulty.read().await;
    assert!(!dc.levels.is_empty(), "difficulty config must have levels");
    // Should at least have some common difficulties
    assert!(
        dc.levels.contains_key("Blue") || dc.levels.contains_key("Easy"),
        "should contain at least one expected difficulty"
    );
}

/// Verify site info reflects config
#[tokio::test]
async fn test_site_config_loaded() {
    let state = AppState::new().await;
    assert!(!state.site_name.is_empty(), "site name must not be empty");
    // Default fallback uses localhost:3000 for tests without a config file
}

/// Verify that multiple problems can coexist
#[tokio::test]
async fn test_problem_state_operations() {
    let state = AppState::new().await;

    // Record the initial problem count (may contain seed data in some environments)
    let initial_count = state.problems.read().await.len();

    // Add a test problem with a unique ID
    let problem = Problem {
        id: "integration-test-unique-1".to_string(),
        title: "Test Problem".to_string(),
        author_id: "admin".to_string(),
        author_name: "管理员".to_string(),
        contest: String::new(),
        contest_id: None,
        difficulty: "Easy".to_string(),
        content: "Test content".to_string(),
        solution: None,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
        public_at: None,
        claimed_by: None,
        verifier_solution: None,
        verifiers: vec![],
        visible_to: vec![],
        link: None,
        remark: None,
        editable_by: vec![],
    };
    state
        .problems
        .write()
        .await
        .insert(problem.id.clone(), problem);

    // Verify it's there
    let problems = state.problems.read().await;
    assert_eq!(problems.len(), initial_count + 1);
    let p = problems.get("integration-test-unique-1").unwrap();
    assert_eq!(p.title, "Test Problem");
    assert_eq!(p.status, "pending");
}

/// Verify user role checks
#[tokio::test]
async fn test_role_based_access() {
    let state = AppState::new().await;

    let users = state.users.read().await;
    let admin = users.get("admin").unwrap();
    assert_eq!(admin.role, "superadmin");
    assert!(admin.role == "admin" || admin.role == "superadmin");
}

/// Verify OAuth config is loaded properly
#[tokio::test]
async fn test_oauth_config() {
    let state = AppState::new().await;
    // OAuth values should be loaded from config or defaults
    // In CI/test environments the values come from hardcoded defaults
    assert!(
        state.cpoauth_redirect_uri.contains("callback"),
        "redirect URI must be constructed from site_url and contain 'callback'"
    );
    // Client ID may be the default or overridden by env var
    println!("OAuth client_id length: {}", state.cpoauth_client_id.len());
}

/// Verify admin password is loaded from config
#[tokio::test]
async fn test_admin_password_loaded() {
    let state = AppState::new().await;
    assert!(
        !state.admin_password.read().await.is_empty(),
        "admin password must be configured"
    );
}

// ============== Problem "reply" (review) tests ==============

/// Helper: insert a pending problem with the given author and return its id.
async fn seed_pending_problem(state: &AppState, author_id: &str) -> String {
    let id = format!(
        "test-pending-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let problem = Problem {
        id: id.clone(),
        title: "待审核测试题".to_string(),
        author_id: author_id.to_string(),
        author_name: "测试出题人".to_string(),
        contest: String::new(),
        contest_id: None,
        difficulty: "Easy".to_string(),
        content: "测试内容".to_string(),
        solution: None,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
        public_at: None,
        claimed_by: None,
        verifier_solution: None,
        verifiers: vec![],
        visible_to: vec![],
        link: None,
        remark: None,
        editable_by: vec![],
    };
    state.insert_problem(&problem).await;
    id
}

/// Reply to a pending problem keeps it pending and notifies the author.
#[tokio::test]
async fn test_reply_problem_keeps_pending_and_notifies_author() {
    let state = AppState::new().await;
    let author_id = "member1".to_string();
    let problem_id = seed_pending_problem(&state, &author_id).await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reply", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"请补充数据范围说明"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true);

    // Problem must still exist and remain pending (not deleted)
    let problems = state.problems.read().await;
    let p = problems
        .get(&problem_id)
        .expect("problem should not be deleted");
    assert_eq!(p.status, "pending");

    // Author received a notification with the suggestion
    let notifications = state.notifications.read().await;
    let has_notif = notifications.values().any(|n| {
        n.user_id == author_id && n.title == "题目审核建议" && n.body.contains("请补充数据范围说明")
    });
    assert!(has_notif, "author should receive a reply notification");
}

/// Reply requires a non-empty reason.
#[tokio::test]
async fn test_reply_problem_requires_reason() {
    let state = AppState::new().await;
    let problem_id = seed_pending_problem(&state, "member1").await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reply", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], false);
    assert!(v["message"].as_str().unwrap().contains("建议"));

    // Problem remains pending (untouched)
    let problems = state.problems.read().await;
    assert!(problems.get(&problem_id).is_some());
}

/// Non-admin (member) cannot reply to a problem.
#[tokio::test]
async fn test_reply_problem_requires_permission() {
    let state = AppState::new().await;
    // Insert a member user (joined) lacking approve_all_problems
    let member = User {
        id: "member2".to_string(),
        username: "member2".to_string(),
        display_name: "普通成员".to_string(),
        avatar_url: None,
        email: None,
        role: "member".to_string(),
        team_status: "joined".to_string(),
        created_at: chrono::Utc::now(),
        bio: String::new(),
        password_hash: None,
        effective_role: "member".to_string(),
        group_ids: vec![],
        user_permissions: vec![],
    };
    state.users.write().await.insert(member.id.clone(), member);

    let problem_id = seed_pending_problem(&state, "member2").await;
    let token = create_session(&state, "member2").await;
    let app = test_router(state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reply", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], false);
    assert_eq!(v["message"], "权限不足");

    // No notification was sent
    let notifications = state.notifications.read().await;
    assert!(notifications.values().all(|n| n.title != "题目审核建议"));
}

// ============== Global plugin disable tests ==============

/// Superadmin can globally disable the plugin feature.
#[tokio::test]
async fn test_global_plugin_disable() {
    let state = AppState::new().await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // Disable globally
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/plugins/global")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["plugins_disabled"], true);

    // State reflects the disable
    assert!(*state.plugins_disabled.read().await);

    // Public list reflects the flag
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/plugins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list["plugins_disabled"], true);

    // Re-enable
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/plugins/global")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"enabled":true}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(v["plugins_disabled"], false);
    assert!(!*state.plugins_disabled.read().await);
}

/// Non-superadmin cannot toggle the global plugin switch.
#[tokio::test]
async fn test_global_plugin_disable_requires_permission() {
    let state = AppState::new().await;
    let member = User {
        id: "member3".to_string(),
        username: "member3".to_string(),
        display_name: "成员三".to_string(),
        avatar_url: None,
        email: None,
        role: "member".to_string(),
        team_status: "joined".to_string(),
        created_at: chrono::Utc::now(),
        bio: String::new(),
        password_hash: None,
        effective_role: "member".to_string(),
        group_ids: vec![],
        user_permissions: vec![],
    };
    state.users.write().await.insert(member.id.clone(), member);
    let token = create_session(&state, "member3").await;
    let app = test_router(state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/plugins/global")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"enabled":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    // State unchanged
    assert!(!*state.plugins_disabled.read().await);
}

// ============== "reject" (待审核 -> 已退回) tests ==============

/// Rejecting a pending problem requires a reason of at least 10 chars.
#[tokio::test]
async fn test_reject_problem_requires_reason_at_least_10_chars() {
    let state = AppState::new().await;
    let problem_id = seed_pending_problem(&state, "member1").await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // Short reason (rejected)
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reject", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"reason":"太短"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], false);
    assert!(v["message"].as_str().unwrap().contains("10"));

    // Still pending
    assert_eq!(
        state.problems.read().await.get(&problem_id).unwrap().status,
        "pending"
    );

    // Valid reason (>= 10 chars)
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reject", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"reason":"题目描述不够清晰需要补充更多细节。"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let p = state
        .problems
        .read()
        .await
        .get(&problem_id)
        .unwrap()
        .clone();
    assert_eq!(p.status, "returned");
    assert!(p.remark.is_some());
}

/// Resubmitting a returned problem brings it back to pending.
#[tokio::test]
async fn test_resubmit_returned_problem() {
    let state = AppState::new().await;

    // Ensure the author (member1) exists as a joined member
    let author = User {
        id: "member1".to_string(),
        username: "member1".to_string(),
        display_name: "测试出题人".to_string(),
        avatar_url: None,
        email: None,
        role: "member".to_string(),
        team_status: "joined".to_string(),
        created_at: chrono::Utc::now(),
        bio: String::new(),
        password_hash: None,
        effective_role: "member".to_string(),
        group_ids: vec![],
        user_permissions: vec![],
    };
    state
        .users
        .write()
        .await
        .insert("member1".to_string(), author);

    let problem_id = seed_pending_problem(&state, "member1").await;
    let admin_token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // Reject first (admin) -> returned
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/reject", problem_id))
                .header("Authorization", format!("Bearer {}", admin_token))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"reason":"题目描述不够清晰需要补充更多细节。"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        state.problems.read().await.get(&problem_id).unwrap().status,
        "returned"
    );

    // Author (member1) resubmits -> pending
    let author_token = create_session(&state, "member1").await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/{}/resubmit", problem_id))
                .header("Authorization", format!("Bearer {}", author_token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let p = state
        .problems
        .read()
        .await
        .get(&problem_id)
        .unwrap()
        .clone();
    assert_eq!(p.status, "pending");
    assert!(p.remark.is_none());
}

// ============== Multi-verifier claim tests ==============

/// Multiple distinct members can claim the same approved problem.
#[tokio::test]
async fn test_multi_verifier_claim() {
    let state = AppState::new().await;

    for (id, name) in [("v1", "验题人一"), ("v2", "验题人二")] {
        let member = User {
            id: id.to_string(),
            username: id.to_string(),
            display_name: name.to_string(),
            avatar_url: None,
            email: None,
            role: "member".to_string(),
            team_status: "joined".to_string(),
            created_at: chrono::Utc::now(),
            bio: String::new(),
            password_hash: None,
            effective_role: "member".to_string(),
            group_ids: vec![],
            user_permissions: vec![],
        };
        state.users.write().await.insert(id.to_string(), member);
    }

    let author = User {
        id: "author".to_string(),
        username: "author".to_string(),
        display_name: "出题人".to_string(),
        avatar_url: None,
        email: None,
        role: "member".to_string(),
        team_status: "joined".to_string(),
        created_at: chrono::Utc::now(),
        bio: String::new(),
        password_hash: None,
        effective_role: "member".to_string(),
        group_ids: vec![],
        user_permissions: vec![],
    };
    state
        .users
        .write()
        .await
        .insert("author".to_string(), author);
    let problem = Problem {
        id: "multi-verifier-problem".to_string(),
        title: "多验题人测试题".to_string(),
        author_id: "author".to_string(),
        author_name: "出题人".to_string(),
        contest: String::new(),
        contest_id: None,
        difficulty: "Easy".to_string(),
        content: "内容".to_string(),
        solution: None,
        status: "approved".to_string(),
        created_at: chrono::Utc::now(),
        public_at: None,
        claimed_by: None,
        verifier_solution: None,
        verifiers: vec![],
        visible_to: vec![],
        link: None,
        remark: None,
        editable_by: vec![],
    };
    state.insert_problem(&problem).await;

    let app = test_router(state.clone());

    let t1 = create_session(&state, "v1").await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/problems/claim/multi-verifier-problem")
                .header("Authorization", format!("Bearer {}", t1))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let t2 = create_session(&state, "v2").await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/problems/claim/multi-verifier-problem")
                .header("Authorization", format!("Bearer {}", t2))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let p = state
        .problems
        .read()
        .await
        .get("multi-verifier-problem")
        .unwrap()
        .clone();
    assert_eq!(p.verifiers.len(), 2);
    assert_eq!(p.claimed_by.as_deref(), Some("v1"));
}

/// A verifier can post a comment; a non-verifier cannot.
#[tokio::test]
async fn test_verifier_comment() {
    let state = AppState::new().await;
    let problem = Problem {
        id: "comment-problem".to_string(),
        title: "评论测试题".to_string(),
        author_id: "someone".to_string(),
        author_name: "某人".to_string(),
        contest: String::new(),
        contest_id: None,
        difficulty: "Easy".to_string(),
        content: "内容".to_string(),
        solution: None,
        status: "approved".to_string(),
        created_at: chrono::Utc::now(),
        public_at: None,
        claimed_by: None,
        verifier_solution: None,
        verifiers: vec![],
        visible_to: vec![],
        link: None,
        remark: None,
        editable_by: vec![],
    };
    state.insert_problem(&problem).await;

    for (id, name) in [("cv", "评论验题人"), ("other", "其他人")] {
        let member = User {
            id: id.to_string(),
            username: id.to_string(),
            display_name: name.to_string(),
            avatar_url: None,
            email: None,
            role: "member".to_string(),
            team_status: "joined".to_string(),
            created_at: chrono::Utc::now(),
            bio: String::new(),
            password_hash: None,
            effective_role: "member".to_string(),
            group_ids: vec![],
            user_permissions: vec![],
        };
        state.users.write().await.insert(id.to_string(), member);
    }

    let app = test_router(state.clone());

    // Non-verifier cannot comment
    let t_other = create_session(&state, "other").await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/problems/verifier-comment/comment-problem")
                .header("Authorization", format!("Bearer {}", t_other))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], false);

    // "cv" claims then comments
    let t_cv = create_session(&state, "cv").await;
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/problems/claim/comment-problem")
                .header("Authorization", format!("Bearer {}", t_cv))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/problems/verifier-comment/comment-problem")
                .header("Authorization", format!("Bearer {}", t_cv))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"这是一个验题评论"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let p = state
        .problems
        .read()
        .await
        .get("comment-problem")
        .unwrap()
        .clone();
    assert_eq!(p.verifiers.len(), 1);
    assert_eq!(p.verifiers[0].comments.len(), 1);
    assert_eq!(p.verifiers[0].comments[0].content, "这是一个验题评论");
}
