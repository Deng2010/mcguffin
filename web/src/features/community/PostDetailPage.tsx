import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import NotFoundPage from "../notfound/NotFoundPage";
import { useAuthStore } from "../../stores/authStore";
import { apiFetch } from "../../services/api";
import MarkdownRenderer from "../../components/MarkdownRenderer";
import MarkdownEditor from "../../components/MarkdownEditor";
import ReactionRow from "../../components/ReactionRow";
import ReplyCard from "../../components/ReplyCard";
import MentionDropdown from "../../components/MentionDropdown";
import { useMention } from "../../hooks/useMention";
import { formatTime } from "../../utils/time";
import { groupReplies } from "../../utils/groups";
import type { MentionMember } from "../../hooks/useMention";
import type {
  DiscussionTag,
  DiscussionEmoji,
  DiscussionReply,
  PostDetail,
} from "../../types";

// ============== Constants ==============

const REPLY_MAX_LEN = 300;
const REPLY_PAGE_SIZE = 10;

const STATUS_LABEL: Record<string, string> = {
  open: "待处理",
  in_progress: "处理中",
  resolved: "已解决",
  closed: "已关闭",
};

const STATUS_BG_COLOR: Record<string, string> = {
  open: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
  in_progress:
    "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300",
  resolved:
    "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300",
  closed: "bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-400",
};

// ============== Component ==============

