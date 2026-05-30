# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

McGuffin is a team collaboration tool for algorithm contest problem-setters. React SPA (TypeScript + Vite + Tailwind) served by a Rust/Axum backend, with CP OAuth authentication and in-memory state persisted to JSON.

## Build & Run

```bash
# Backend
cd server
cargo check              # type-check only (fast)
cargo test               # run all tests
cargo build --release    # production binary → target/release/mcguffin-server
cargo run                # dev server on :3000
cargo run --bin mcguffin -- init   # CLI: init config + backup dir

# Frontend
cd web
bun install              # install dependencies
bun run dev              # dev server on :5173 (proxies /api to :3000)
bun run build            # production build → dist/ (runs tsc --noEmit first)
bun run test             # vitest tests
```

The production server serves the frontend SPA from `../web/dist/` — run `bun run build` before deploying.

## Architecture

### Backend (`server/src/`)

| File               | Purpose                                                                                                                                                                                                            |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `main.rs`          | Axum server bootstrap, all route definitions, CORS, static file serving, compression                                                                                                                               |
| `lib.rs`           | Module declarations + wildcard re-exports                                                                                                                                                                          |
| `types.rs`         | All data structs: `User`, `TeamMember`, `Problem`, `Contest`, `Post`, `Suggestion`, `Announcement`, `Notification`, `MemberGroup`, plus all config/payload types, permission constants, role→permissions mapping   |
| `state.rs`         | `AppState` with all `Arc<RwLock<HashMap<...>>>` fields, `save()` (atomic tmp+rename), `reload()`, config loading from `/usr/share/mcguffin/config.toml`, default seed data                                         |
| `utils.rs`         | Auth helpers: `resolve_user()`, `is_admin()`, `is_superadmin()`, `is_team_member()`, `get_token_from_headers()`, audit logging via `AppState::log_audit()`                                                         |
| `auth.rs`          | OAuth authorize/callback (PKCE), admin password login + session creation, token refresh                                                                                                                            |
| `user.rs`          | Current user (`GET /api/user/me`), profile update, verify token, logout, public profile, username availability check                                                                                               |
| `team.rs`          | Member list, join requests (list/apply/review), role changes, member removal (superadmin gates)                                                                                                                    |
| `problems.rs`      | Full problem CRUD, review workflow (approve/publish/reject), claiming, verifier solutions, visibility ACL, contest assignment, pending problems for admin                                                          |
| `contests.rs`      | Contest CRUD, status toggling (upcoming/ongoing/finished), problem ordering                                                                                                                                        |
| `info.rs`          | Public site info, description update, difficulty config                                                                                                                                                            |
| `admin.rs`         | Config read/write (via `toml_edit`), restart, backup/restore, data export (JSON/TOML), audit log, user management (role/remove/groups/permissions), group CRUD, problem ACL, unified resource ACL, showcase config |
| `discussions.rs`   | Unified post system: post CRUD, replies, reactions, tags, emojis, truncation + orphan cleanup at startup                                                                                                           |
| `community.rs`     | Public community feed endpoint (`GET /api/community/posts`)                                                                                                                                                        |
| `suggestions.rs`   | Suggestion CRUD with status workflow (open→in_progress→resolved→closed)                                                                                                                                            |
| `announcements.rs` | Announcement CRUD, visibility filtering (public vs team-only)                                                                                                                                                      |
| `notifications.rs` | Notification CRUD, unread count, mark-read (single + all)                                                                                                                                                          |
| `pages.rs`         | Server-rendered login/portfolio pages (legacy SSR)                                                                                                                                                                 |
| `bin/mcguffin.rs`  | CLI tool: `init`, `config show/set`, `backup create/list/restore/delete`, `service start/stop/restart`, `status`                                                                                                   |

### Frontend (`web/src/`)

