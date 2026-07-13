import { useState, useEffect } from "react";
import { apiFetch } from "../../../services/api";
import type {
  MemberGroup,
  GroupUser,
} from "../config-context";
import { PERM_LABELS } from "../config-context";

export default function GroupsSection() {
  const [groups, setGroups] = useState<MemberGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [msg, setMsg] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");
  const [newName, setNewName] = useState("");
  const [allUsers, setAllUsers] = useState<GroupUser[]>([]);
  const [selectedUserIds, setSelectedUserIds] = useState<Set<string>>(
    new Set(),
  );
  const [initialGroupUserIds, setInitialGroupUserIds] = useState<Set<string>>(
    new Set(),
  );
  const [savingMembers, setSavingMembers] = useState(false);

  const loadGroups = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<MemberGroup[]>("/admin/groups");
      setGroups(Array.isArray(res) ? res : []);
    } catch (err) {
      setMsg(`加载失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadGroups();
  }, []);

  const loadUsers = async () => {
    try {
      const res = await apiFetch<
        { success?: boolean; users?: GroupUser[] } | GroupUser[]
      >("/admin/users");
      const users = Array.isArray(res) ? res : (res as any).users ?? [];
      setAllUsers(users as GroupUser[]);
    } catch (err) {
      setMsg(`加载用户失败: ${err}`);
    }
  };

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) return;
    try {
      const res = await apiFetch<{
        success: boolean;
        message: string;
        id?: string;
      }>("/admin/groups", {
        method: "POST",
        body: JSON.stringify({ name, permissions: [] }),
      });
      if (res.success) {
        setNewName("");
        loadGroups();
      } else {
        setMsg(res.message);
      }
    } catch (err) {
      setMsg(`创建失败: ${err}`);
    }
  };

  const startEdit = (g: MemberGroup) => {
    setEditingId(g.id);
    setEditName(g.name);
    setSavingMembers(false);
    // Load users and pre-select those in this group
    loadUsers().then(() => {
      apiFetch<{ success?: boolean; users?: any[] } | any[]>("/admin/users")
        .then((res) => {
          const users = Array.isArray(res)
            ? res
            : (res as any).users ?? [];
          const userIds = new Set<string>();
          for (const u of users) {
            if (u.group_ids?.includes(g.id)) userIds.add(u.id);
          }
          setSelectedUserIds(userIds);
          setInitialGroupUserIds(new Set(userIds));
        });
    });
  };

  const toggleUser = (userId: string) => {
    setSelectedUserIds((prev) => {
      const next = new Set(prev);
      if (next.has(userId)) next.delete(userId);
      else next.add(userId);
      return next;
    });
  };

  const handleSave = async () => {
    if (!editingId) return;
    const name = editName.trim();
    if (!name) return;

    // Save group name
    try {
      const currentGroup = groups.find((g) => g.id === editingId);
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/admin/groups/${editingId}`,
        {
          method: "PUT",
          body: JSON.stringify({
            name,
            permissions: currentGroup?.permissions ?? [],
          }),
        },
      );
      if (!res.success) {
        setMsg(res.message);
        return;
      }
    } catch (err) {
      setMsg(`保存组名失败: ${err}`);
      return;
    }

    // Update user-group memberships — only touch users whose membership changed
    setSavingMembers(true);
    let hasError = false;
    const groupId = editingId;
    for (const user of allUsers) {
      const wasInGroup = initialGroupUserIds.has(user.id);
      const nowInGroup = selectedUserIds.has(user.id);
      if (wasInGroup === nowInGroup) continue; // no change
      const newGroups = nowInGroup
        ? [
            ...((allUsers.find((u) => u.id === user.id) as any)?.group_ids ??
              []),
            groupId,
          ]
        : (
            (allUsers.find((u) => u.id === user.id) as any)?.group_ids ?? []
          ).filter((gid: string) => gid !== groupId);
      try {
        const r = await apiFetch<{ success: boolean }>(
          `/admin/users/${user.id}/groups`,
          {
            method: "PUT",
            body: JSON.stringify({ group_ids: newGroups }),
          },
        );
        if (!r.success) {
          hasError = true;
        }
      } catch {
        hasError = true;
      }
    }
    setSavingMembers(false);
    setEditingId(null);
    loadGroups();
    if (hasError) setMsg("部分成员更新失败，请刷新重试");
  };

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`确定要删除成员组「${name}」吗？`)) return;
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/admin/groups/${id}`,
        { method: "DELETE" },
      );
      if (res.success) {
        if (editingId === id) setEditingId(null);
        loadGroups();
      } else {
        setMsg(res.message);
      }
    } catch (err) {
      setMsg(`删除失败: ${err}`);
    }
  };

  if (loading)
    return (
      <div className="text-center py-8 text-gray-400 dark:text-gray-500">
        加载成员组...
      </div>
    );

  return (
    <div>
      {msg && (
        <div
          className={`mb-4 p-3 text-sm border ${
            msg.includes("失败")
              ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300"
              : "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"
          }`}
        >
          {msg}
          <button
            onClick={() => setMsg("")}
            className="ml-3 text-xs underline"
          >
            关闭
          </button>
        </div>
      )}

      {/* Create */}
      <div className="flex items-center gap-2 mb-6 p-3 bg-blue-50 dark:bg-blue-900/30 border border-dashed border-blue-300 dark:border-blue-800">
        <input
          type="text"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          className="flex-1 px-3 py-1.5 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm focus:outline-none"
          placeholder="新成员组名称"
          onKeyDown={(e) => e.key === "Enter" && handleCreate()}
        />
        <button
          onClick={handleCreate}
          className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600"
        >
          创建
        </button>
      </div>

      {/* Group list */}
      {groups.length === 0 && (
        <p className="text-center py-8 text-gray-400 dark:text-gray-500">
          暂无成员组
        </p>
      )}
      <div className="space-y-4">
        {groups.map((g) => (
          <div
            key={g.id}
            className="border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 shadow"
          >
            {editingId === g.id ? (
              <div className="p-4">
                <div className="flex items-center gap-2 mb-3">
                  <label className="text-sm text-gray-600 dark:text-gray-400 shrink-0">
                    组名
                  </label>
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    className="flex-1 px-3 py-1.5 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-sm"
                  />
                  <button
                    onClick={handleSave}
                    disabled={savingMembers}
                    className="px-3 py-1.5 bg-green-600 text-white text-sm hover:bg-green-700 disabled:opacity-50"
                  >
                    {savingMembers ? "保存中..." : "保存"}
                  </button>
                  <button
                    onClick={() => setEditingId(null)}
                    className="px-3 py-1.5 border border-gray-300 dark:border-gray-700 text-sm hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    取消
                  </button>
                </div>
                {/* Member list */}
                <div className="border-t border-gray-200 dark:border-gray-700 pt-3 mt-3">
                  <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">
                    包含成员
                  </p>
                  {allUsers.length === 0 ? (
                    <p className="text-xs text-gray-400 dark:text-gray-500">
                      加载用户中...
                    </p>
                  ) : (
                    <div className="max-h-48 overflow-y-auto space-y-1">
                      {allUsers.map((u) => (
                        <label
                          key={u.id}
                          className="flex items-center gap-2 px-2 py-1 hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer rounded"
                        >
                          <input
                            type="checkbox"
                            checked={selectedUserIds.has(u.id)}
                            onChange={() => toggleUser(u.id)}
                            className="accent-gray-800 dark:accent-gray-300"
                          />
                          <span className="text-sm text-gray-700 dark:text-gray-300">
                            {u.display_name}
                          </span>
                          <span className="text-xs text-gray-400 dark:text-gray-500">
                            @{u.username}
                          </span>
                        </label>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <div>
                <div className="flex items-center justify-between px-4 py-3 border-b border-gray-100 dark:border-gray-800">
                  <h3 className="text-sm font-semibold text-gray-800 dark:text-gray-100">
                    {g.name}
                  </h3>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => startEdit(g)}
                      className="text-xs px-2 py-1 border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                    >
                      编辑
                    </button>
                    <button
                      onClick={() => handleDelete(g.id, g.name)}
                      className="text-xs px-2 py-1 border border-red-300 dark:border-red-800 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                    >
                      删除
                    </button>
                  </div>
                </div>
                <div className="px-4 py-2">
                  {g.permissions.length === 0 ? (
                    <span className="text-xs text-gray-400">无额外权限</span>
                  ) : (
                    <div className="flex flex-wrap gap-1">
                      {g.permissions.map((p) => (
                        <span
                          key={p}
                          className="px-2 py-0.5 text-xs bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300"
                        >
                          {PERM_LABELS[p] ?? p}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
