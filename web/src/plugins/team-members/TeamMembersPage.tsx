import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "../../stores/authStore";
import { apiFetch } from "../../services/api";
import { usePluginUserMe, usePluginTeamMembers } from "../sdk";
import { useToast } from "../../errors/ToastContext";
import { errorMessage } from "../../errors/normalize";

// ── Types ──

interface JoinRequest {
  id: string;
  user_id: string;
  user_name: string;
  user_email: string;
  reason: string;
  status: string;
  created_at: string;
}

type TabId = "all" | "admin" | "member";

// ── Constants ──

const PLUGIN_ID = "team-members";

const ROLE_LABEL: Record<string, string> = {
  superadmin: "超级管理员",
  admin: "管理员",
  member: "成员",
};

// ── Component ──

export default function TeamMembersPage() {
  const navigate = useNavigate();
  const toast = useToast();
  const { user, hasPermission } = useAuthStore();

  // SDK hooks — 通过后端插件 API 获取数据（自动鉴权）
  const { user: me } = usePluginUserMe(PLUGIN_ID);
  const {
    members,
    loading: membersLoading,
    error: membersError,
    refresh,
  } = usePluginTeamMembers(PLUGIN_ID);

  const [requests, setRequests] = useState<JoinRequest[]>([]);
  const [activeTab, setActiveTab] = useState<TabId>("all");
  const canManage = hasPermission("manage_team");

  // Load join requests (not available via SDK, use direct API)
  useEffect(() => {
    if (!canManage) return;
    apiFetch<JoinRequest[]>("/team/requests")
      .then(setRequests)
      .catch(() => {});
  }, [canManage]);

  // ── Filtering ──

  const filteredMembers = members.filter((m) => {
    if (activeTab === "admin")
      return m.role === "admin" || m.role === "superadmin";
    if (activeTab === "member") return m.role === "member";
    return true;
  });

  const counts = {
    admin: members.filter(
      (m) => m.role === "admin" || m.role === "superadmin",
    ).length,
    member: members.filter((m) => m.role === "member").length,
  };

  // ── Operations ──

  const handleReviewRequest = useCallback(
    async (requestId: string, action: "approve" | "reject") => {
      try {
        await apiFetch(`/team/review/${requestId}/${action}`, {
          method: "POST",
        });
        refresh();
        apiFetch<JoinRequest[]>("/team/requests")
          .then(setRequests)
          .catch(() => {});
      } catch (err) {
        toast.error(`操作失败: ${errorMessage(err)}`);
      }
    },
    [refresh],
  );

  const handleChangeRole = useCallback(
    async (userId: string, newRole: string) => {
      try {
        const res = await apiFetch<{ success: boolean; message: string }>(
          `/team/members/role/${userId}`,
          { method: "POST", body: JSON.stringify({ role: newRole }) },
        );
        if (!res.success) {
          toast.error(res.message);
          return;
        }
        refresh();
      } catch (err) {
        toast.error(`角色修改失败: ${errorMessage(err)}`);
      }
    },
    [refresh],
  );

  const handleRemoveMember = useCallback(
    async (userId: string, name: string) => {
      if (!window.confirm(`确定要将 ${name} 移出团队吗？`)) return;
      try {
        const res = await apiFetch<{ success: boolean; message: string }>(
          `/team/members/remove/${userId}`,
          { method: "POST" },
        );
        if (!res.success) {
          toast.error(res.message);
          return;
        }
        refresh();
      } catch (err) {
        toast.error(`移除失败: ${errorMessage(err)}`);
      }
    },
    [refresh],
  );

  const isCurrentUser = (userId: string) => user?.id === userId;
  const isSuperAdmin = user?.role === "superadmin";

  const canManageUser = (member: {
    user_id: string;
    role: string;
  }) => {
    if (!canManage || isCurrentUser(member.user_id)) return false;
    if (member.role === "superadmin") return false;
    if (member.role === "admin" && !isSuperAdmin) return false;
    return true;
  };

  // ── Loading ──

  if (membersLoading) {
    return (
      <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">
        通过插件 API 加载团队成员...
      </div>
    );
  }

  if (membersError) {
    return (
      <div className="p-6 max-w-5xl mx-auto">
        <div className="p-6 bg-red-50 dark:bg-red-900/30 border border-red-300 dark:border-red-800 text-red-700 dark:text-red-300">
          <h2 className="text-lg font-semibold mb-2">插件 API 错误</h2>
          <p className="text-sm mb-4">{membersError}</p>
          <button
            onClick={() => refresh()}
            className="px-4 py-1.5 text-sm border border-red-300 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/20"
          >
            重试
          </button>
        </div>
      </div>
    );
  }

  // ── Render ──

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center gap-3 mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
          团队成员
        </h1>
        <span className="text-xs px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400">
          插件 · SDK
        </span>
      </div>

      {/* Current user (via SDK) */}
      {me && (
        <div className="mb-6 p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
          <p className="text-sm text-gray-600 dark:text-gray-400">
            当前用户：
            <span className="font-medium text-gray-800 dark:text-gray-200">
              {me.display_name}
            </span>
            <span className="ml-2 text-xs text-gray-400">
              ({ROLE_LABEL[me.role] || me.role})
            </span>
          </p>
        </div>
      )}

      {/* Non-member hint */}
      {user && user.team_status !== "joined" && (
        <div className="mb-6 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800">
          <p className="text-blue-700 dark:text-blue-300">
            您还不是团队成员，请联系管理员申请加入。
          </p>
        </div>
      )}

      {/* Join requests */}
      {canManage && requests.length > 0 && (
        <div className="mb-8">
          <h2 className="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200">
            待处理入队申请 ({requests.length})
          </h2>
          <div className="space-y-2">
            {requests.map((req) => (
              <div
                key={req.id}
                className="flex items-center justify-between p-4 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow"
              >
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-gray-800 dark:text-gray-100">
                    {req.user_name}
                    <span className="text-sm text-gray-500 dark:text-gray-400 ml-2">
                      {req.user_email}
                    </span>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-300 mt-1 truncate">
                    {req.reason}
                  </p>
                </div>
                <div className="flex gap-2 ml-4 shrink-0">
                  <button
                    onClick={() => handleReviewRequest(req.id, "approve")}
                    className="px-4 py-1.5 text-sm bg-gray-800 dark:bg-gray-700 text-white hover:bg-gray-700 dark:hover:bg-gray-600"
                  >
                    接受
                  </button>
                  <button
                    onClick={() => handleReviewRequest(req.id, "reject")}
                    className="px-4 py-1.5 text-sm border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
                  >
                    拒绝
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6">
        {([
          { id: "all" as TabId, label: "全部", count: members.length },
          { id: "admin" as TabId, label: "管理员", count: counts.admin },
          { id: "member" as TabId, label: "成员", count: counts.member },
        ]).map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? "border-gray-800 dark:border-gray-100 text-gray-900 dark:text-gray-100"
                : "border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
            }`}
          >
            {tab.label}
            <span
              className={`mg-tab-count ${
                activeTab === tab.id
                  ? "bg-gray-800 dark:bg-gray-600 text-white"
                  : "bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
              }`}
            >
              {tab.count}
            </span>
          </button>
        ))}
      </div>

      {/* Member list */}
      <div className="space-y-2">
        {filteredMembers.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">
            暂无
            {activeTab === "admin"
              ? "管理员"
              : activeTab === "member"
                ? "成员"
                : ""}
          </div>
        ) : (
          filteredMembers.map((m) => (
            <div
              key={m.user_id}
              className="flex items-center justify-between p-4 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
              onClick={() => navigate(`/profile/${m.username}`)}
            >
              <div className="flex items-center gap-3 min-w-0">
                {m.avatar_url ? (
                  <img
                    src={m.avatar_url}
                    alt=""
                    className="w-10 h-10 rounded-full object-cover shrink-0"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = "none";
                    }}
                  />
                ) : (
                  <div className="w-10 h-10 bg-gray-300 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-600 dark:text-gray-400 font-bold text-sm shrink-0">
                    {m.display_name?.charAt(0) || "?"}
                  </div>
                )}
                <div className="min-w-0">
                  <div className="font-medium text-gray-800 dark:text-gray-100 flex items-center gap-2">
                    <span className="truncate">{m.display_name}</span>
                    {isCurrentUser(m.user_id) && (
                      <span className="text-xs text-gray-400 dark:text-gray-500 shrink-0">
                        (你)
                      </span>
                    )}
                  </div>
                  <div className="text-sm text-gray-500 dark:text-gray-400">
                    {m.username}
                    <span className="mx-1 text-gray-300 dark:text-gray-600">
                      ·
                    </span>
                    加入于 {m.joined_at}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-3 shrink-0 ml-4">
                {canManageUser(m) ? (
                  <select
                    value={m.role}
                    onChange={(e) => {
                      e.stopPropagation();
                      handleChangeRole(m.user_id, e.target.value);
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="text-sm border border-gray-300 dark:border-gray-700 px-2 py-1 bg-white dark:bg-gray-800 focus:outline-none"
                  >
                    <option value="member">成员</option>
                    <option value="admin">管理员</option>
                  </select>
                ) : (
                  <span className="text-sm px-3 py-1 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 border border-gray-200 dark:border-gray-700">
                    {ROLE_LABEL[m.role] || m.role}
                  </span>
                )}

                {canManageUser(m) && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleRemoveMember(m.user_id, m.display_name);
                    }}
                    className="px-3 py-1 text-sm text-red-600 dark:text-red-400 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    移除
                  </button>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      <p className="mt-6 text-xs text-gray-400 dark:text-gray-500">
        共 {members.length} 名团队成员（通过插件 API 加载）
        {canManage && ` · ${requests.length} 条待审批申请`}
      </p>
    </div>
  );
}
