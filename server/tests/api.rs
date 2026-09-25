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

// ============== Problem review / edit payload edge cases ==============

/// approve/publish/unpublish 不带 body，但前端仍会带上
/// `Content-Type: application/json`（apiFetch 统一设置），此时绝不能 400。
#[tokio::test]
async fn test_review_problem_accepts_empty_json_body() {
    let state = AppState::new().await;
    let problem_id = seed_pending_problem(&state, "member1").await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/problems/review/{}/approve", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "空 body + JSON Content-Type 不应被 axum 的 Json 提取器拒绝"
    );
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true);
    assert_eq!(
        state.problems.read().await.get(&problem_id).unwrap().status,
        "approved"
    );
}

/// 编辑题目：`null` 表示清空（remark / contest_id / link），字段缺失表示不修改。
#[tokio::test]
async fn test_edit_problem_null_clears_optional_fields() {
    let state = AppState::new().await;
    let problem_id = seed_pending_problem(&state, "member1").await;
    {
        let mut problems = state.problems.write().await;
        let p = problems.get_mut(&problem_id).unwrap();
        p.remark = Some("旧的审核备注".to_string());
        p.contest_id = Some("contest-1".to_string());
        p.link = Some("https://example.com/problem".to_string());
    }

    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // 只清空 remark：contest_id / link 未出现在 body 中 → 保持不变
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/problems/{}", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"remark":null}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], true, "清空 remark 应成功: {v}");

    {
        let problems = state.problems.read().await;
        let after = problems.get(&problem_id).unwrap();
        assert!(after.remark.is_none(), "null 应清空 remark");
        assert_eq!(
            after.contest_id.as_deref(),
            Some("contest-1"),
            "未提交的 contest_id 不应被修改"
        );
        assert_eq!(
            after.link.as_deref(),
            Some("https://example.com/problem"),
            "未提交的 link 不应被修改"
        );
    }

    // 空字符串同样等价于清空；contest_id / link 用 null 清空
    let res = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/problems/{}", problem_id))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"remark":"","contest_id":null,"link":null}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let problems = state.problems.read().await;
    let after = problems.get(&problem_id).unwrap();
    assert!(after.remark.is_none(), "空字符串应清空 remark");
    assert!(after.contest_id.is_none(), "null 应清空 contest_id");
    assert!(after.link.is_none(), "null 应清空 link");
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

// ============== Plugin data API & persistence tests ==============

/// 唯一插件 id（测试共享同一个 SQLite 文件，避免相互污染）。
fn uniq_plugin_id(prefix: &str) -> String {
    format!(
        "t-{}-{}",
        prefix,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

/// 通过 API 注册一个插件，返回 (router, token)。
async fn register_plugin_api(
    state: &AppState,
    plugin_id: &str,
    perms: &[&str],
) -> (Router, String) {
    let token = create_session(state, "admin").await;
    let app = test_router(state.clone());
    let body = serde_json::json!({
        "id": plugin_id,
        "manifest": {
            "id": plugin_id,
            "name": plugin_id,
            "version": "1.0.0",
            "description": "",
            "permissions_needed": perms,
        },
        "permissions": perms,
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plugins/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    (app, token)
}

fn authed_json_post(uri: String, token: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Authorization", format!("Bearer {}", token))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// KV 读写 + 原子计数器 + keys 列举 + SQLite 持久化。
#[tokio::test]
async fn test_plugin_kv_counter_keys_and_persistence() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("kv");
    let (app, token) = register_plugin_api(&state, &pid, &["storage"]).await;

    // KV 写入
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "counter", "value": "1"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 原子加（基于已有值 "1"）
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data/add", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "counter", "delta": 4}),
        ))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["value"], 5);

    // 原子减
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data/add", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "counter", "delta": -2}),
        ))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["value"], 3);

    // KV 读取
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/plugins/{}/data?namespace=ns&key=counter",
                    pid
                ))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["value"], "3");

    // keys 列举
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/data/keys?namespace=ns", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["keys"], serde_json::json!(["counter"]));

    // SQLite 已持久化 KV 与清单
    let row = sqlx::query(
        "SELECT value FROM plugin_data WHERE plugin_id = ? AND namespace = 'ns' AND key = 'counter'",
    )
    .bind(&pid)
    .fetch_one(&state.db)
    .await
    .expect("plugin_data row should exist");
    use sqlx::Row;
    let value: String = row.get("value");
    assert_eq!(value, "3");

    let row = sqlx::query("SELECT source, enabled FROM plugins WHERE id = ?")
        .bind(&pid)
        .fetch_one(&state.db)
        .await
        .expect("plugins row should exist");
    let source: String = row.get("source");
    assert_eq!(source, "code");

    // 清理
    let _ = std::fs::remove_dir_all(format!("plugins/{}", pid));
    let _ = sqlx::query("DELETE FROM plugin_data WHERE plugin_id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// 字符串集合：add / members / is-member / remove。
#[tokio::test]
async fn test_plugin_set_operations() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("set");
    let (app, token) = register_plugin_api(&state, &pid, &["storage"]).await;

    for (member, expect_added) in [("u1", true), ("u1", false), ("u2", true)] {
        let res = app
            .clone()
            .oneshot(authed_json_post(
                format!("/api/plugins/{}/data/set-add", pid),
                &token,
                serde_json::json!({"namespace": "ns", "key": "s", "member": member}),
            ))
            .await
            .unwrap();
        let v = body_json(res).await;
        assert_eq!(v["added"], expect_added, "set-add {}", member);
    }

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/plugins/{}/data/set-members?namespace=ns&key=s",
                    pid
                ))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["count"], 2);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/plugins/{}/data/set-is-member?namespace=ns&key=s&member=u3",
                    pid
                ))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["is_member"], false);

    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data/set-remove", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "s", "member": "u1"}),
        ))
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["removed"], true);

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/plugins/{}/data/set-members?namespace=ns&key=s",
                    pid
                ))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["members"], serde_json::json!(["u2"]));

    let _ = sqlx::query("DELETE FROM plugin_data WHERE plugin_id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// 未申请 storage 权限的插件访问数据接口 → 403。
#[tokio::test]
async fn test_plugin_data_requires_storage_perm() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("noperm");
    let (app, token) = register_plugin_api(&state, &pid, &[]).await;

    let res = app
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "a", "value": "1"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let v = body_json(res).await;
    assert_eq!(v["code"], "PLUGIN_PERMISSION_DENIED");

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// 插件文件存储：写 / 读 / 列 / 删 + 路径穿越防护。
#[tokio::test]
async fn test_plugin_file_api() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("file");
    let (app, token) = register_plugin_api(&state, &pid, &["storage"]).await;

    // 写文件
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plugins/{}/files/docs/hello.txt", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from("hello plugin"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["path"], "docs/hello.txt");
    assert_eq!(v["size"], 12);

    // 读文件
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/files/docs/hello.txt", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    assert_eq!(&body[..], b"hello plugin");

    // 列出文件
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/files/list", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let v = body_json(res).await;
    assert_eq!(v["files"], serde_json::json!(["docs/hello.txt"]));

    // 路径穿越被拒绝
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/plugins/{}/files/..%2Fevil.txt", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::from("evil"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 删除文件
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/plugins/{}/files/docs/hello.txt", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 删除后读取 → 404
    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/files/docs/hello.txt", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // 清理磁盘与 DB
    let _ = std::fs::remove_dir_all(format!("plugins/{}", pid));
    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// ZIP 安装：上传 → 资产可公开访问 → 卸载后清理。
#[tokio::test]
async fn test_plugin_install_zip_and_assets() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("zip");
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // 构造内存 zip（plugin.json + index.js）
    let zip_bytes = {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        w.start_file("plugin.json", opts).unwrap();
        std::io::Write::write_all(
            &mut w,
            serde_json::to_vec(&serde_json::json!({
                "id": pid,
                "name": "ZIP 测试插件",
                "version": "1.2.3",
                "permissions_needed": ["storage"],
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap();
        w.start_file("index.js", opts).unwrap();
        std::io::Write::write_all(&mut w, b"console.log('hi');".as_slice()).unwrap();
        w.finish().unwrap().into_inner()
    };

    // 安装
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/plugins/install-zip")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/octet-stream")
                .body(Body::from(zip_bytes))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["success"], true);
    assert_eq!(v["plugin"]["source"], "zip");
    assert_eq!(v["plugin"]["entry"], "index.js");

    // 资产公开可访问（无需 token）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/assets/index.js", pid))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    assert_eq!(&body[..], b"console.log('hi');");

    // 公开列表带 source 字段
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
    let v = body_json(res).await;
    let found = v["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == pid)
        .cloned();
    assert!(found.is_some(), "public list should contain zip plugin");
    assert_eq!(found.unwrap()["source"], "zip");

    // 卸载 → 资产目录与 DB 记录被清理
    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/plugins/{}", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(!std::path::Path::new(&format!("plugins/{}", pid)).exists());
}

/// ZIP 更新：替换资产、保留启用状态与已授权权限、提示新申请的权限、插件数据不丢。
#[tokio::test]
async fn test_plugin_update_zip_preserves_state() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("upd");
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    /// 构造内存 zip（plugin.json + index.js）。
    fn make_zip(id: &str, version: &str, perms: &[&str], js: &str) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        w.start_file("plugin.json", opts).unwrap();
        std::io::Write::write_all(
            &mut w,
            serde_json::to_vec(&serde_json::json!({
                "id": id,
                "name": "更新测试插件",
                "version": version,
                "permissions_needed": perms,
            }))
            .unwrap()
            .as_slice(),
        )
        .unwrap();
        w.start_file("index.js", opts).unwrap();
        std::io::Write::write_all(&mut w, js.as_bytes()).unwrap();
        w.finish().unwrap().into_inner()
    }

    async fn upload(app: Router, uri: String, token: &str, bytes: Vec<u8>) -> serde_json::Value {
        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(bytes))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "上传插件包应成功");
        body_json(res).await
    }

    // 1) 安装 v1（permissions_needed = storage）
    let v1 = upload(
        app.clone(),
        "/api/admin/plugins/install-zip".to_string(),
        &token,
        make_zip(&pid, "1.0.0", &["storage"], "v1"),
    )
    .await;
    assert_eq!(v1["updated"], false);
    assert_eq!(
        v1["plugin"]["permissions_needed"],
        serde_json::json!(["storage"])
    );

    // 写入插件 KV 数据，稍后验证更新不影响数据
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "k", "value": "keep-me"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 禁用插件，验证更新后禁用状态保留
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/plugins/{}/disable", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2) 用 id 不一致的包更新 → 400（防止误把别的插件当更新装进来）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/plugins/{}/update-zip", pid))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/octet-stream")
                .body(Body::from(make_zip(
                    "some-other-plugin",
                    "9.9.9",
                    &["storage"],
                    "evil",
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let v = body_json(res).await;
    assert_eq!(v["code"], "PLUGIN_INVALID_PACKAGE");

    // 3) 更新到 v2：新版本额外申请 read:team，权限保持冻结、禁用状态保留
    let v2 = upload(
        app.clone(),
        format!("/api/admin/plugins/{}/update-zip", pid),
        &token,
        make_zip(&pid, "2.0.0", &["storage", "read:team"], "v2"),
    )
    .await;
    assert_eq!(v2["updated"], true);
    assert_eq!(v2["previous_version"], "1.0.0");
    assert_eq!(v2["plugin"]["version"], "2.0.0");
    assert_eq!(v2["plugin"]["enabled"], false);
    assert_eq!(
        v2["plugin"]["permissions_needed"],
        serde_json::json!(["storage"])
    );
    assert_eq!(v2["new_permissions"], serde_json::json!(["read:team"]));

    // 插件数据仍在
    let data = state.plugin_data.read().await;
    assert_eq!(
        data.get(&pid)
            .and_then(|ns| ns.get("ns"))
            .and_then(|kv| kv.get("k"))
            .map(String::as_str),
        Some("keep-me")
    );
    drop(data);

    // 4) 重新启用后，资产已替换为 v2
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/plugins/{}/enable", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/assets/index.js", pid))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 1_000_000)
        .await
        .unwrap();
    assert_eq!(&body[..], b"v2");

    // 5) 非 URL 安装的插件调用「从原 URL 更新」→ 409 PLUGIN_UPDATE_UNAVAILABLE
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/plugins/{}/update", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let v = body_json(res).await;
    assert_eq!(v["code"], "PLUGIN_UPDATE_UNAVAILABLE");

    // 清理磁盘与 DB
    let _ = std::fs::remove_dir_all(format!("plugins/{}", pid));
    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("DELETE FROM plugin_data WHERE plugin_id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// 损坏的 zip 包 → 400 PLUGIN_INVALID_PACKAGE。
#[tokio::test]
async fn test_plugin_install_zip_rejects_garbage() {
    let state = AppState::new().await;
    let token = create_session(&state, "admin").await;
    let app = test_router(state);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/plugins/install-zip")
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/octet-stream")
                .body(Body::from("not a zip file"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let v = body_json(res).await;
    assert_eq!(v["code"], "PLUGIN_INVALID_PACKAGE");
}

/// 重注册不能修改已注册插件的权限（防止无鉴权接口被用来扩权）。
#[tokio::test]
async fn test_plugin_reregister_cannot_change_permissions() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("freeze");
    let (app, _token) = register_plugin_api(&state, &pid, &["storage"]).await;

    // 用更宽的权限重注册
    let body = serde_json::json!({
        "id": pid,
        "manifest": {
            "id": pid,
            "name": pid,
            "version": "2.0.0",
            "description": "",
            "permissions_needed": ["storage", "read:users:email"],
        },
        "permissions": ["storage", "read:users:email"],
    });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/plugins/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let plugins = state.plugins.read().await;
    let manifest = plugins.get(&pid).expect("plugin registered");
    assert_eq!(
        manifest.permissions,
        vec!["storage".to_string()],
        "重注册不得改变权限清单"
    );
    assert_eq!(manifest.version, "2.0.0", "元信息应刷新");
    drop(plugins);

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// write:team 蕴含 read:team：只申请 write:team 的插件也能列成员。
#[tokio::test]
async fn test_plugin_write_team_implies_read_team() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("implyteam");
    let (app, token) = register_plugin_api(&state, &pid, &["write:team"]).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/users", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "write:team 应蕴含 read:team（而不是 403）"
    );

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// read:users:email 蕴含 read:users：能读取用户资料。
#[tokio::test]
async fn test_plugin_read_users_email_implies_read_users() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("implyusers");
    let (app, token) = register_plugin_api(&state, &pid, &["read:users:email"]).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/plugins/{}/users/admin", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["id"], "admin");
    assert!(
        v.get("email").is_some(),
        "read:users:email 应包含 email 字段"
    );

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// superadmin 可通过管理接口调整插件权限，非法权限被拒绝。
#[tokio::test]
async fn test_admin_set_plugin_permissions() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("perms");
    let (app, token) = register_plugin_api(&state, &pid, &["storage"]).await;

    // 正常调整（含去重）
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/plugins/{}/permissions", pid))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "permissions": ["read:team", "storage", "read:team"],
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(
        v["permissions_needed"],
        serde_json::json!(["read:team", "storage"])
    );

    // 已持久化
    let row = sqlx::query("SELECT permissions FROM plugins WHERE id = ?")
        .bind(&pid)
        .fetch_one(&state.db)
        .await
        .expect("plugins row");
    use sqlx::Row;
    let perms: String = row.get("permissions");
    assert_eq!(perms, r#"["read:team","storage"]"#);

    // 非法权限 → 400
    let res = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/admin/plugins/{}/permissions", pid))
                .header("Authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "permissions": ["root:everything"],
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// KV 配额：超大值 / 超长 key 被拒。
#[tokio::test]
async fn test_plugin_kv_quota_limits() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("quota");
    let (app, token) = register_plugin_api(&state, &pid, &["storage"]).await;

    // 64 KiB + 1 的值 → 400
    let big = "x".repeat(64 * 1024 + 1);
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "k", "value": big}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let v = body_json(res).await;
    assert_eq!(v["code"], "PLUGIN_DATA_INVALID");

    // 超长 key → 400
    let res = app
        .clone()
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "k".repeat(257), "value": "v"}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 边界内的值可以写入
    let ok = "x".repeat(1024);
    let res = app
        .oneshot(authed_json_post(
            format!("/api/plugins/{}/data", pid),
            &token,
            serde_json::json!({"namespace": "ns", "key": "ok", "value": ok}),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let _ = sqlx::query("DELETE FROM plugin_data WHERE plugin_id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}

/// 插件生命周期操作写入审计日志。
#[tokio::test]
async fn test_plugin_lifecycle_audited() {
    let state = AppState::new().await;
    let pid = uniq_plugin_id("audit");
    let token = create_session(&state, "admin").await;
    let app = test_router(state.clone());

    // 注册 + 禁用（走管理接口，带审计）
    let _ = register_plugin_api(&state, &pid, &["storage"]).await;
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/plugins/{}/disable", pid))
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 审计日志包含 plugin.disable + 资源标识
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit-log")
                .header("Authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    // GET /api/admin/audit-log 直接返回条目数组
    let entries = v.as_array().expect("audit entries array");
    let found = entries
        .iter()
        .any(|e| e["action"] == "plugin.disable" && e["resource"] == format!("plugin:{}", pid));
    assert!(
        found,
        "审计日志应包含 plugin.disable（resource=plugin:{}），实际: {:?}",
        pid,
        entries
            .iter()
            .filter(|e| e["action"].as_str().unwrap_or("").starts_with("plugin."))
            .collect::<Vec<_>>()
    );

    let _ = sqlx::query("DELETE FROM plugins WHERE id = ?")
        .bind(&pid)
        .execute(&state.db)
        .await;
}
