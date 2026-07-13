use std::collections::HashMap;

/// All permission identifiers used in the system.
/// These are the canonical names — both backend and frontend use them.
pub mod perms {
    // ── Public / Guest ──
    pub const VIEW_SHOWCASE: &str = "view_showcase";
    pub const APPLY_JOIN: &str = "apply_join";

    // ── Team ──
    pub const VIEW_TEAM: &str = "view_team";
    /// Approve/reject join requests
    pub const MANAGE_TEAM: &str = "manage_team";
    /// Kick members, change roles
    pub const MANAGE_MEMBERS: &str = "manage_members";

    // ── Problems ──
    pub const SUBMIT_PROBLEM: &str = "submit_problem";
    pub const VIEW_PROBLEMS: &str = "view_problems";
    pub const APPROVE_PROBLEM: &str = "approve_problem";

    // ── Contests ──
    pub const MANAGE_CONTESTS: &str = "manage_contests";
    /// View all contests including drafts
    pub const VIEW_ALL_CONTESTS: &str = "view_all_contests";
    /// View only public contests
    pub const VIEW_PUBLIC_CONTESTS: &str = "view_public_contests";

    // ── Site ──
    pub const MANAGE_SITE: &str = "manage_site";

    // ── Discussions / Community ──
    pub const VIEW_DISCUSSIONS: &str = "view_discussions";
    pub const MANAGE_DISCUSSIONS: &str = "manage_discussions";
    pub const MANAGE_TAGS: &str = "manage_tags";

    // ── System ──
    pub const EDIT_SHOWCASE: &str = "edit_showcase";
    pub const MANAGE_NOTIFICATIONS: &str = "manage_notifications";
    pub const MANAGE_BACKUPS: &str = "manage_backups";
    pub const VIEW_STATS: &str = "view_stats";
    /// Manage unified posts (discussions, suggestions, announcements).
    /// Replaces the deprecated manage_discussions.
    pub const MANAGE_POSTS: &str = "manage_posts";

    /// All defined permissions (used for config validation)
    pub const ALL: &[&str] = &[
        VIEW_SHOWCASE,
        APPLY_JOIN,
        VIEW_TEAM,
        MANAGE_TEAM,
        MANAGE_MEMBERS,
        SUBMIT_PROBLEM,
        VIEW_PROBLEMS,
        APPROVE_PROBLEM,
        MANAGE_CONTESTS,
        VIEW_ALL_CONTESTS,
        VIEW_PUBLIC_CONTESTS,
        MANAGE_SITE,
        EDIT_SHOWCASE,
        VIEW_DISCUSSIONS,
        MANAGE_DISCUSSIONS,
        MANAGE_TAGS,
        MANAGE_NOTIFICATIONS,
        MANAGE_BACKUPS,
        VIEW_STATS,
        MANAGE_POSTS,
    ];
}

/// The special wildcard permission meaning "all permissions" (superadmin only).
pub const PERM_WILDCARD: &str = "*";

/// Return the default role→permissions mapping.
pub fn default_role_permissions() -> HashMap<String, Vec<String>> {
    let mut m = HashMap::new();
    // superadmin gets wildcard + all explicit permissions (frontend doesn't understand wildcards)
    let all_perms: Vec<String> = perms::ALL.iter().map(|p| p.to_string()).collect();
    let mut superadmin_perms = vec![PERM_WILDCARD.to_string()];
    superadmin_perms.extend(all_perms);
    m.insert("superadmin".to_string(), superadmin_perms);
    m.insert(
        "admin".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::VIEW_TEAM.to_string(),
            perms::MANAGE_TEAM.to_string(),
            perms::MANAGE_MEMBERS.to_string(),
            perms::SUBMIT_PROBLEM.to_string(),
            perms::VIEW_PROBLEMS.to_string(),
            perms::APPROVE_PROBLEM.to_string(),
            perms::MANAGE_CONTESTS.to_string(),
            perms::VIEW_ALL_CONTESTS.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::MANAGE_SITE.to_string(),
            perms::EDIT_SHOWCASE.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
            perms::MANAGE_POSTS.to_string(),
            perms::MANAGE_TAGS.to_string(),
            perms::MANAGE_NOTIFICATIONS.to_string(),
            perms::VIEW_STATS.to_string(),
        ],
    );
    m.insert(
        "member".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::VIEW_TEAM.to_string(),
            perms::SUBMIT_PROBLEM.to_string(),
            perms::VIEW_PROBLEMS.to_string(),
            perms::VIEW_ALL_CONTESTS.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
        ],
    );
    m.insert(
        "guest".to_string(),
        vec![
            perms::VIEW_SHOWCASE.to_string(),
            perms::APPLY_JOIN.to_string(),
            perms::VIEW_PUBLIC_CONTESTS.to_string(),
            perms::VIEW_DISCUSSIONS.to_string(),
        ],
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify all permissions in default_role_permissions() are in perms::ALL.
    /// This catches drift when adding new permissions — prevents the frontend
    /// from being out of sync with the backend's known permission set.
    #[test]
    fn test_all_role_permissions_are_in_known_set() {
        let role_perms = default_role_permissions();
        let known: std::collections::HashSet<&str> = perms::ALL.iter().cloned().collect();
        for (role, perms) in &role_perms {
            for p in perms {
                if p == PERM_WILDCARD {
                    continue; // wildcard is not in perms::ALL
                }
                assert!(
                    known.contains(p.as_str()),
                    "权限「{}」（角色: {}）不在 perms::ALL 中，请先在 perms 模块中定义",
                    p,
                    role,
                );
            }
        }
    }
}
