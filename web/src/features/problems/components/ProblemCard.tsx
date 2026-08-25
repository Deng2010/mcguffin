import { DiffBadge } from "../../../hooks/useDifficulties";
import type { ProblemListItem } from "../../../types";

export function statusBadge(s: string) {
  switch (s) {
    case "pending":
      return (
        <span className="px-2 py-0.5 text-xs bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300">
          待审核
        </span>
      );
    case "approved":
      return (
        <span className="px-2 py-0.5 text-xs bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
          已通过
        </span>
      );
    case "published":
      return (
        <span className="px-2 py-0.5 text-xs bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300">
          已发布
        </span>
      );
    case "returned":
      return (
        <span className="px-2 py-0.5 text-xs bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-300">
          已退回
        </span>
      );
    default:
      return s;
  }
}

// Shared meta info row for all card types
export function ProblemMeta({
  p,
  difficultyMap,
}: {
  p: ProblemListItem;
  difficultyMap: Map<string, any>;
}) {
  return (
    <div className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-gray-500 dark:text-gray-400 mt-1">
      <span>作者：{p.author_name}</span>
      <span>赛事：{p.contest || "无"}</span>
      <span>
        难度：
        <DiffBadge difficulty={p.difficulty} map={difficultyMap} />
      </span>
      {p.status && <span>状态：{statusBadge(p.status)}</span>}
      {"has_verifier_solution" in p && (p as any).has_verifier_solution && (
        <span className="text-purple-600 dark:text-purple-400 font-medium">
          已有验题人题解
        </span>
      )}
    </div>
  );
}

export const cardClass =
  "mg-box-shadow p-4 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors";
export const guestCardClass = "mg-box-shadow p-4";

interface ProblemCardProps {
  p: ProblemListItem;
  isGuest: boolean;
  isAuthor: boolean;
  difficultyMap: Map<string, any>;
  onGoDetail: (problemId: string) => void;
  extraActions?: React.ReactNode;
}

// ====== Problem Card (shared by all tabs) ======
export default function ProblemCard({
  p,
  isGuest,
  isAuthor,
  difficultyMap,
  onGoDetail,
  extraActions,
}: ProblemCardProps) {
  return (
    <div
      key={p.id}
      className={isGuest ? guestCardClass : cardClass}
      onClick={isGuest ? undefined : () => onGoDetail(p.id)}
    >
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-lg font-semibold text-gray-800 dark:text-gray-100 truncate">
              {p.title}
            </span>
            {statusBadge(p.status)}
            {isAuthor && (
              <span className="text-xs px-2 py-0.5 bg-blue-100 text-blue-700 font-medium dark:bg-blue-900/30 dark:text-blue-300">
                我的题目
              </span>
            )}
            {(p.verifiers || []).length > 0 && (
              <span className="text-xs px-2 py-0.5 bg-purple-100 text-purple-700 font-medium dark:bg-purple-900/30 dark:text-purple-300">
                已有 {(p.verifiers || []).length} 人验题
              </span>
            )}
            {(p.verifiers || []).some((v) => v.has_solution) && (
              <span className="text-xs px-2 py-0.5 bg-purple-50 text-purple-500 font-medium dark:bg-purple-900/20 dark:text-purple-400">
                有验题题解
              </span>
            )}
          </div>
          <ProblemMeta p={p} difficultyMap={difficultyMap} />
        </div>
        <div
          className="flex items-center gap-2 ml-4 shrink-0"
          onClick={(e) => e.stopPropagation()}
        >
          {p.link && (
            <a
              href={p.link}
              target="_blank"
              rel="noopener noreferrer"
              className="px-3 py-1.5 text-xs border border-blue-300 text-blue-600 hover:bg-blue-50 dark:border-blue-800 dark:text-blue-400 dark:hover:bg-blue-900/20"
              title="外部链接"
            >
              打开 ↗
            </a>
          )}
          {isGuest && !p.link && (
            <span className="text-xs text-gray-400 dark:text-gray-500 italic">
              仅团队成员可查看
            </span>
          )}
          {!isGuest && extraActions}
        </div>
      </div>
    </div>
  );
}
