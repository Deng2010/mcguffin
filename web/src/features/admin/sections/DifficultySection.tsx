import { useContext } from "react";
import { ConfigCtx } from "../config-context";

export default function DifficultySection() {
  const c = useContext(ConfigCtx);
  return (
    <div>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
        添加、编辑或删除难度等级。名称用作内部标识（如
        Easy），标签显示给用户（如 简单），颜色用于 UI 展示。使用 ↑↓
        按钮调整显示顺序。
      </p>
      <div className="space-y-3">
        {c.difficulties.map((d, i) => (
          <div
            key={i}
            className="flex items-center gap-2 bg-gray-50 dark:bg-gray-800/50 p-2"
          >
            <div className="flex flex-col gap-0.5">
              <button
                onClick={() => c.moveDiff(i, -1)}
                disabled={i === 0}
                className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
              >
                ↑
              </button>
              <button
                onClick={() => c.moveDiff(i, 1)}
                disabled={i === c.difficulties.length - 1}
                className="text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 disabled:opacity-30 text-xs leading-none px-1"
              >
                ↓
              </button>
            </div>
            <span className="text-xs text-gray-400 w-5 text-right">
              {i + 1}
            </span>
            <input
              type="text"
              value={d.name}
              onChange={(e) => c.updateDiff(i, "name", e.target.value)}
              className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
              placeholder="名称"
            />
            <input
              type="text"
              value={d.label}
              onChange={(e) => c.updateDiff(i, "label", e.target.value)}
              className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
              placeholder="标签"
            />
            <input
              type="color"
              value={d.color}
              onChange={(e) => c.updateDiff(i, "color", e.target.value)}
              className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
            />
            <span className="text-xs text-gray-500 dark:text-gray-400 w-20">
              {d.color}
            </span>
            <button
              onClick={() => c.removeDiff(i)}
              className="px-2 py-1 text-red-600 text-sm hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
            >
              删除
            </button>
          </div>
        ))}
        <div className="flex items-center gap-2 bg-blue-50 dark:bg-blue-900/30 p-2 border border-dashed border-blue-300 dark:border-blue-800">
          <input
            type="text"
            value={c.newDiffName}
            onChange={(e) => c.setNewDiffName(e.target.value)}
            className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
            placeholder="新难度名称"
          />
          <input
            type="text"
            value={c.newDiffLabel}
            onChange={(e) => c.setNewDiffLabel(e.target.value)}
            className="flex-1 px-3 py-1.5 border border-gray-300 bg-white dark:border-gray-700 dark:bg-gray-800 text-sm focus:outline-none focus:border-gray-500"
            placeholder="显示标签"
          />
          <input
            type="color"
            value={c.newDiffColor}
            onChange={(e) => c.setNewDiffColor(e.target.value)}
            className="w-10 h-9 p-0.5 border border-gray-300 dark:border-gray-700 cursor-pointer"
          />
          <button
            onClick={c.addDiff}
            className="px-3 py-1.5 bg-blue-600 text-white text-sm hover:bg-blue-700 dark:bg-blue-700 dark:hover:bg-blue-600"
          >
            添加
          </button>
        </div>
      </div>
    </div>
  );
}