| File                      | Purpose                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `App.tsx`                 | HashRouter, all routes with nested layouts (MainLayout, AdminGuardLayout), Footer, context providers                                                                                       |
| `main.tsx`                | Entry point (ReactDOM.createRoot)                                                                                                                                                          |
| `index.css`               | Tailwind directives + custom styles (Prism theme, scrollbar, callouts)                                                                                                                     |
| `types.ts`                | TypeScript interfaces for all data types, `Permission` union type, `defaultRolePermissions` fallback map                                                                                   |
| `api.ts`                  | `apiFetch<T>()` wrapper — prepends `/api`, injects Bearer token from localStorage, plus notification-specific helpers                                                                      |
| `AuthContext.tsx`         | Auth state (user/isAuthenticated/loading), `hasPermission()` with backend role→permissions map, `login()` (OAuth redirect), `accountLogin()` (admin password), `logout()`, `refreshUser()` |
| `SiteContext.tsx`         | Site info (name, title, version, description, difficulty_order, showcase IDs), `updateDescription()`, `document.title` sync                                                                |
| `DarkModeContext.tsx`     | Dark mode toggle, localStorage persistence, system preference fallback, `<html class="dark">` sync                                                                                         |
| `NotificationContext.tsx` | Notification polling (30s), unread count, `markRead()`, `markAllRead()`                                                                                                                    |

### Components (`web/src/components/`)

| File                       | Purpose                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------ |
| `Navbar.tsx`               | Navigation links with permission-gated visibility, dark mode toggle, notification dropdown |
| `ProtectedRoute.tsx`       | Route guard — checks `hasPermission()`, redirects to `/login` if unauthorized              |
| `AdminLayout.tsx`          | Admin section sidebar + content outlet                                                     |
| `MarkdownRenderer.tsx`     | react-markdown with KaTeX (math), Prism (code highlighting), custom callout blocks         |
| `MarkdownEditor.tsx`       | Textarea-based Markdown editor with live preview toggle                                    |
| `MentionDropdown.tsx`      | `@username` mention autocomplete dropdown                                                  |
| `NotificationDropdown.tsx` | Notification bell popover with unread indicator, mark-read actions                         |
| `ReactionRow.tsx`          | Emoji reaction row (display + click-to-toggle)                                             |
| `ReplyCard.tsx`            | Discussion reply card with author info, reactions, delete action                           |

### Pages (`web/src/pages/`) — 24 page components

| Page                     | Route                            | Access                                  |
| ------------------------ | -------------------------------- | --------------------------------------- |
| `ShowcasePage`           | `/`                              | Public                                  |
| `LoginPage`              | `/login`                         | Public                                  |
| `AuthCallbackPage`       | `/auth/callback`                 | Public (OAuth callback)                 |
| `ProblemsPage`           | `/problems`                      | Public                                  |
| `ProblemDetailPage`      | `/problems/:id`                  | `view_problems`                         |
| `TeamPage`               | `/team`                          | `view_team`                             |
| `ApplyPage`              | `/apply`                         | `apply_join`                            |
| `ContestManagePage`      | `/contests`                      | Public                                  |
| `ProfilePage`            | `/profile`, `/profile/:username` | `view_showcase` (own) / Public (others) |
| `DiscussionsPage`        | `/discussions`                   | `view_discussions`                      |
| `DiscussionDetailPage`   | `/discussions/:id`               | `view_discussions`                      |
| `SuggestionsPage`        | `/suggestions`                   | `view_discussions`                      |
| `SuggestionDetailPage`   | `/suggestions/:id`               | `view_discussions`                      |
| `AnnouncementsPage`      | `/announcements`                 | Public                                  |
| `AnnouncementDetailPage` | `/announcements/:id`             | Public                                  |
| `CommunityPage`          | `/community`                     | Public                                  |
| `PostDetailPage`         | `/post/:id`                      | Public                                  |
| `NotFoundPage`           | `*`                              | Public                                  |
| `AdminConfigPage`        | `/admin/config`                  | `manage_site`                           |
| `AdminDiscussionsPage`   | `/admin/discussions`             | `manage_site`                           |
| `AdminUsersPage`         | `/admin/users`                   | `manage_site`                           |
| `AdminGroupsPage`        | `/admin/groups`                  | `manage_site`                           |
| `AdminRolesPage`         | `/admin/roles`                   | `manage_site`                           |
| `AdminBackupsPage`       | `/admin/backups`                 | `manage_site`                           |

