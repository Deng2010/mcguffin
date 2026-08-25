import { Link } from "react-router-dom";
import MarkdownEditor from "../../../components/MarkdownEditor";
import type { Difficulty } from "../../../types";

export type ContestMode = "none" | "select" | "custom";

interface ContestOption {
  id: string;
  name: string;
}

interface SubmitProblemFormProps {
  canSubmit: boolean;
  teamStatus?: string;
  contests: ContestOption[];
  difficulties: { name: string; label: string; color: string }[];
  // form state
  formTitle: string;
  contestMode: ContestMode;
  selectedContestId: string;
  customContest: string;
  formDifficulty: string;
  formContent: string;
  formSolution: string;
  formRemark: string;
  submitted: boolean;
  formError: string;
  // setters
  setFormTitle: (v: string) => void;
  setContestMode: (v: ContestMode) => void;
  setSelectedContestId: (v: string) => void;
  setCustomContest: (v: string) => void;
  setFormDifficulty: (v: string) => void;
  setFormContent: (v: string) => void;
  setFormSolution: (v: string) => void;
  setFormRemark: (v: string) => void;
  onSubmit: (e: React.FormEvent) => void;
}

export default function SubmitProblemForm({
  canSubmit,
  teamStatus,
  contests,
  difficulties,
  formTitle,
  contestMode,
  selectedContestId,
  customContest,
  formDifficulty,
  formContent,
  formSolution,
  formRemark,
  submitted,
  formError,
  setFormTitle,
  setContestMode,
  setSelectedContestId,
  setCustomContest,
  setFormDifficulty,
  setFormContent,
  setFormSolution,
  setFormRemark,
  onSubmit,
}: SubmitProblemFormProps) {
  if (!canSubmit) {
    return (
      <div className="text-center py-12">
        <h2 className="text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4">
          投稿题目
        </h2>
        <p className="text-gray-600 dark:text-gray-300 mb-6">
          只有团队成员才能投稿题目
        </p>
        {teamStatus === "pending" ? (
          <div className="bg-yellow-50 border border-yellow-300 p-4 max-w-md mx-auto dark:bg-yellow-900/30 dark:border-yellow-800">
            <p className="text-yellow-700 dark:text-yellow-300">
              您的入队申请正在审核中...
            </p>
          </div>
        ) : (
          <Link
            to="/apply"
            className="inline-block px-6 py-3 bg-gray-800 text-white font-medium hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            申请加入团队
          </Link>
        )}
      </div>
    );
  }

  return (
    <form onSubmit={onSubmit} className="max-w-3xl">
      {formError && (
        <div className="mb-4 p-3 bg-red-50 border border-red-300 text-red-700 text-sm dark:bg-red-900/30 dark:border-red-800 dark:text-red-300">
          {formError}
        </div>
      )}
      {submitted && (
        <div className="mb-4 p-3 bg-green-50 border border-green-300 text-green-700 text-sm dark:bg-green-900/30 dark:border-green-800 dark:text-green-300">
          提交成功！题目已进入审核流程。
        </div>
      )}

      <div className="mb-4">
        <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">
          题目标题
        </label>
        <input
          type="text"
          value={formTitle}
          onChange={(e) => setFormTitle(e.target.value)}
          required
          className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
          placeholder="请输入题目标题"
          disabled={submitted}
        />
      </div>

      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">
            比赛/赛事
          </label>
          <div className="space-y-2">
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="radio"
                name="contestMode"
                checked={contestMode === "none"}
                onChange={() => setContestMode("none")}
                disabled={submitted}
                className="accent-gray-800 dark:accent-gray-400"
              />
              <span className="text-gray-600 dark:text-gray-300">无</span>
            </label>
            {contests.length > 0 && (
              <label className="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  type="radio"
                  name="contestMode"
                  checked={contestMode === "select"}
                  onChange={() => setContestMode("select")}
                  disabled={submitted}
                  className="accent-gray-800 dark:accent-gray-400"
                />
                <span className="text-gray-600 dark:text-gray-300">
                  从已有比赛选择
                </span>
              </label>
            )}
            {contestMode === "select" && (
              <select
                value={selectedContestId}
                onChange={(e) => setSelectedContestId(e.target.value)}
                disabled={submitted}
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm ml-6 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
              >
                <option value="">-- 选择比赛 --</option>
                {contests.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            )}
            <label className="flex items-center gap-2 text-sm cursor-pointer">
              <input
                type="radio"
                name="contestMode"
                checked={contestMode === "custom"}
                onChange={() => setContestMode("custom")}
                disabled={submitted}
                className="accent-gray-800 dark:accent-gray-400"
              />
              <span className="text-gray-600 dark:text-gray-300">
                自行输入
              </span>
            </label>
            {contestMode === "custom" && (
              <input
                type="text"
                value={customContest}
                onChange={(e) => setCustomContest(e.target.value)}
                disabled={submitted}
                className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm ml-6 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
                placeholder="如：LeetCode周赛"
              />
            )}
          </div>
        </div>
        <div>
          <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">
            难度
          </label>
          <select
            value={formDifficulty}
            onChange={(e) => setFormDifficulty(e.target.value as Difficulty)}
            disabled={submitted}
            className="w-full px-4 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
          >
            {difficulties.map((d) => (
              <option key={d.name} value={d.name}>
                {d.label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="mb-6">
        <MarkdownEditor
          value={formContent}
          onChange={setFormContent}
          label="题目内容 (Markdown)"
          placeholder="# 题目描述&#10;&#10;请在这里编写题目..."
          disabled={submitted}
          required
          rows={30}
        />
      </div>

      <div className="mb-6">
        <MarkdownEditor
          value={formSolution}
          onChange={setFormSolution}
          label="题解 (Markdown)"
          optionalNote="可选"
          placeholder="# 题解&#10;&#10;请在这里编写题解（可选）..."
          disabled={submitted}
          rows={30}
        />
      </div>

      <div className="mb-6">
        <label className="block text-sm font-medium mb-2 text-gray-700 dark:text-gray-200">
          备注（仅审核阶段可见）
        </label>
        <textarea
          value={formRemark}
          onChange={(e) => setFormRemark(e.target.value)}
          rows={3}
          disabled={submitted}
          className="w-full px-3 py-2 border border-gray-300 bg-white focus:outline-none focus:border-gray-500 text-sm dark:border-gray-700 dark:bg-gray-800 dark:focus:border-gray-400"
          placeholder="给审核员的备注，审核通过后自动隐藏..."
        />
      </div>

      <button
        type="submit"
        disabled={submitted}
        className="px-6 py-3 bg-gray-800 text-white font-medium border border-gray-900 hover:bg-gray-700 disabled:opacity-50 dark:bg-gray-700 dark:border-gray-600 dark:hover:bg-gray-600"
      >
        {submitted ? "提交成功!" : "提交题目"}
      </button>
    </form>
  );
}
