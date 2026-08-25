import type { DifficultyEntry } from "../../../types";

interface FilterBarProps {
  searchText: string;
  filterDifficulty: string;
  filterAuthor: string;
  difficulties: DifficultyEntry[];
  onSearchTextChange: (v: string) => void;
  onFilterDifficultyChange: (v: string) => void;
  onFilterAuthorChange: (v: string) => void;
  onReset: () => void;
}

export default function FilterBar({
  searchText,
  filterDifficulty,
  filterAuthor,
  difficulties,
  onSearchTextChange,
  onFilterDifficultyChange,
  onFilterAuthorChange,
  onReset,
}: FilterBarProps) {
  const hasActiveFilters = searchText || filterDifficulty || filterAuthor;
  return (
    <div className="mg-box-shadow p-4 mb-4 space-y-3">
      {/* Search row */}
      <div className="flex items-center gap-2">
        <svg
          className="w-4 h-4 text-gray-400 dark:text-gray-500 shrink-0"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
        <input
          type="text"
          value={searchText}
          onChange={(e) => onSearchTextChange(e.target.value)}
          placeholder="搜索题目名称..."
          className="flex-1 px-3 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
        />
      </div>

      {/* Filter row */}
      <div className="flex flex-wrap items-center gap-3">
        {/* Difficulty */}
        <div className="flex items-center gap-1.5">
          <label className="text-xs text-gray-500 dark:text-gray-400 font-medium">
            难度
          </label>
          <select
            value={filterDifficulty}
            onChange={(e) => onFilterDifficultyChange(e.target.value)}
            className="px-2 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
          >
            <option value="">全部</option>
            {difficulties.map((d) => (
              <option key={d.name} value={d.name}>
                {d.label}
              </option>
            ))}
          </select>
        </div>

        {/* Author */}
        <div className="flex items-center gap-1.5">
          <label className="text-xs text-gray-500 dark:text-gray-400 font-medium">
            作者
          </label>
          <input
            type="text"
            value={filterAuthor}
            onChange={(e) => onFilterAuthorChange(e.target.value)}
            placeholder="作者名..."
            className="w-28 px-2 py-1.5 border border-gray-300 bg-white text-sm focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
          />
        </div>

        {/* Reset */}
        {hasActiveFilters && (
          <button
            onClick={onReset}
            className="px-2 py-1.5 text-xs text-gray-500 hover:text-gray-800 hover:bg-gray-100 border border-gray-200 dark:text-gray-400 dark:hover:text-gray-100 dark:hover:bg-gray-800 dark:border-gray-700"
          >
            清除筛选
          </button>
        )}
      </div>
    </div>
  );
}
