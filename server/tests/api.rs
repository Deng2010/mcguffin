// ============== Integration Tests ==============
//
// These tests verify the core behavior of AppState, types,
// and utility functions in a realistic environment.
// Since the server config exists at /usr/share/mcguffin/config.toml,
// AppState::new() will properly initialize.

use mcguffin_server_lib::*;

/// Verify that creating a new AppState loads config correctly
#[test]
fn test_app_state_initialization() {
    let state = AppState::new();

    // Admin user must exist
    let users = state.users.blocking_read();
    let admin = users.get("admin").expect("admin user must exist");
    assert_eq!(admin.role, "superadmin", "admin user must be superadmin");
    assert_eq!(admin.username, "admin");
    assert_eq!(admin.team_status, "joined");

    // Admin must be a team member
    let members = state.team_members.blocking_read();
    let team_admin = members.get("admin").expect("admin must be a team member");
    assert_eq!(team_admin.user_id, "admin");
    // Role is now stored in users table, not team_members
    drop(members);
    assert_eq!(admin.role, "superadmin", "admin user must be superadmin");
    // Verify team member has only basic fields
    let members = state.team_members.blocking_read();
    let team_admin = members.get("admin").expect("admin must be a team member");
    assert_eq!(team_admin.joined_at, "2024-01-01");
}

/// Verify difficulty configuration is loaded
#[test]
fn test_difficulty_config_loaded() {
    let state = AppState::new();
    let dc = state.difficulty.blocking_read();
    assert!(!dc.levels.is_empty(), "difficulty config must have levels");
    // Should at least have some common difficulties
    assert!(dc.levels.contains_key("Blue") || dc.levels.contains_key("Easy"),
        "should contain at least one expected difficulty");
}

/// Verify site info reflects config
#[test]
fn test_site_config_loaded() {
    let state = AppState::new();
    assert!(!state.site_name.is_empty(), "site name must not be empty");
    assert_eq!(state.site_url, "https://lba-oi.team");
}

/// Verify that multiple problems can coexist
#[test]
fn test_problem_state_operations() {
    let state = AppState::new();

    // Record the initial problem count (may contain seed data in some environments)
    let initial_count = state.problems.blocking_read().len();

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
    };
    state.problems.blocking_write().insert(problem.id.clone(), problem);

    // Verify it's there
    let problems = state.problems.blocking_read();
    assert_eq!(problems.len(), initial_count + 1);
    let p = problems.get("integration-test-unique-1").unwrap();
    assert_eq!(p.title, "Test Problem");
    assert_eq!(p.status, "pending");
}

/// Verify user role checks
#[test]
fn test_role_based_access() {
    let state = AppState::new();

    let users = state.users.blocking_read();
    let admin = users.get("admin").unwrap();
    assert_eq!(admin.role, "superadmin");

    // Admin has all admin permissions via role check
    let is_admin_user = admin.role == "admin" || admin.role == "superadmin";
    assert!(is_admin_user);
}

/// Verify OAuth config is loaded properly
#[test]
fn test_oauth_config() {
    let state = AppState::new();
    assert!(!state.cpoauth_client_id.is_empty(), "OAuth client ID must be configured");
    assert!(state.cpoauth_redirect_uri.contains("callback"),
        "redirect URI must contain callback");
}

/// Verify admin password is loaded from config
#[test]
fn test_admin_password_loaded() {
    let state = AppState::new();
    assert!(!state.admin_password.is_empty(), "admin password must be configured");
}
