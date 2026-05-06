# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

McGuffin is a team collaboration tool for algorithm contest problem-setters. React SPA frontend (TypeScript + Vite + Tailwind) served by a Rust/Axum backend, with CP OAuth authentication and in-memory state persisted to JSON.

## Build & Run

```bash
# Quick restart (rebuilds both + restarts server)
./restart.sh                   # full rebuild + restart
./restart.sh --backend-only    # skip frontend rebuild
./restart.sh --frontend-only   # skip backend rebuild

# Backend
cd server
cargo check              # type-check only (fast)
cargo test               # run all tests
cargo build --release    # production binary → target/release/mcguffin-server
cargo run                # dev server on :3000

# Frontend
cd web
bun install              # install dependencies
bun run dev              # dev server on :5173
bun run build            # production build → dist/ (runs tsc --noEmit first)
bun run test             # vitest tests
```

The production server serves the frontend SPA from `../web/dist/` — run `bun run build` before deploying backend changes that serve static files.

## Architecture

### Backend (`server/src/`)

| File | Purpose |
|------|---------|
| `main.rs` | Axum server bootstrap, all route definitions, CORS config |
| `lib.rs` | Module declarations + wildcard re-exports |
| `types.rs` | All data structs: `User`, `TeamMember`, `Problem`, `Contest`, `JoinRequest`, `Suggestion`, `Announcement`, plus config/payload types |
| `state.rs` | `AppState` struct (`Arc<RwLock<HashMap<...>>>` fields), `SavedData` persistence format, `save()` (atomic write via tmp+rename), `reload()`, config loading from `/usr/share/mcguffin/config.toml` |
| `utils.rs` | Auth helpers: `resolve_user()`, `is_admin()`, `is_superadmin()`, `is_team_member()`, `get_token_from_headers()` |
| `auth.rs` | OAuth authorize/callback, admin password login, token refresh |
| `user.rs` | Current user, profile update, verify, logout |
| `team.rs` | Member list, join requests, role changes, member removal with superadmin gates |
| `problems.rs` | Full problem CRUD, review workflow (approve/publish/reject), claiming, verifier solutions, visibility |
| `contests.rs` | Contest CRUD, status toggling, problem ordering |
| `info.rs` | Public site info, description update, difficulty config |
| `admin.rs` | Config read/write (TOML editing via `toml_edit`), restart, backup/restore, data export |
| `suggestions.rs` | Suggestion CRUD with status workflow (open→in_progress→resolved→closed) |
| `announcements.rs` | Announcement CRUD, visibility filtering (public vs team-only) |
| `pages.rs` | Server-rendered login/portfolio pages (legacy) |

### Frontend (`web/src/`)

| File | Purpose |
|------|---------|
| `App.tsx` | HashRouter, all routes, Footer |
| `main.tsx` | Entry point |
| `types.ts` | TypeScript interfaces, `Permission` union type, `rolePermissions` map |
| `api.ts` | `apiFetch<T>()` wrapper — prepends `/api`, injects Bearer token from localStorage |
| `AuthContext.tsx` | Auth state, login/logout, `hasPermission()`, `refreshUser()` |
| `SiteContext.tsx` | Site info (name, title, version, description), `document.title` sync |
| `components/Navbar.tsx` | Navigation links with permission-gated visibility |
| `components/ProtectedRoute.tsx` | Route guard checking `hasPermission()` |
| `components/MarkdownRenderer.tsx` | react-markdown with KaTeX, Prism, custom callout blocks |
| `hooks/useDifficulties.tsx` | Dynamic difficulty levels from API |
| `pages/` | 13 page components (ShowcasePage, ProblemsPage, ProblemDetailPage, TeamPage, ApplyPage, ContestManagePage, ProfilePage, LoginPage, AuthCallbackPage, AdminConfigPage, AdminBackupPage, SuggestionsPage, AnnouncementsPage, NotFoundPage) |

### Key Design Patterns

**Permission system**: Two-tier — `rolePermissions` map grants permissions by role, `ProtectedRoute` checks them at route level, components call `hasPermission()` for conditional UI. Superadmin (hardcoded `user_id == "admin"`) has all permissions.

**Role hierarchy**: `superadmin` (id="admin", immune to demotion/removal) > `admin` (manages team/problems but can't touch other admins) > `member` > `guest` > `pending`.

**API pattern**: Every handler takes `State<AppState>` + `HeaderMap`, calls `resolve_user()` for auth, checks role via `is_admin()`/`is_superadmin()`, returns `Json(serde_json::json!(...))`. No middleware auth layer — each handler does its own checks.

**Persistence**: `AppState.save()` serializes all HashMaps to JSON, writes to `.tmp` file, atomically renames over `mcguffin_data.json`. All mutable state wrapped in `Arc<RwLock<HashMap<...>>>`.

**Config**: `/usr/share/mcguffin/config.toml` (TOML format). Read at startup, editable at runtime via admin UI (superadmin only). `SiteConfig.title` → browser tab title; `SiteConfig.name` → navbar site name.

**Superadmin protections**: The hardcoded admin (`ADMIN_USER_ID = "admin"`) cannot be demoted or removed. Only superadmin can promote to admin, demote an admin, or remove an admin. Checked in `team.rs` `change_member_role()` and `remove_member()`.

**Unsaved changes**: `config.toml` changes (oauth, password, server) require restart. Difficulty config and site description take effect immediately.