export default function PostDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user, hasPermission, isAuthenticated } = useAuthStore();
  const [post, setPost] = useState<PostDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [replyContent, setReplyContent] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [emojis, setEmojis] = useState<DiscussionEmoji[]>([]);
  const [allTags, setAllTags] = useState<DiscussionTag[]>([]);
  const [editingTags, setEditingTags] = useState(false);
  const [editTagIds, setEditTagIds] = useState<string[]>([]);
  const [savingTags, setSavingTags] = useState(false);
  const [replyTo, setReplyTo] = useState<DiscussionReply | null>(null);
  const [replyPage, setReplyPage] = useState(1);
  const [teamMembers, setTeamMembers] = useState<MentionMember[]>([]);
  const mainMention = useMention(teamMembers);
  const inlineMention = useMention(teamMembers);
  const [allMembers, setAllMembers] = useState<
    { id: string; username: string; display_name: string }[]
  >([]);
  const [editVisibleTo, setEditVisibleTo] = useState<string[]>([]);
  const [editEditableBy, setEditEditableBy] = useState<string[]>([]);
  const [editingAcl, setEditingAcl] = useState(false);
  const [savingAcl, setSavingAcl] = useState(false);

  const isAdmin = hasPermission("manage_posts");
  const canDelete = post && (isAdmin || post.author_id === user?.id);
  const replyCharsLeft = REPLY_MAX_LEN - replyContent.length;

  const loadPost = () => {
    if (!id) return;
    apiFetch<PostDetail>(`/posts/${id}`)
      .then((data) => {
        setPost(data);
        setReplyPage(1);
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadPost();
    apiFetch<DiscussionEmoji[]>("/posts/emojis")
      .then(setEmojis)
      .catch(() => {});
    apiFetch<DiscussionTag[]>("/posts/tags")
      .then(setAllTags)
      .catch(() => {});
    apiFetch<MentionMember[]>("/team/members")
      .then(setTeamMembers)
      .catch(() => {});
    if (isAdmin && allMembers.length === 0) {
      apiFetch<{ id: string; username: string; display_name: string }[]>(
        "/admin/users",
      )
        .then(setAllMembers)
        .catch(() => {});
    }
  }, [id]);

  const handleReply = async () => {
    if (!replyContent.trim() || !id) return;
    if (replyContent.length > REPLY_MAX_LEN) {
      alert(`回复不能超过${REPLY_MAX_LEN}字`);
      return;
    }
    setSubmitting(true);
    try {
      const body: Record<string, any> = { content: replyContent.trim() };
      body.mentioned_user_ids = mainMention.getMentionedUserIds(replyContent);
      if (replyTo) {
        body.parent_id = replyTo.id;
        body.reply_to = replyTo.author_name;
      }
      await apiFetch(`/posts/${id}/reply`, {
        method: "POST",
        body: JSON.stringify(body),
      });
      setReplyContent("");
      setReplyTo(null);
      loadPost();
    } catch (err) {
      alert(`回复失败: ${err}`);
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!post || !id) return;
    if (!confirm("确定删除此帖子？")) return;
    try {
      const res = await apiFetch<any>(`/posts/${id}`, { method: "DELETE" });
      if (res.success) {
        navigate("/community");
      } else {
        alert(res.message || "删除失败");
      }
    } catch (err) {
      alert(`删除失败: ${err}`);
    }
  };

  const handleDeleteReply = async (replyId: string) => {
    if (!id) return;
    if (!confirm("确定删除此回复？")) return;
    try {
      const res = await apiFetch<any>(`/posts/${id}/reply/${replyId}`, {
        method: "DELETE",
      });
      if (res.success) {
        loadPost();
      } else {
        alert(res.message || "删除失败");
      }
    } catch (err) {
      alert(`删除失败: ${err}`);
    }
  };

  const handleReact = async (emoji: string) => {
    if (!id) return;
    try {
      await apiFetch(`/posts/${id}/react`, {
        method: "POST",
        body: JSON.stringify({ emoji }),
      });
      loadPost();
    } catch {
      /* ignore */
    }
  };

  const handleReactReply = async (replyId: string, emoji: string) => {
    if (!id) return;
    try {
      await apiFetch(`/posts/${id}/reply/${replyId}/react`, {
        method: "POST",
        body: JSON.stringify({ emoji }),
      });
      loadPost();
    } catch {
      /* ignore */
    }
  };

  const toggleEditTag = (tagId: string) => {
    setEditTagIds((prev) =>
      prev.includes(tagId) ? prev.filter((t) => t !== tagId) : [...prev, tagId],
    );
  };

  const handleSaveTags = async () => {
    if (!id) return;
    setSavingTags(true);
    try {
      await apiFetch(`/posts/${id}`, {
        method: "PUT",
        body: JSON.stringify({ tags: editTagIds }),
      });
      setEditingTags(false);
      loadPost();
    } catch (err) {
      alert(`保存失败: ${err}`);
    } finally {
      setSavingTags(false);
    }
  };

  const handleTogglePinned = async () => {
    if (!id || !post) return;
    try {
      await apiFetch(`/posts/${id}`, {
        method: "PUT",
        body: JSON.stringify({ pinned: !post.pinned }),
      });
      loadPost();
    } catch (err) {
      alert(`操作失败: ${err}`);
    }
  };

  const handleToggleTeamOnly = async () => {
    if (!id || !post) return;
    try {
      await apiFetch(`/posts/${id}`, {
        method: "PUT",
        body: JSON.stringify({ team_only: !post.team_only }),
      });
      loadPost();
    } catch (err) {
      alert(`操作失败: ${err}`);
    }
  };

  const toggleVisibleMember = (userId: string) => {
    setEditVisibleTo((prev) =>
      prev.includes(userId)
        ? prev.filter((id) => id !== userId)
        : [...prev, userId],
    );
  };

  const toggleEditableMember = (userId: string) => {
    setEditEditableBy((prev) =>
      prev.includes(userId)
        ? prev.filter((id) => id !== userId)
        : [...prev, userId],
    );
  };

  const handleSaveAcl = async () => {
    if (!id) return;
    setSavingAcl(true);
    try {
      const res = await apiFetch<{ success: boolean; message: string }>(
        `/admin/acl/post/${id}`,
        {
          method: "PUT",
          body: JSON.stringify({
            visible_to: editVisibleTo,
            editable_by: editEditableBy,
          }),
        },
      );
      if (res.success) {
        setEditingAcl(false);
        loadPost();
      } else {
        alert(res.message || "保存权限失败");
      }
    } catch (err) {
      alert(`保存权限失败: ${err}`);
    } finally {
      setSavingAcl(false);
    }
  };

  const handleStatusChange = async (status: string) => {
    if (!id) return;
    try {
      await apiFetch(`/posts/${id}`, {
        method: "PUT",
        body: JSON.stringify({ status }),
      });
      loadPost();
    } catch (err) {
      alert(`更新失败: ${err}`);
    }
  };

  const handleStartReply = (reply: DiscussionReply) => {
    setReplyTo(replyTo?.id === reply.id ? null : reply);
  };

  if (loading)
    return (
      <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">
        加载中...
      </div>
    );
  if (!post) return <NotFoundPage />;

  const hasStatus =
    post.status &&
    ["open", "in_progress", "resolved", "closed"].includes(post.status);
  const { topLevel, childrenMap } = groupReplies(post.replies);
  const replyTotalPages = Math.max(
    1,
    Math.ceil(topLevel.length / REPLY_PAGE_SIZE),
  );
  const safeReplyPage = Math.min(replyPage, replyTotalPages);
  const pagedTopLevel = topLevel.slice(
    (safeReplyPage - 1) * REPLY_PAGE_SIZE,
    safeReplyPage * REPLY_PAGE_SIZE,
  );

  return (
    <div className="max-w-4xl mx-auto px-6 py-8">
      {/* Back button */}
      <button
        onClick={() => navigate("/community")}
        className="mb-6 inline-flex items-center gap-1 px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 border border-gray-300 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800"
      >
        <svg
          className="w-3.5 h-3.5"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M15 19l-7-7 7-7"
          />
        </svg>
        返回社区
      </button>

      {/* Post card */}
      <div
        className={`bg-white border dark:bg-gray-900 shadow p-6 mb-6 ${
          post.pinned
            ? "border-yellow-400 ring-1 ring-yellow-100 dark:border-yellow-800 dark:ring-yellow-900/30"
            : "border-gray-300 dark:border-gray-700"
        }`}
      >
        {/* Emoji + Title + Delete */}
        <div className="flex items-start gap-3 mb-3">
          {post.emoji && (
            <span className="text-4xl leading-none shrink-0 mt-0.5">
              {post.emoji}
            </span>
          )}
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 flex-wrap">
              <h1 className="text-xl font-bold text-gray-800 dark:text-gray-100">
                {post.title}
              </h1>
              {post.pinned && (
                <span className="text-xs px-1.5 py-0.5 border border-red-300 dark:border-red-800 text-red-500 dark:text-red-400 leading-none">
                  置顶
                </span>
              )}
              {post.team_only && (
                <span className="text-xs px-1.5 py-0.5 border border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 leading-none">
                  内部
                </span>
              )}
              {hasStatus && (
                <span
                  className={`text-xs leading-none ${STATUS_BG_COLOR[post.status] || ""}`}
                >
                  {STATUS_LABEL[post.status] || post.status}
                </span>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {/* Status dropdown (admin) */}
            {isAdmin && hasStatus && (
              <select
                value={post.status}
                onChange={(e) => handleStatusChange(e.target.value)}
                className="text-xs border border-gray-300 dark:border-gray-700 px-2 py-1 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-200 focus:outline-none"
              >
                <option value="open">待处理</option>
                <option value="in_progress">处理中</option>
                <option value="resolved">已解决</option>
                <option value="closed">已关闭</option>
              </select>
            )}
            {canDelete && (
              <button
                onClick={handleDelete}
                className="shrink-0 px-2 py-1 text-xs text-red-500 border border-red-300 dark:border-red-800 hover:bg-red-50 dark:hover:bg-red-900/20"
              >
                删除
              </button>
            )}
          </div>
        </div>

        {/* Meta */}
        <div className="flex items-center gap-3 text-xs text-gray-400 dark:text-gray-500 mb-3 ml-0">
          <span className="flex items-center gap-1.5">
            {post.author_avatar_url ? (
              <img
                src={post.author_avatar_url}
                className="w-5 h-5 rounded-full object-cover"
                alt=""
              />
            ) : (
              <span className="w-5 h-5 inline-flex items-center justify-center bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 text-[10px] font-bold shrink-0">
                {post.author_name?.charAt(0) || "?"}
              </span>
            )}
            <span>{post.author_name}</span>
          </span>
          <span>{formatTime(post.created_at)}</span>
        </div>

        {/* Tags */}
        {editingTags ? (
          <div className="mb-4">
            <div className="flex flex-wrap gap-1.5 mb-2">
              {allTags.map((tag) => (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => toggleEditTag(tag.id)}
                  className={`text-xs px-2 py-0.5 inline-flex items-center gap-1 border ${
                    editTagIds.includes(tag.id)
                      ? "border-gray-600 dark:border-gray-400 bg-gray-100 dark:bg-gray-700"
                      : "border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800"
                  }`}
                >
                  <span
                    className="w-1.5 h-1.5 inline-block"
                    style={{ backgroundColor: tag.color }}
                  />
                  {tag.name}
                  {tag.admin_only && (
                    <span className="text-[10px] text-gray-400 ml-0.5">
                      (管理)
                    </span>
                  )}
                </button>
              ))}
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSaveTags}
                disabled={savingTags}
                className="px-3 py-1 text-xs bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50"
              >
                {savingTags ? "保存中..." : "保存"}
              </button>
              <button
                onClick={() => {
                  setEditingTags(false);
                }}
                className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800"
              >
                取消
              </button>
            </div>
          </div>
        ) : (
          ((post.tags && post.tags.length > 0) || isAdmin) && (
            <div className="flex flex-wrap items-center gap-1.5 mb-4 ml-0">
              {post.tags.map((tag) => (
                <span
                  key={tag.id}
                  className="text-xs px-2 py-0.5 inline-flex items-center gap-1 border border-gray-300 dark:border-gray-700"
                >
                  <span
                    className="w-1.5 h-1.5 inline-block"
                    style={{ backgroundColor: tag.color }}
                  />
                  {tag.name}
                </span>
              ))}
              {isAdmin && (
                <button
                  onClick={() => {
                    setEditTagIds(post.tags.map((t) => t.id));
                    setEditingTags(true);
                  }}
                  className="text-xs px-2 py-0.5 border border-dashed border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800"
                  title="编辑标签"
                >
                  + 编辑标签
                </button>
              )}
            </div>
          )
        )}

        {/* Admin controls */}
        {isAdmin && (
          <div className="flex items-center gap-3 mb-4 ml-0">
            <button
              onClick={handleTogglePinned}
              className={`text-xs px-2 py-0.5 border ${
                post.pinned
                  ? "border-red-300 dark:border-red-800 text-red-500 dark:text-red-400 bg-red-50 dark:bg-red-900/20"
                  : "border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800"
              }`}
            >
              {post.pinned ? "取消置顶" : "置顶"}
            </button>
            <button
              onClick={handleToggleTeamOnly}
              className={`text-xs px-2 py-0.5 border ${
                post.team_only
                  ? "border-gray-600 dark:border-gray-400 text-gray-600 dark:text-gray-300 bg-gray-100 dark:bg-gray-800"
                  : "border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800"
              }`}
            >
              {post.team_only ? "设为公开" : "设为内部"}
            </button>
            <button
              onClick={() => {
                setEditVisibleTo(post.visible_to || []);
                setEditEditableBy(post.editable_by || []);
                setEditingAcl(!editingAcl);
              }}
              className={`text-xs px-2 py-0.5 border ${
                editingAcl
                  ? "border-gray-600 dark:border-gray-400 text-gray-600 dark:text-gray-300 bg-gray-100 dark:bg-gray-800"
                  : "border-gray-300 dark:border-gray-700 text-gray-400 dark:text-gray-500 hover:bg-gray-50 dark:hover:bg-gray-800"
              }`}
            >
              {editingAcl ? "关闭权限" : "权限控制"}
            </button>
          </div>
        )}
        {editingAcl && allMembers.length > 0 && (
          <div className="mb-4 ml-0 border border-gray-200 dark:border-gray-700 p-3">
            <h4 className="text-xs font-semibold text-gray-700 mb-3 dark:text-gray-200">
              访问控制
            </h4>
            <div className="mb-3">
              <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">
                可见成员
              </label>
              <div className="flex flex-wrap gap-2">
                {allMembers.map((m) => (
                  <label
                    key={m.id}
                    className="flex items-center gap-1.5 text-sm cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={editVisibleTo.includes(m.id)}
                      onChange={() => toggleVisibleMember(m.id)}
                      className="accent-gray-800 dark:accent-gray-400"
                    />
                    {m.display_name || m.username}
                  </label>
                ))}
              </div>
            </div>
            <div className="mb-3">
              <label className="block text-xs font-medium mb-1.5 text-gray-600 dark:text-gray-300">
                可编辑成员
              </label>
              <div className="flex flex-wrap gap-2">
                {allMembers.map((m) => (
                  <label
                    key={m.id}
                    className="flex items-center gap-1.5 text-sm cursor-pointer"
                  >
                    <input
                      type="checkbox"
                      checked={editEditableBy.includes(m.id)}
                      onChange={() => toggleEditableMember(m.id)}
                      className="accent-gray-800 dark:accent-gray-400"
                    />
                    {m.display_name || m.username}
                  </label>
                ))}
              </div>
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSaveAcl}
                disabled={savingAcl}
                className="px-3 py-1 text-xs bg-gray-800 text-white hover:bg-gray-700 disabled:opacity-50"
              >
                {savingAcl ? "保存中..." : "保存权限"}
              </button>
            </div>
          </div>
        )}

        {/* Content */}
        <div className="prose prose-sm max-w-none text-gray-700 dark:text-gray-200">
          <MarkdownRenderer content={post.content} />
        </div>

        {/* Reactions */}
        <ReactionRow
          reactions={post.reactions}
          emojis={emojis}
          currentUserId={user?.id}
          onReact={handleReact}
        />
      </div>

      {/* Replies section */}
      <div className="mb-6">
        <h2 className="text-base font-semibold text-gray-800 dark:text-gray-100 mb-3">
          回复 ({post.replies?.length || 0})
        </h2>

        {topLevel.length === 0 ? (
          <div className="text-center py-8 text-gray-400 dark:text-gray-500 text-sm">
            暂无回复
          </div>
        ) : (
          <div className="space-y-3">
            {pagedTopLevel.map((reply) => (
              <div key={reply.id}>
                <ReplyCard
                  reply={reply}
                  emojis={emojis}
                  currentUserId={user?.id}
                  isAdmin={isAdmin}
                  onDelete={handleDeleteReply}
                  onReact={handleReactReply}
                  onReply={handleStartReply}
                >
                  {/* Children (sub-replies) */}
                  {childrenMap[reply.id] &&
                    childrenMap[reply.id].length > 0 && (
                      <div className="mt-3 space-y-2 ml-0 border-l-2 border-gray-200 dark:border-gray-700 pl-4">
                        {childrenMap[reply.id].map((child) => (
                          <ReplyCard
                            key={child.id}
                            reply={child}
                            emojis={emojis}
                            currentUserId={user?.id}
                            isAdmin={isAdmin}
                            onDelete={handleDeleteReply}
                            onReact={handleReactReply}
                            onReply={handleStartReply}
                            hideReplyButton
                          />
                        ))}
                      </div>
                    )}

                  {/* Sub-reply form inline */}
                  {replyTo?.id === reply.id && (
                    <div className="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
                      <div className="text-xs text-gray-400 dark:text-gray-500 mb-2">
                        回复{" "}
                        <span className="font-medium text-gray-600 dark:text-gray-300">
                          @{replyTo.author_name}
                        </span>
                      </div>
                      <div className="flex gap-2 items-start relative">
                        <input
                          type="text"
                          value={replyContent}
                          onChange={(e) => {
                            const val = e.target.value;
                            if (val.length <= REPLY_MAX_LEN) {
                              setReplyContent(val);
                              inlineMention.handleTextChange(
                                val,
                                e.target.selectionStart,
                              );
                            }
                          }}
                          onKeyDown={(e) => {
                            if (inlineMention.handleKeyDown(e)) return;
                            if (
                              inlineMention.open &&
                              (e.key === "Enter" || e.key === "Tab")
                            ) {
                              e.preventDefault();
                              const [newText, selected] =
                                inlineMention.insertSelected(
                                  replyContent,
                                  e.currentTarget.selectionStart,
                                );
                              if (selected) {
                                setReplyContent(newText);
                              }
                              return;
                            }
                            if (e.key === "Escape") {
                              setReplyTo(null);
                              setReplyContent("");
                            }
                            if (e.ctrlKey && e.key === "Enter") {
                              e.preventDefault();
                              handleReply();
                            }
                          }}
                          placeholder="输入回复..."
                          autoFocus
                          className="flex-1 border border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 text-gray-800 dark:text-gray-100 px-3 py-2 text-sm"
                        />
                        <MentionDropdown
                          open={inlineMention.open}
                          filtered={inlineMention.filtered}
                          selectedIndex={inlineMention.selectedIndex}
                          className="top-full mt-1 left-0"
                          onSelect={(m) => {
                            const inp =
                              document.querySelector<HTMLInputElement>("input");
                            const cursorPos =
                              inp?.selectionStart ?? replyContent.length;
                            const newText = inlineMention.insertMention(
                              replyContent,
                              cursorPos,
                              m,
                            );
                            setReplyContent(newText);
                          }}
                        />
                        <button
                          onClick={handleReply}
                          disabled={submitting || !replyContent.trim()}
                          className="px-3 py-2 bg-gray-800 text-white text-xs hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
                        >
                          {submitting ? "..." : "发送"}
                        </button>
                      </div>
                      <div className="flex justify-between mt-1">
                        <span className="text-xs text-gray-400 dark:text-gray-500">
                          Esc 取消
                        </span>
                        <span
                          className={`text-xs ${replyCharsLeft < 0 ? "text-red-500 font-bold" : replyCharsLeft < 30 ? "text-yellow-500" : "text-gray-400 dark:text-gray-500"}`}
                        >
                          {replyContent.length} / {REPLY_MAX_LEN}
                        </span>
                      </div>
                    </div>
                  )}
                </ReplyCard>
              </div>
            ))}
          </div>
        )}

        {/* Reply pagination */}
        {replyTotalPages > 1 && (
          <div className="flex items-center justify-center gap-2 mt-4">
            <button
              disabled={safeReplyPage <= 1}
              onClick={() => setReplyPage((p) => Math.max(1, p - 1))}
              className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
            >
              上一页
            </button>
            <span className="text-xs text-gray-400 dark:text-gray-500 px-2">
              {safeReplyPage} / {replyTotalPages}
            </span>
            <button
              disabled={safeReplyPage >= replyTotalPages}
              onClick={() =>
                setReplyPage((p) => Math.min(replyTotalPages, p + 1))
              }
              className="px-3 py-1 text-xs border border-gray-300 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed"
            >
              下一页
            </button>
          </div>
        )}
      </div>

      {/* Top-level reply form */}
      {isAuthenticated ? (
        <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 shadow p-4 relative">
          <h3 className="text-sm font-medium text-gray-700 dark:text-gray-200 mb-2">
            回复
          </h3>
          <div className="relative">
            <MarkdownEditor
              value={replyContent}
              onChange={setReplyContent}
              placeholder="输入回复（支持 Markdown，Ctrl+Enter 发送）"
              rows={6}
            />
            <MentionDropdown
              open={mainMention.open}
              filtered={mainMention.filtered}
              selectedIndex={mainMention.selectedIndex}
              className="bottom-full mb-1 left-0"
              onSelect={(m) => {
                setReplyContent((prev) =>
                  mainMention.insertMention(prev, prev.length, m),
                );
              }}
            />
          </div>
          <div className="flex items-center justify-between mt-2">
            <span
              className={`text-xs ${replyCharsLeft < 0 ? "text-red-500 font-bold" : replyCharsLeft < 30 ? "text-yellow-500" : "text-gray-400 dark:text-gray-500"}`}
            >
              {replyContent.length} / {REPLY_MAX_LEN}
            </span>
            <button
              onClick={handleReply}
              disabled={submitting || !replyContent.trim()}
              className="px-4 py-2 bg-gray-800 text-white text-sm hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
            >
              {submitting ? "发送中..." : "发送"}
            </button>
          </div>
        </div>
      ) : (
        <div className="bg-white border border-gray-300 dark:bg-gray-900 dark:border-gray-700 shadow p-6 text-center text-gray-400 dark:text-gray-500">
          <p className="text-sm">
            请
            <a
              href="/login"
              className="text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 underline"
            >
              登录
            </a>
            后参与回复
          </p>
        </div>
      )}
    </div>
  );
}
