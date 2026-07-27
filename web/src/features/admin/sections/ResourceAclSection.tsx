import { useState, useEffect } from "react";
import { apiFetch } from "../../../services/api";

interface AclResource {
  id: string;
  title: string;
  status: string;
  team_only?: boolean;
  visible_to: string[];
  editable_by: string[];
}

interface AclUser {
  id: string;
  username: string;
  display_name: string;
  role: string;
}

interface AclData {
  problems: AclResource[];
  contests: AclResource[];
  posts: AclResource[];
  users: AclUser[];
}

const STATUS_LABELS: Record<string, string> = {
  // Problems
  pending: "待审核",
  approved: "已审核",
  published: "已发布",
  rejected: "已拒绝",
  // Contests
  draft: "未公开",
  public: "已公开",
  // Posts
  open: "待处理",
  in_progress: "处理中",
  resolved: "已解决",
  closed: "已关闭",
};

const ROLE_LABELS: Record<string, string> = {
  superadmin: "超级管理员",
  admin: "管理",
  member: "成员",
  guest: "游客",
};

type ResourceType = "problem" | "contest" | "post";

export default function ResourceAclSection() {
  const [data, setData] = useState<AclData | null>(null);
  const [loading, setLoading] = useState(true);
  const [msg, setMsg] = useState("");
  const [expandedTypes, setExpandedTypes] = useState<Set<string>>(
    new Set(["problem", "contest", "post"]),
  );
  const [expandedStatusGroups, setExpandedStatusGroups] = useState<Set<string>>(
    new Set(),
  );
  const [editingAcl, setEditingAcl] = useState<Set<string>>(new Set());
  // Local edits: key = "type:id", value = { visible_to, editable_by }
  const [aclEdits, setAclEdits] = useState<
    Record<string, { visible_to: string[]; editable_by: string[] }>
  >({});
  const [savingId, setSavingId] = useState<string | null>(null);

  const loadData = async () => {
    setLoading(true);
    try {
      const res = await apiFetch<AclData>("/admin/acl/resources");
      setData(res);
    } catch (err) {
      setMsg(`加载失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const toggleType = (t: string) => {
    setExpandedTypes((prev) => {
      const next = new Set(prev);
      if (next.has(t)) next.delete(t);
      else next.add(t);
      return next;
    });
  };

  const toggleStatusGroup = (key: string) => {
    setExpandedStatusGroups((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const startEdit = (type: ResourceType, resource: AclResource) => {
    const key = `${type}:${resource.id}`;
    setEditingAcl((prev) => new Set(prev).add(key));
    setAclEdits((prev) => ({
      ...prev,
      [key]: {
        visible_to: [...resource.visible_to],
        editable_by: [...resource.editable_by],
      },
    }));
  };

  const cancelEdit = (type: ResourceType, id: string) => {
    const key = `${type}:${id}`;
    setEditingAcl((prev) => {
      const next = new Set(prev);
      next.delete(key);
      return next;
    });
    setAclEdits((prev) => {
      const next = { ...prev };
      delete next[key];
      return next;
    });
  };

  const toggleUser = (
    type: ResourceType,
    id: string,
    userId: string,
    field: "visible_to" | "editable_by",
  ) => {
    const key = `${type}:${id}`;
    setAclEdits((prev) => {
      const current = prev[key] || { visible_to: [], editable_by: [] };
      const arr = current[field];
      const nextArr = arr.includes(userId)
        ? arr.filter((u) => u !== userId)
        : [...arr, userId];
      return { ...prev, [key]: { ...current, [field]: nextArr } };
    });
  };

  const handleSave = async (type: ResourceType, id: string) => {
    const key = `${type}:${id}`;
    const edit = aclEdits[key];
    if (!edit) return;
    setSavingId(key);
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/admin/acl/${type}/${id}`,
        {
          method: "PUT",
          body: JSON.stringify({
            visible_to: edit.visible_to,
            editable_by: edit.editable_by,
          }),
        },
      );
      if (res.success) {
        setMsg("保存成功");
        setTimeout(() => setMsg(""), 3000);
        cancelEdit(type, id);
        loadData();
      } else {
        setMsg(res.message || "保存失败");
      }
    } catch (err) {
      setMsg(`保存失败: ${err}`);
    } finally {
      setSavingId(null);
    }
  };

  if (loading)
    return (
      <div className="text-center py-8 text-gray-400 dark:text-gray-500">
        加载资源权限数据...
      </div>
    );
  if (!data)
    return (
      <div className="text-center py-8 text-gray-400 dark:text-gray-500">
        加载失败
      </div>
    );

  const users = data.users || [];
  // Filter out superadmin (id=admin) from ACL options
  const aclUsers = users.filter((u) => u.id !== "admin");

  const groupByStatus = (
    items: AclResource[],
  ): Record<string, AclResource[]> => {
    const groups: Record<string, AclResource[]> = {};
    for (const item of items) {
      const status = item.status || "__no_status__";
      if (!groups[status]) groups[status] = [];
      groups[status].push(item);
    }
    return groups;
  };

  const resourceTypes: {
    type: ResourceType;
    label: string;
    items: AclResource[];
    statusLabels: Record<string, string>;
  }[] = [
    {
      type: "problem",
      label: "题目",
      items: data.problems,
      statusLabels: {
        pending: "待审核",
        approved: "已审核",
        published: "已发布",
        rejected: "已拒绝",
      },
    },
    {
      type: "contest",
      label: "比赛",
      items: data.contests,
      statusLabels: { draft: "未公开", public: "已公开" },
    },
    {
      type: "post",
      label: "讨论/帖子",
      items: data.posts,
      statusLabels: {
        "": "普通帖子",
        open: "待处理",
        in_progress: "处理中",
        resolved: "已解决",
        closed: "已关闭",
      },
    },
  ];

  const getStatusLabel = (
    status: string,
    typeLabels: Record<string, string>,
  ): string => {
    if (status === "" || status === "__no_status__")
      return typeLabels[""] || "其他";
    return STATUS_LABELS[status] || typeLabels[status] || status || "其他";
  };

  return (
    <div>
      {msg && (
        <div
          className={`mb-4 p-3 text-sm border ${msg.includes("失败") ? "bg-red-50 border-red-300 text-red-700 dark:bg-red-900/30 dark:border-red-800 dark:text-red-300" : "bg-green-50 border-green-300 text-green-700 dark:bg-green-900/30 dark:border-green-800 dark:text-green-300"}`}
        >
          {msg}
        </div>
      )}

      <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
        按资源类型和状态分组，可逐资源设置哪些成员可查看（visible_to）和可编辑（editable_by）。超级管理员不受限制。
      </p>

      <div className="space-y-4">
        {resourceTypes.map(({ type, label, items, statusLabels }) => {
          const groups = groupByStatus(items);
          const statusKeys = Object.keys(groups).sort((a, b) => {
            // Sort: put known statuses first in logical order, unknowns last
            const known = Object.keys(statusLabels);
            const ai = known.indexOf(a);
            const bi = known.indexOf(b);
            if (ai >= 0 && bi >= 0) return ai - bi;
            if (ai >= 0) return -1;
            if (bi >= 0) return 1;
            return a.localeCompare(b);
          });

          return (
            <div
              key={type}
              className="border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 shadow"
            >
              <button
                onClick={() => toggleType(type)}
                className="w-full flex items-center gap-2 px-4 py-3 text-sm font-semibold text-gray-800 dark:text-gray-100 hover:bg-gray-50 dark:hover:bg-gray-800/50 text-left"
              >
                <span className="text-xs text-gray-400 w-3 shrink-0">
                  {expandedTypes.has(type) ? "▼" : "▶"}
                </span>
                <span>{label}</span>
                <span className="text-xs text-gray-400 ml-1">
                  ({items.length})
                </span>
              </button>

              {expandedTypes.has(type) && (
                <div className="border-t border-gray-200 dark:border-gray-700 px-4 py-3 space-y-3">
                  {statusKeys.length === 0 && (
                    <p className="text-sm text-gray-400 dark:text-gray-500 py-2">
                      暂无资源
                    </p>
                  )}
                  {statusKeys.map((status) => {
                    const groupItems = groups[status];
                    const statusLabel = getStatusLabel(status, statusLabels);
                    const groupKey = `${type}:${status}`;
                    return (
                      <div key={groupKey}>
                        <button
                          onClick={() => toggleStatusGroup(groupKey)}
                          className="w-full flex items-center gap-2 px-2 py-1.5 text-sm text-gray-600 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800/30 text-left"
                        >
                          <span className="text-xs text-gray-400 w-3 shrink-0">
                            {expandedStatusGroups.has(groupKey) ? "▼" : "▶"}
                          </span>
                          <span className="font-medium">{statusLabel}</span>
                          <span className="text-xs text-gray-400">
                            ({groupItems.length})
                          </span>
                        </button>

                        {expandedStatusGroups.has(groupKey) && (
                          <div className="ml-5 space-y-1.5 mt-1">
                            {groupItems.map((item) => {
                              const aclKey = `${type}:${item.id}`;
                              const isEditing = editingAcl.has(aclKey);
                              const edit = aclEdits[aclKey];
                              const currentV = edit
                                ? edit.visible_to
                                : item.visible_to;
                              const currentE = edit
                                ? edit.editable_by
                                : item.editable_by;

                              return (
                                <div
                                  key={item.id}
                                  className="border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 p-3"
                                >
                                  <div className="flex items-center justify-between mb-2">
                                    <span className="text-sm font-medium text-gray-700 dark:text-gray-200 truncate flex-1 mr-2">
                                      {item.title}
                                      {item.team_only && (
                                        <span className="ml-2 text-xs text-gray-400 dark:text-gray-500">
                                          [内部]
                                        </span>
                                      )}
                                    </span>
                                    <div className="flex items-center gap-1 shrink-0">
                                      {isEditing ? (
                                        <>
                                          <button
                                            onClick={() =>
                                              handleSave(type, item.id)
                                            }
                                            disabled={savingId === aclKey}
                                            className="px-2 py-0.5 text-xs bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:hover:bg-gray-600"
                                          >
                                            {savingId === aclKey
                                              ? "保存中..."
                                              : "保存"}
                                          </button>
                                          <button
                                            onClick={() =>
                                              cancelEdit(type, item.id)
                                            }
                                            className="px-2 py-0.5 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                                          >
                                            取消
                                          </button>
                                        </>
                                      ) : (
                                        <button
                                          onClick={() => startEdit(type, item)}
                                          className="px-2 py-0.5 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                                        >
                                          编辑权限
                                        </button>
                                      )}
                                    </div>
                                  </div>

                                  {/* Show current ACL summary when not editing */}
                                  {!isEditing ? (
                                    <div className="text-xs text-gray-500 dark:text-gray-400 space-y-1">
                                      <div>
                                        <span className="font-medium">
                                          可见:{" "}
                                        </span>
                                        {currentV.length === 0 ? (
                                          <span className="text-gray-400">
                                            全部
                                          </span>
                                        ) : (
                                          currentV.map((uid, i) => {
                                            const u = users.find(
                                              (x) => x.id === uid,
                                            );
                                            return (
                                              <span key={uid}>
                                                {u ? u.display_name : uid}
                                                {i < currentV.length - 1
                                                  ? ", "
                                                  : ""}
                                              </span>
                                            );
                                          })
                                        )}
                                      </div>
                                      <div>
                                        <span className="font-medium">
                                          可编辑:{" "}
                                        </span>
                                        {currentE.length === 0 ? (
                                          <span className="text-gray-400">
                                            默认
                                          </span>
                                        ) : (
                                          currentE.map((uid, i) => {
                                            const u = users.find(
                                              (x) => x.id === uid,
                                            );
                                            return (
                                              <span key={uid}>
                                                {u ? u.display_name : uid}
                                                {i < currentE.length - 1
                                                  ? ", "
                                                  : ""}
                                              </span>
                                            );
                                          })
                                        )}
                                      </div>
                                    </div>
                                  ) : (
                                    /* ACL edit mode */
                                    <div className="space-y-3 mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
                                      {aclUsers.length === 0 ? (
                                        <p className="text-xs text-gray-400">
                                          暂无成员
                                        </p>
                                      ) : (
                                        <>
                                          <div>
                                            <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">
                                              可见成员
                                            </label>
                                            <div className="flex flex-wrap gap-1.5">
                                              {aclUsers.map((u) => (
                                                <label
                                                  key={u.id}
                                                  className="flex items-center gap-1 text-xs cursor-pointer"
                                                >
                                                  <input
                                                    type="checkbox"
                                                    checked={currentV.includes(
                                                      u.id,
                                                    )}
                                                    onChange={() =>
                                                      toggleUser(
                                                        type,
                                                        item.id,
                                                        u.id,
                                                        "visible_to",
                                                      )
                                                    }
                                                    className="accent-blue-600 dark:accent-blue-400"
                                                  />
                                                  {u.display_name}
                                                  <span className="text-gray-400">
                                                    (
                                                    {ROLE_LABELS[u.role] ||
                                                      u.role}
                                                    )
                                                  </span>
                                                </label>
                                              ))}
                                            </div>
                                          </div>
                                          <div>
                                            <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">
                                              可编辑成员
                                            </label>
                                            <div className="flex flex-wrap gap-1.5">
                                              {aclUsers.map((u) => (
                                                <label
                                                  key={u.id}
                                                  className="flex items-center gap-1 text-xs cursor-pointer"
                                                >
                                                  <input
                                                    type="checkbox"
                                                    checked={currentE.includes(
                                                      u.id,
                                                    )}
                                                    onChange={() =>
                                                      toggleUser(
                                                        type,
                                                        item.id,
                                                        u.id,
                                                        "editable_by",
                                                      )
                                                    }
                                                    className="accent-emerald-600 dark:accent-emerald-400"
                                                  />
                                                  {u.display_name}
                                                  <span className="text-gray-400">
                                                    (
                                                    {ROLE_LABELS[u.role] ||
                                                      u.role}
                                                    )
                                                  </span>
                                                </label>
                                              ))}
                                            </div>
                                          </div>
                                        </>
                                      )}
                                    </div>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
