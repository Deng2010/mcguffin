import { createContext } from "react";

export interface ConfigData {
  server: { site_url: string; port: number };
  admin: { password: string; display_name: string };
  site: {
    name: string;
    title?: string | null;
    difficulty_order: string[];
    timezone?: string;
  };
  oauth: { cp_client_id: string; cp_client_secret: string };
  backup: {
    interval_minutes: number;
    retention_count: number;
    backup_directory?: string | null;
  };
  difficulty: Record<string, { label: string; color: string }>;
  discussion_tags?: Record<string, { color: string; description: string }>;
  discussion_emojis?: Record<string, { char: string }>;
}

export interface DifficultyEntry {
  name: string;
  label: string;
  color: string;
}

export type TabId =
  | "server"
  | "admin"
  | "site"
  | "oauth"
  | "difficulty"
  | "backup"
  | "discussions"
  | "groups";

export const inputClass =
  "w-full px-4 py-2 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 focus:outline-none focus:border-gray-500 text-sm";

export interface ConfigCtx {
  siteUrl: string;
  setSiteUrl: (v: string) => void;
  port: string;
  setPort: (v: string) => void;
  adminPassword: string;
  setAdminPassword: (v: string) => void;
  displayName: string;
  setDisplayName: (v: string) => void;
  siteName: string;
  setSiteName: (v: string) => void;
  siteTitle: string;
  setSiteTitle: (v: string) => void;
  siteTimezone: string;
  setSiteTimezone: (v: string) => void;
  cpClientId: string;
  setCpClientId: (v: string) => void;
  cpClientSecret: string;
  setCpClientSecret: (v: string) => void;
  difficulties: DifficultyEntry[];
  difficultyOrder: string[];
  newDiffName: string;
  setNewDiffName: (v: string) => void;
  newDiffLabel: string;
  setNewDiffLabel: (v: string) => void;
  newDiffColor: string;
  setNewDiffColor: (v: string) => void;
  updateDiff: (
    idx: number,
    field: keyof DifficultyEntry,
    value: string,
  ) => void;
  moveDiff: (idx: number, direction: -1 | 1) => void;
  removeDiff: (idx: number) => void;
  addDiff: () => void;
  backupInterval: number;
  setBackupInterval: (v: number) => void;
  backupRetention: number;
  setBackupRetention: (v: number) => void;
  backupDirectory: string;
  setBackupDirectory: (v: string) => void;
  discussionTags: Record<string, { color: string; description: string }>;
  setDiscussionTags: (
    v: Record<string, { color: string; description: string }>,
  ) => void;
  discussionEmojis: Record<string, { char: string }>;
  setDiscussionEmojis: (v: Record<string, { char: string }>) => void;
}

export const ConfigCtx = createContext<ConfigCtx>(null!);

export const PERM_LABELS: Record<string, string> = {
  view_showcase: "浏览展示",
  apply_join: "申请加入",
  view_team: "查看团队",
  manage_team: "审批入队",
  manage_members: "管理成员",
  submit_problem: "投稿题目",
  view_pending_problems: "浏览所有待审核题目",
  view_approved_problems: "浏览已通过题目",
  view_public_problems: "浏览公开题目",
  approve_all_problems: "审核所有题目",
  manage_all_contests: "管理所有比赛",
  access_admin: "进入后台",
  edit_showcase: "编辑展示",
  view_all_posts: "浏览所有帖子",
  manage_tags: "管理标签",
  manage_notifications: "发送通知",
  manage_backups: "备份恢复",
  view_stats: "查看统计",
  manage_posts: "管理所有帖子",
};

export interface MemberGroup {
  id: string;
  name: string;
  permissions: string[];
}

export interface GroupUser {
  id: string;
  display_name: string;
  username: string;
}
