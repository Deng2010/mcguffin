import { Link } from "react-router-dom";
import MarkdownRenderer from "../../../components/MarkdownRenderer";
import { contestStatus as calcContestStatus } from "../../../utils/time";
import { getContestsSettings } from "../registry";
import { CompactProblemCard } from "./ProblemCard";
import type {
  ShowcaseComponentProps,
  ShowcaseContest,
  ShowcaseProblem,
} from "../types";

function statusBadge(
  start: string,
  end: string,
  serverNowMs: number | null,
  timezone?: string,
): { label: string; color: string } {
  const status = calcContestStatus(start, end, serverNowMs, timezone);
  if (status === "ended")
    return {
      label: "已结束",
      color: "bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-400",
    };
  if (status === "running")
    return {
      label: "进行中",
      color:
        "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300",
    };
  return {
    label: "未开始",
    color: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300",
  };
}

/** 按 contest.problem_order 排序的比赛内嵌题目 */
function contestProblems(
  contest: ShowcaseContest,
  allProblems: ShowcaseProblem[],
): ShowcaseProblem[] {
  const list = allProblems.filter((p) => p.contest_id === contest.id);
  if (contest.problem_order && contest.problem_order.length > 0) {
    const orderMap = new Map(contest.problem_order.map((id, i) => [id, i]));
    list.sort((a, b) => {
      const ai = orderMap.get(a.id) ?? 999;
      const bi = orderMap.get(b.id) ?? 999;
      return ai - bi;
    });
  }
  return list;
}

/**
 * 展板组件：比赛
 * 展示公开比赛；按组件设置 selectedIds 筛选（不选 = 全部），
 * showDescription / showProblems 控制比赛简介与内嵌题目是否展示。
 */
export default function ContestsComponent({
  config,
  ctx,
}: ShowcaseComponentProps) {
  const { selectedIds, showDescription, showProblems } =
    getContestsSettings(config);

  const list: ShowcaseContest[] =
    selectedIds.length > 0
      ? (selectedIds
          .map((id) => ctx.contests.find((c) => c.id === id))
          .filter(Boolean) as ShowcaseContest[])
      : ctx.contests;

  if (list.length === 0) return null;

  return (
    <section className="h-full">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-700 dark:text-gray-200">
          比赛 ({list.length})
        </h2>
        {list.length < ctx.contests.length && (
          <Link
            to="/contests"
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            查看全部比赛
          </Link>
        )}
      </div>

      <div className="space-y-4">
        {list.map((c) => {
          const status = statusBadge(
            c.start_time,
            c.end_time,
            ctx.getServerNow(),
            ctx.siteInfo?.timezone,
          );
          const cProblems = contestProblems(c, ctx.problems);
          return (
            <div key={c.id} className="mg-box-shadow p-5">
              <div className="flex items-start justify-between mb-2">
                <div>
                  <h3 className="text-lg font-bold text-gray-800 dark:text-gray-100">
                    {c.name}
                  </h3>
                  <div className="flex items-center gap-3 mt-1">
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {c.start_time} ~ {c.end_time}
                    </span>
                    <span
                      className={`px-2 py-0.5 text-xs font-medium ${status.color}`}
                    >
                      {status.label}
                    </span>
                  </div>
                </div>
                {c.link && (
                  <a
                    href={c.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="shrink-0 px-3 py-1.5 text-xs border border-blue-300 dark:border-blue-800 text-blue-600 dark:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded"
                  >
                    进入比赛 ↗
                  </a>
                )}
              </div>
              {showDescription && c.description && (
                <MarkdownRenderer content={c.description} className="mb-3" />
              )}
              {showProblems &&
                (cProblems.length > 0 ? (
                  <div className="space-y-1.5">
                    {cProblems.map((p) => (
                      <CompactProblemCard
                        key={p.id}
                        p={p}
                        difficultyMap={ctx.difficultyMap}
                      />
                    ))}
                  </div>
                ) : (
                  <div className="text-sm text-gray-400 dark:text-gray-500">
                    暂无题目
                  </div>
                ))}
            </div>
          );
        })}
      </div>
    </section>
  );
}