// ============== Integration Tests ==============
//
// These tests verify the core behavior of AppState, types,
// and utility functions in a realistic environment.

use mcguffin_server_lib::*;

/// Verify that creating a new AppState loads config correctly
#[tokio::test]
async fn test_app_state_initialization() {
    let state = AppState::new().await;

    // Admin user must exist
    let users = state.users.lock().await;
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

    let users = state.users.lock().await;
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
