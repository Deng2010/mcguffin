// ============== User & Auth Types ==============

export interface User {
  id: string
  username: string
  display_name: string
  avatar_url: string | null
  email: string | null
  role: 'superadmin' | 'admin' | 'member' | 'guest'
  team_status: 'none' | 'pending' | 'joined'
  created_at: string
  bio?: string
  /** Effective role for permission lookup (computed by backend) */
  effective_role?: string
}

export type Permission =
  /** View showcase (public) */
  | 'view_showcase'
  /** Apply to join team */
  | 'apply_join'
  /** View team member list */
  | 'view_team'
  /** Manage team (approve/reject applications) */
  | 'manage_team'
  /** Kick members, change roles */
  | 'manage_members'
  /** Submit problems */
  | 'submit_problem'
  /** View problem list and details (filtered by permissions) */
  | 'view_problems'
  /** Review/approve problems */
  | 'approve_problem'
  /** Manage contests (create/edit/delete/toggle visibility) */
  | 'manage_contests'
  /** Edit site info (team showcase description) */
  | 'manage_site'
  /** Edit site description and showcase selections */
  | 'edit_showcase'
  /** View and participate in discussions (includes suggestions and announcements) */
  | 'view_discussions'
  /** Manage discussions, suggestions, announcements, community posts, and tags */
  | 'manage_discussions'
  /** Manage discussion tags and emojis */
  | 'manage_tags'
  /** Send global notifications */
  | 'manage_notifications'
  /** Backup/restore data */
  | 'manage_backups'
  /** View statistics */
  | 'view_stats'
  /** Manage community posts */
  | 'manage_posts'

/** Default role→permissions mapping (fallback when backend unavailable). */
export const defaultRolePermissions: Record<string, Permission[]> = {
  superadmin: ['view_showcase', 'view_team', 'manage_team', 'manage_members', 'submit_problem', 'view_problems', 'approve_problem', 'manage_contests', 'manage_site', 'edit_showcase', 'view_discussions', 'manage_discussions', 'manage_tags', 'manage_notifications', 'manage_backups', 'view_stats', 'manage_posts'],
  admin:   ['view_showcase', 'view_team', 'manage_team', 'manage_members', 'submit_problem', 'view_problems', 'approve_problem', 'manage_contests', 'manage_site', 'edit_showcase', 'view_discussions', 'manage_posts', 'manage_tags', 'manage_notifications', 'view_stats'],
  member: ['view_showcase', 'view_team', 'submit_problem', 'view_problems', 'view_discussions'],
  guest: ['view_showcase', 'apply_join', 'view_discussions'],
}

// ============== Suggestion Types ==============

export type SuggestionStatus = 'open' | 'in_progress' | 'resolved' | 'closed'

export interface SuggestionReply {
  id: string
  author_id: string
  author_name: string
  content: string
  created_at: string
}

export interface Suggestion {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  status: SuggestionStatus
  replies: SuggestionReply[]
  created_at: string
  updated_at: string
}

// ============== Announcement Types ==============

export interface Announcement {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  pinned: boolean
  public: boolean
  created_at: string
  updated_at: string
}

// ============== Team Types ==============

export interface TeamMember {
  id: string
  user_id: string
  name: string
  avatar: string
  role: 'superadmin' | 'admin' | 'member'
  joined_at: string
}

export interface JoinRequest {
  id: string
  user_id: string
  user_name: string
  user_email: string
  reason: string
  status: 'pending' | 'approved' | 'rejected'
  created_at: string
}

// ============== Problem Types ==============

export type Difficulty = 'Easy' | 'Medium' | 'Hard'
export type ProblemStatus = 'pending' | 'approved' | 'published'

export interface Problem {
  id: string
  title: string
  author_id: string
  author_name: string
  contest: string
  difficulty: Difficulty
  content: string
  status: ProblemStatus
  created_at: string
  public_at: string | null
}

export interface ProblemListItem {
  id: string
  title: string
  author_id: string
  author_name: string
  contest: string
  difficulty: Difficulty
  status: ProblemStatus
  created_at: string
  public_at: string | null
  claimed_by: string | null
  has_verifier_solution: boolean
  link?: string | null
  remark?: string | null
}

export interface ProblemDetail {
  id: string
  title: string
  author_id: string
  author_name: string
  contest: string
  contest_id?: string | null
  difficulty: Difficulty
  content?: string
  solution?: string
  remark?: string
  status: ProblemStatus
  created_at: string
  public_at: string | null
  claimed_by: string | null
  has_verifier_solution: boolean
  can_submit_verifier_solution?: boolean
  verifier_solution?: string
}

export interface AdminPendingProblem {
  link?: string | null
  id: string
  title: string
  author_name: string
  contest: string
  difficulty: Difficulty
  content: string
  solution: string | null
  status: ProblemStatus
  created_at: string
  visible_to: string[]
  claimed_by: string | null
  has_verifier_solution: boolean
  remark?: string | null
}

export interface SubmitProblemPayload {
  title: string
  contest: string
  remark?: string
  contest_id?: string
  difficulty: Difficulty
  content: string
  solution?: string
}

// ============== Notification Types ==============

export interface Notification {
  id: string
  user_id: string
  title: string
  body: string
  read: boolean
  created_at: string
  link: string | null
}

export interface NotificationResponse {
  notifications: Notification[]
  unread_count: number
}

export interface ApiResponse {
  success: boolean
  message: string
  data?: unknown
}

// ============== Discussion Types ==============

export interface DiscussionTag {
  id: string
  name: string
  color: string
  admin_only?: boolean
}

export interface DiscussionEmoji {
  id: string
  char: string
  name: string
}

export interface DiscussionReply {
  id: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  content: string
  reactions: Record<string, string[]>
  parent_id: string | null
  reply_to: string | null
  created_at: string
}

/** 讨论列表项 */
export interface Discussion {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  tags: DiscussionTag[]
  reactions: Record<string, string[]>
  emoji: string
  reply_count: number
  created_at: string
  updated_at: string
  pinned: boolean
  team_only: boolean
}

/** 讨论详情（含回复） */
export interface DiscussionDetail {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  tags: DiscussionTag[]
  emoji: string
  reactions: Record<string, string[]>
  replies: DiscussionReply[]
  created_at: string
  updated_at: string
  pinned: boolean
  team_only: boolean
}

// ============== Unified Post Types ==============

export interface PostDetail {
  id: string
  title: string
  content: string
  author_id: string
  author_name: string
  author_avatar_url: string | null
  tags: DiscussionTag[]
  emoji: string | null
  reactions: Record<string, string[]>
  replies: DiscussionReply[]
  created_at: string
  updated_at: string
  pinned: boolean
  team_only: boolean
  status: string
  visible_to?: string[]
  editable_by?: string[]
}