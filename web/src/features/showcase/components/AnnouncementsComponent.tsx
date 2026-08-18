import { Link } from "react-router-dom";
import MarkdownRenderer from "../../../components/MarkdownRenderer";
import { getAnnouncementsSettings } from "../registry";
import type { ShowcaseComponentProps } from "../types";

/**
 * 展板组件：公告
 * 展示置顶公告与最新公告；条数由组件设置 showCount 控制。
 */
export default function AnnouncementsComponent({
  config,
  ctx,
}: ShowcaseComponentProps) {
  const { showCount } = getAnnouncementsSettings(config);
  const { announcements } = ctx;

  // 置顶公告优先，其余按时间补足到 showCount 条
  const pinned = announcements.filter((a) => a.pinned);
  const others = announcements.filter((a) => !a.pinned);
  const shown = [...pinned, ...others].slice(0, Math.max(pinned.length, showCount));
  const hasMore = announcements.length > shown.length;

  return (
    <section className="h-full">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-700 dark:text-gray-200">
          公告
        </h2>
        {hasMore && (
          <Link
            to="/community?tag=公告"
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            查看全部公告
          </Link>
        )}
      </div>

      {announcements.length === 0 ? (
        <div>
          <p className="text-sm text-gray-400 dark:text-gray-500">暂无公告</p>
          {ctx.canAccessAdmin && (
            <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">
              在配置 → 讨论区 →
              标签管理中添加「公告」标签并发布带有此标签的帖子以发布公告
            </p>
          )}
        </div>
      ) : (
        <div className="space-y-2">
          {shown.map((a) => (
            <div
              key={a.id}
              className={`mg-box-shadow ${a.pinned ? "border-yellow-400" : ""} p-4`}
            >
              <div className="flex items-center gap-2 mb-1">
                {a.pinned && (
                  <span className="text-xs px-1.5 py-0.5 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 border border-yellow-200">
                    置顶
                  </span>
                )}
                <span className="font-medium text-gray-800 dark:text-gray-100 text-sm">
                  {a.title}
                </span>
              </div>
              <div className="text-xs text-gray-400 dark:text-gray-500 mb-2">
                {a.author_name} ·{" "}
                {new Date(a.created_at).toLocaleDateString("zh-CN")}
              </div>
              <div className="text-sm text-gray-600 dark:text-gray-300 prose prose-sm max-w-none">
                <MarkdownRenderer content={a.content} />
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}