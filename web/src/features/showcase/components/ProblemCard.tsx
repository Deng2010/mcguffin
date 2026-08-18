import { Link } from "react-router-dom";
import { DiffBadge } from "../../../hooks/useDifficulties";
import type { ShowcaseProblem } from "../types";

/** 题目大卡片：有 external link 时跳外链，否则进入题目详情页 */
export function ProblemCard({
  p,
  difficultyMap,
  showDifficulty = true,
}: {
  p: ShowcaseProblem;
  difficultyMap: Map<string, any>;
  showDifficulty?: boolean;
}) {
  const badge = showDifficulty ? (
    <DiffBadge
      difficulty={p.difficulty}
      map={difficultyMap}
      className="px-2 py-0.5 text-xs font-medium"
    />
  ) : null;

  const inner = (
    <div className="flex items-center justify-between">
      <div>
        <span className="font-medium text-gray-800 dark:text-gray-100">
          {p.title}
        </span>
        <div className="flex items-center gap-3 mt-0.5 text-xs text-gray-400 dark:text-gray-500">
          <span>作者：{p.author_name}</span>
          {p.contest && <span>{p.contest}</span>}
        </div>
      </div>
      <div className="flex items-center gap-2">{badge}</div>
    </div>
  );

  const cls =
    "block mg-box-shadow p-4 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors";

  if (p.link) {
    return (
      <a
        href={p.link}
        target="_blank"
        rel="noopener noreferrer"
        className={cls}
      >
        {inner}
      </a>
    );
  }
  return (
    <Link to={`/problems/${p.id}`} className={cls}>
      {inner}
    </Link>
  );
}

/** 紧凑题目卡片（嵌套在比赛组件内） */
export function CompactProblemCard({
  p,
  difficultyMap,
  showDifficulty = true,
}: {
  p: ShowcaseProblem;
  difficultyMap: Map<string, any>;
  showDifficulty?: boolean;
}) {
  const badge = showDifficulty ? (
    <DiffBadge
      difficulty={p.difficulty}
      map={difficultyMap}
      className="px-1.5 py-0.5 text-xs"
    />
  ) : null;

  const inner = (
    <div className="flex items-center justify-between">
      <span className="text-sm text-gray-800 dark:text-gray-100">{p.title}</span>
      <div className="flex items-center gap-2">
        <span className="text-xs text-gray-400 dark:text-gray-500">
          作者：{p.author_name}
        </span>
        {badge}
      </div>
    </div>
  );

  const cls =
    "flex items-center justify-between border border-gray-200 dark:border-gray-700 p-2.5 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors";

  if (p.link) {
    return (
      <a
        href={p.link}
        target="_blank"
        rel="noopener noreferrer"
        className={cls}
      >
        {inner}
      </a>
    );
  }
  return (
    <Link to={`/problems/${p.id}`} className={cls}>
      {inner}
    </Link>
  );
}