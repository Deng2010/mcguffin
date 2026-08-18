import { Link } from "react-router-dom";
import { getProblemsSettings } from "../registry";
import { ProblemCard } from "./ProblemCard";
import type { ShowcaseComponentProps, ShowcaseProblem } from "../types";

/**
 * 展板组件：公开题目
 * 展示已发布题目；按组件设置 selectedIds 筛选（不选 = 全部），
 * showDifficulty 控制是否显示难度徽标。
 */
export default function ProblemsComponent({
  config,
  ctx,
}: ShowcaseComponentProps) {
  const { selectedIds, showDifficulty } = getProblemsSettings(config);

  const list: ShowcaseProblem[] =
    selectedIds.length > 0
      ? (selectedIds
          .map((id) => ctx.problems.find((p) => p.id === id))
          .filter(Boolean) as ShowcaseProblem[])
      : ctx.problems;

  if (list.length === 0) return null;

  return (
    <section className="h-full">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-gray-700 dark:text-gray-200">
          公开题目 ({list.length})
        </h2>
        {list.length < ctx.problems.length && (
          <Link
            to="/problems"
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            查看全部题目
          </Link>
        )}
      </div>
      <div className="space-y-2">
        {list.map((p) => (
          <ProblemCard
            key={p.id}
            p={p}
            difficultyMap={ctx.difficultyMap}
            showDifficulty={showDifficulty}
          />
        ))}
      </div>
    </section>
  );
}