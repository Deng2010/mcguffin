// ============== User & Auth Types ==============

export interface User {
  id: string
  username: string
  display_name: string
  avatar_url: string | null
  email: string | null
  role: 'superadmin' | 'admin' | 'member' | 'guest' | 'pending'
  team_status: 'none' | 'pending' | 'joined'
  created_at: string
  bio?: string
}

export type Permission =
  /** View showcase (public) */
  | 'view_showcase'
  /** Apply to join team */
  | 'apply_join'
  /** View team member list */
  | 'view_team'
  /** Manage team (approve/reject applications, change roles) */
  | 'manage_team'
  /** Submit problems */
  | 'submit_problem'
  /** View problem list and details (filtered by permissions) */
  | 'view_problems'
  /** Review/approve problems */
  | 'approve_problem'
  /** Manage contests (create/edit/delete/toggle visibility) */
  | 'edit_contests'
  /** Edit site info (team showcase description) */
  | 'manage_site'
  /** View and submit suggestions */
  | 'view_suggestions'
  /** Manage announcements */
  | 'manage_announcements'

export const rolePermissions: Record<User['role'], Permission[]> = {
  superadmin: ['view_showcase', 'view_team', 'manage_team', 'submit_problem', 'view_problems', 'approve_problem', 'edit_contests', 'manage_site', 'view_suggestions', 'manage_announcements'],
  admin:   ['view_showcase', 'view_team', 'manage_team', 'submit_problem', 'view_problems', 'approve_problem', 'edit_contests', 'manage_site', 'view_suggestions', 'manage_announcements'],
  member:  ['view_showcase', 'view_team', 'submit_problem', 'view_problems', 'view_suggestions'],
  guest:   ['view_showcase', 'apply_join'],
  pending: ['view_showcase', 'apply_join'],
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
