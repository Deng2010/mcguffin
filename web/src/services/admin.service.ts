import type { AdminUser, MemberGroup } from "../types";
import { apiFetch } from "./api";

export interface ConfigValue {
  server?: Record<string, any>;
  site?: Record<string, any>;
  admin?: Record<string, any>;
  oauth?: Record<string, any>;
  backup?: Record<string, any>;
  difficulty?: Record<string, any>;
  permissions?: Record<string, string[]>;
  discussion_tags?: Record<string, any>;
  discussion_emojis?: Record<string, any>;
  [key: string]: any;
}

export interface Group {
  id: string;
  name: string;
  permissions: string[];
  created_at?: string;
}

export interface BackupItem {
  name: string;
  size: number;
  created_at: string;
}

export async function getConfig(): Promise<ConfigValue> {
  return apiFetch<ConfigValue>("/admin/config");
}

/** 操作结果（success/message 风格响应） */
export interface ActionResult {
  success: boolean;
  message?: string;
  [key: string]: any;
}

export async function updateConfig(body: ConfigValue): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/config", {
    method: "PUT",
    body: JSON.stringify(body),
  });
}

export async function restartServer(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/restart", { method: "POST" });
}

export async function exportConfig(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/export/config");
}

export async function getAdminUsers(): Promise<AdminUser[]> {
  return apiFetch<AdminUser[]>("/admin/users");
}

export async function getGroups(): Promise<MemberGroup[]> {
  return apiFetch<MemberGroup[]>("/admin/groups");
}

export async function changeUserRole(
  userId: string,
  role: "admin" | "member" | "guest",
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/users/${userId}/role`, {
    method: "POST",
    body: JSON.stringify({ role }),
  });
}

export async function removeUser(userId: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/users/${userId}/remove`, {
    method: "POST",
  });
}

export async function updateUserPermissions(
  userId: string,
  permissions: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/users/${userId}/permissions`, {
    method: "PUT",
    body: JSON.stringify({ permissions }),
  });
}

export async function updateUserGroups(
  userId: string,
  groupIds: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/users/${userId}/groups`, {
    method: "PUT",
    body: JSON.stringify({ group_ids: groupIds }),
  });
}

export async function createGroup(
  body: Omit<Group, "id">,
): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/groups", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function updateGroup(
  id: string,
  body: Omit<Group, "id">,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/groups/${id}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
}

export async function deleteGroup(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/groups/${id}`, {
    method: "DELETE",
  });
}

export async function getBackups(): Promise<BackupItem[]> {
  return apiFetch<BackupItem[]>("/admin/backups");
}

export async function createBackup(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/backup", { method: "POST" });
}

export async function restoreBackup(name: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(
    `/admin/backup/restore/${encodeURIComponent(name)}`,
    {
      method: "POST",
    },
  );
}

export async function deleteBackup(name: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/backup/${encodeURIComponent(name)}`, {
    method: "DELETE",
  });
}

export async function downloadBackup(name: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(
    `/admin/backup/download/${encodeURIComponent(name)}`,
  );
}

export async function restoreFromUpload(
  content: string,
  filename: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/backup/restore-upload", {
    method: "POST",
    body: JSON.stringify({ content, filename }),
  });
}

export async function exportData(type: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/export/${encodeURIComponent(type)}`);
}

export async function exportDatabase(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/export/db");
}

export async function importData(content: string): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/import/data", {
    method: "POST",
    body: JSON.stringify({ content }),
  });
}

export async function importConfig(content: string): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/import/config", {
    method: "POST",
    body: JSON.stringify({ content }),
  });
}

export async function resetPermissions(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/permissions/reset", {
    method: "POST",
  });
}

export async function getAclResources(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/acl/resources");
}

export async function resetAcl(): Promise<ActionResult> {
  return apiFetch<ActionResult>("/admin/acl/reset", {
    method: "POST",
  });
}

export async function updateResourceAcl(
  type: string,
  id: string,
  visibleTo: string[],
  editableBy: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/acl/${type}/${id}`, {
    method: "PUT",
    body: JSON.stringify({ visible_to: visibleTo, editable_by: editableBy }),
  });
}

export async function updateContestAcl(
  id: string,
  visibleTo: string[],
  editableBy: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/acl/contest/${id}`, {
    method: "PUT",
    body: JSON.stringify({ visible_to: visibleTo, editable_by: editableBy }),
  });
}

export async function updateProblemAcl(
  id: string,
  editableBy: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/acl/problem/${id}`, {
    method: "PUT",
    body: JSON.stringify({ editable_by: editableBy }),
  });
}

export async function updatePostAcl(
  id: string,
  visibleTo: string[],
  editableBy: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/admin/acl/post/${id}`, {
    method: "PUT",
    body: JSON.stringify({ visible_to: visibleTo, editable_by: editableBy }),
  });
}