### Hooks & Utils (`web/src/hooks/`, `web/src/utils/`)

| File                        | Purpose                                                                          |
| --------------------------- | -------------------------------------------------------------------------------- |
| `hooks/useDifficulties.tsx` | Dynamic difficulty levels fetched from API, wrapped in a React context           |
| `hooks/useMention.ts`       | `@mention` autocomplete state management (filtering, selection, cursor position) |
| `utils/groups.ts`           | Group utility functions                                                          |
| `utils/time.ts`             | Time formatting helpers                                                          |

### Tests (`web/src/test/`)

| File            | Purpose                        |
| --------------- | ------------------------------ |
| `api.test.ts`   | API client tests               |
| `types.test.ts` | Type tests                     |
| `setup.ts`      | Vitest setup (jsdom, matchers) |

### CI/CD (`.github/workflows/`)

| File          | Purpose                                                                                                    |
| ------------- | ---------------------------------------------------------------------------------------------------------- |
| `test.yml`    | Runs frontend + backend tests on push/PR across Linux, macOS, Windows                                      |
| `release.yml` | Nightly build on `main` push — builds server binaries + frontend dist for 3 platforms, uploads as artifact |

## Key Design Patterns

**Permission system**: Two-tier — backend computes `effective_role` considering `role` + `team_status` + `group_ids` + `user_permissions`; frontend fetches role→permissions mapping from `GET /api/auth/permissions`. `ProtectedRoute` checks at route level, `hasPermission()` for conditional UI. Superadmin (hardcoded `user_id == "admin"`) has wildcard `*` all permissions.

**Role hierarchy**: `superadmin` (id="admin", immune to demotion/removal) > `admin` > `member` > `guest` > `pending`. `effective_role` overrides: if user has `member` role with `team_status == "joined"` → effective `member`; else `pending`/`none` maps to `guest`.

**API pattern**: Every handler takes `State<AppState>` + `HeaderMap`, calls `resolve_user()` for auth, checks permissions inline, returns `Json(serde_json::json!(...))`. No middleware auth layer — each handler does its own checks.

**Persistence**: `AppState.save()` serializes all HashMaps to JSON, writes to `.tmp` file, atomically renames over `mcguffin_data.json`. All mutable state wrapped in `Arc<RwLock<HashMap<...>>>`. `reload()` reads saved JSON back and populates the HashMaps.

**Config**: `/usr/share/mcguffin/config.toml` (TOML format) loaded at startup. Fields: `server.site_url`/`port`/`data_file`, `admin.password`/`display_name`, `site.name`/`title`/`difficulty_order`, `oauth.cp_client_id`/`cp_client_secret`, `difficulty.levels[]`, `discussion_tags[]`, `discussion_emojis[]`, `permissions` (role→list), `permission_groups[]`. Editable at runtime via admin UI (superadmin only). Some changes (difficulty, description, showcase) take effect immediately; others (oauth, password, port) require restart.

**CLI tool** (`cargo run --bin mcguffin`): Manages configuration and backups without running the server. Subcommands: `init` (create default config + backup dir), `config show/set`, `backup create/list/restore/delete`, `service start/stop/restart/status`.

**Superadmin protections**: The hardcoded admin (`ADMIN_USER_ID = "admin"` in `state.rs`) cannot be demoted or removed. Only superadmin can promote/demote/remove other admins. Checked in `team.rs` functions `change_member_role()` and `remove_member()`.

**Unified post system** (`discussions.rs`): Single `Post` struct replaces separate discussion/suggestion/announcement tables. Tags, emojis, reactions, replies, and visibility ACL all stored on the post. Legacy `/api/discussions/*` routes are aliased to the same handlers.
