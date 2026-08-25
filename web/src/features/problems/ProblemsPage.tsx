import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAuthStore } from "../../stores/authStore";
import {
  getAdminMembers,
  setProblemVisibility,
} from "../../services/problem.service";
import { getContests } from "../../services/contest.service";
import { useDifficulties } from "../../hooks/useDifficulties";
import type { ProblemListItem } from "../../types";
import {
  useProblemFilters,
  useProblemLists,
  useProblems,
} from "./hooks/useProblems";
import { useSubmitForm } from "./hooks/useSubmitForm";
import { useProblemActions } from "./hooks/useProblemActions";
import FilterBar from "./components/FilterBar";
import SubmitProblemForm, {
  type ContestMode,
} from "./components/SubmitProblemForm";
import ReasonDialog from "./components/ReasonDialog";
import ProblemCard, { cardClass } from "./components/ProblemCard";

interface TeamMemberOption {
  user_id: string;
  name: string;
}

interface ContestOption {
  id: string;
  name: string;
}

type TabId =
  "list" | "mine" | "pending" | "approved" | "published" | "returned";

export default function ProblemsPage() {
  const { user, hasPermission, isAuthenticated } = useAuthStore();
  const { difficultyMap, difficulties } = useDifficulties();
  const navigate = useNavigate();
  const isGuest = !isAuthenticated || user?.role === "guest";
  const canApprove = hasPermission("approve_all_problems");
  const canSubmit = hasPermission("submit_problem");
  const canViewPending = hasPermission("view_pending_problems");
  const canViewApproved = hasPermission("view_approved_problems");
  const canViewPublic = hasPermission("view_public_problems");

  // ====== Data ======
  const { problems, loading, loadProblems } = useProblems(canApprove);
  const [members, setMembers] = useState<TeamMemberOption[]>([]);
  const [contests, setContests] = useState<ContestOption[]>([]);
  const [visibilityMap, setVisibilityMap] = useState<Record<string, string[]>>(
    {},
  );
  const [activeTab, setActiveTab] = useState<TabId>("list");

  // ====== Search & filter state ======
  const [searchText, setSearchText] = useState("");
  const [filterDifficulty, setFilterDifficulty] = useState("");
  const [filterAuthor, setFilterAuthor] = useState("");

  // ====== Derived lists ======
  const lists = useProblemLists(problems, user?.id, user?.display_name);
  const filteredProblems = useProblemFilters(
    problems,
    searchText,
    filterDifficulty,
    filterAuthor,
  );

  // 拥有“浏览所有待审核题目”权限时展示全部；仅拥有投稿权限时只展示自己提交的题目。
  const visiblePendingList = canViewPending
    ? lists.pendingList
    : ((lists as any).ownPendingList ?? lists.pendingList);
  const visibleReturnedList = canViewPending
    ? lists.returnedList
    : lists.ownReturnedList;
  const pendingCount = visiblePendingList.length;
  const returnedCount = visibleReturnedList.length;
  const approvedCount = lists.approvedList.length;
  const publishedCount = lists.publishedList.length;

  const tabs: { id: TabId; label: string; count?: number }[] = [
    { id: "list", label: "全部题目", count: problems.length },
  ];
  if (user) {
    tabs.push({
      id: "mine",
      label: "我的题目",
      count: lists.myProblems.length,
    });
  }
  if (canViewPending || canSubmit) {
    tabs.push({ id: "pending", label: "待审核", count: pendingCount });
  }
  if (canViewApproved) {
    tabs.push({ id: "approved", label: "已通过", count: approvedCount });
  }
  if (canViewPublic) {
    tabs.push({ id: "published", label: "已发布", count: publishedCount });
  }
  if (canSubmit && returnedCount > 0) {
    tabs.push({ id: "returned", label: "已退回", count: returnedCount });
  }

  // Lazy load members/contests when admin opens the pending tab
  useEffect(() => {
    if (activeTab === "pending" && canApprove && members.length === 0) {
      Promise.all([
        getAdminMembers() as Promise<TeamMemberOption[]>,
        getContests() as Promise<ContestOption[]>,
      ])
        .then(([memberList, contestList]) => {
          setMembers(memberList);
          setContests(contestList);
        })
        .catch(() => {});
    }
  }, [activeTab, canApprove, members.length]);

  // Initialize visibilityMap from problems when problems change (admin only)
  useEffect(() => {
    if (canApprove && problems.length > 0) {
      const vm: Record<string, string[]> = {};
      problems.forEach((p) => {
        if (p.visible_to && p.visible_to.length > 0) {
          vm[p.id] = p.visible_to;
        }
      });
      setVisibilityMap((prev) => {
        // Only merge new entries, preserve user edits
        const merged = { ...prev, ...vm };
        return Object.keys(merged).length > Object.keys(prev).length
          ? merged
          : prev;
      });
    }
  }, [problems, canApprove]);

  // Load contests when submit form opens
  const [showSubmit, setShowSubmit] = useState(false);
  useEffect(() => {
    if (showSubmit) {
      getContests()
        .then((list) => setContests(list as ContestOption[]))
        .catch(() => {});
    }
  }, [showSubmit]);

  // ====== Actions ======
  const actions = useProblemActions(loadProblems);

  const submitForm = useSubmitForm({
    contests,
    onSubmitted: loadProblems,
  });

  const toggleMember = (problemId: string, userId: string) => {
    setVisibilityMap((prev) => {
      const current = prev[problemId] || [];
      const updated = current.includes(userId)
        ? current.filter((id) => id !== userId)
        : [...current, userId];
      return { ...prev, [problemId]: updated };
    });
  };

  const resetFilters = () => {
    setSearchText("");
    setFilterDifficulty("");
    setFilterAuthor("");
  };

  const goDetail = (problemId: string) => navigate(`/problems/${problemId}`);
  const isAuthorOf = (p: { author_name: string }) =>
    user?.display_name === p.author_name;

  // Visibility editor (for pending tab — admin only)
  const renderVisibilityEditor = (problemId: string) => {
    if (members.length === 0) return null;
    return (
      <div className="mb-2">
        <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-200 mb-2">
          可见性设置（选择可查看此题目的成员）
        </h4>
        <div className="flex flex-wrap gap-2 mb-2">
          {members.map((m) => (
            <label
              key={m.user_id}
              className="flex items-center gap-1.5 text-sm cursor-pointer"
            >
              <input
                type="checkbox"
                checked={(visibilityMap[problemId] || []).includes(m.user_id)}
                onChange={() => toggleMember(problemId, m.user_id)}
                className="accent-gray-800 dark:accent-gray-400"
              />
              {m.name}
            </label>
          ))}
        </div>
        <button
          onClick={() => actions.handleSetVisibility(problemId, visibilityMap)}
          className="text-xs px-3 py-1 border border-gray-300 text-gray-600 hover:bg-gray-100 dark:border-gray-700 dark:text-gray-300 dark:hover:bg-gray-800"
        >
          保存可见性
        </button>
      </div>
    );
  };

  const renderCard = (p: ProblemListItem, extraActions?: React.ReactNode) => (
    <ProblemCard
      key={p.id}
      p={p}
      isGuest={isGuest}
      isAuthor={isAuthorOf(p)}
      difficultyMap={difficultyMap as any}
      onGoDetail={goDetail}
      extraActions={extraActions}
    />
  );

  // ====== Tab: All Problems ======
  const renderProblemList = () => {
    if (loading)
      return (
        <div className="text-center py-12 text-gray-400 dark:text-gray-500">
          加载中...
        </div>
      );
    return (
      <>
        <FilterBar
          searchText={searchText}
          filterDifficulty={filterDifficulty}
          filterAuthor={filterAuthor}
          difficulties={difficulties as any}
          onSearchTextChange={setSearchText}
          onFilterDifficultyChange={setFilterDifficulty}
          onFilterAuthorChange={setFilterAuthor}
          onReset={resetFilters}
        />
        {filteredProblems.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">
            {searchText || filterDifficulty || filterAuthor
              ? "没有符合条件的题目"
              : "暂无题目"}
          </div>
        ) : (
          <div className="space-y-4">
            {filteredProblems.map((p) => renderCard(p))}
          </div>
        )}
      </>
    );
  };

  // ====== Tab: My Problems ======
  const renderMyProblems = () => {
    if (lists.myProblems.length === 0)
      return (
        <div className="text-gray-400 text-sm py-8 text-center dark:text-gray-500">
          暂无题目
        </div>
      );
    return (
      <>
        <FilterBar
          searchText={searchText}
          filterDifficulty={filterDifficulty}
          filterAuthor={filterAuthor}
          difficulties={difficulties as any}
          onSearchTextChange={setSearchText}
          onFilterDifficultyChange={setFilterDifficulty}
          onFilterAuthorChange={setFilterAuthor}
          onReset={resetFilters}
        />
        <div className="space-y-4">
          {lists.myProblems
            .filter((p) => {
              const q = searchText.toLowerCase().trim();
              if (q && !p.title.toLowerCase().includes(q)) return false;
              if (filterDifficulty && p.difficulty !== filterDifficulty)
                return false;
              return true;
            })
            .map((p) => renderCard(p))}
        </div>
      </>
    );
  };

  // ====== Tab: Pending ======
  const renderPending = () => {
    const items = visiblePendingList;
    if (items.length === 0)
      return (
        <div className="text-gray-400 text-sm py-8 text-center dark:text-gray-500">
          暂无待审核题目
        </div>
      );
    return (
      <div className="space-y-4">
        {items.map((p) => (
          <div key={p.id}>
            <div className={cardClass} onClick={() => goDetail(p.id)}>
              <div className="flex items-start justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="text-lg font-semibold text-gray-800 dark:text-gray-100 truncate">
                      {p.title}
                    </span>
                    {renderVisibilityEditor(p.id)}
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    );
  };

  // ====== Tab: Approved ======
  const renderApproved = () => {
    if (lists.approvedList.length === 0)
      return (
        <div className="text-gray-400 text-sm py-8 text-center dark:text-gray-500">
          暂无待发布题目
        </div>
      );
    return (
      <div className="space-y-4">
        {lists.approvedList.map((p) =>
          renderCard(
            p,
            <>
              {!isGuest &&
                canSubmit &&
                user?.id !== p.author_id &&
                !(p.verifiers || []).some((v) => v.user_id === user?.id) && (
                  <button
                    onClick={() => actions.handleClaim(p.id)}
                    className="px-3 py-1.5 text-xs bg-white border border-gray-300 text-gray-700 hover:bg-gray-100 dark:bg-gray-900 dark:border-gray-700 dark:text-gray-300 dark:hover:bg-gray-800"
                  >
                    认领验题
                  </button>
                )}
              {!isGuest &&
                canSubmit &&
                (p.verifiers || []).some((v) => v.user_id === user?.id) && (
                  <button
                    onClick={() => actions.handleUnclaim(p.id)}
                    className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
                  >
                    取消认领
                  </button>
                )}
              {canApprove && (
                <>
                  <button
                    onClick={() => actions.handleReview(p.id, "publish")}
                    className="px-3 py-1.5 text-xs bg-green-700 text-white hover:bg-green-600"
                  >
                    发布
                  </button>
                  <button
                    onClick={() => actions.openReasonDialog(p.id, "return")}
                    className="px-3 py-1.5 text-xs border border-yellow-500 text-yellow-700 hover:bg-yellow-50 dark:border-yellow-800 dark:text-yellow-300 dark:hover:bg-yellow-900/20"
                  >
                    退回
                  </button>
                  <button
                    onClick={() => actions.handleDelete(p.id, p.title)}
                    className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
                  >
                    删除
                  </button>
                </>
              )}
            </>,
          ),
        )}
      </div>
    );
  };

  // ====== Tab: Published ======
  const renderPublished = () => {
    if (lists.publishedList.length === 0)
      return (
        <div className="text-gray-400 text-sm py-8 text-center dark:text-gray-500">
          暂无已发布题目
        </div>
      );
    return (
      <div className="space-y-4">
        {lists.publishedList.map((p) =>
          renderCard(
            p,
            canApprove ? (
              <>
                <button
                  onClick={() => actions.handleReview(p.id, "unpublish")}
                  className="px-3 py-1.5 text-xs bg-orange-600 text-white hover:bg-orange-500"
                >
                  取消发布
                </button>
                <button
                  onClick={() => actions.handleDelete(p.id, p.title)}
                  className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
                >
                  删除
                </button>
              </>
            ) : undefined,
          ),
        )}
      </div>
    );
  };

  // ====== Tab: Returned (已退回 — author only) ======
  const renderReturned = () => {
    if (visibleReturnedList.length === 0)
      return (
        <div className="text-gray-400 text-sm py-8 text-center dark:text-gray-500">
          暂无已退回题目
        </div>
      );
    return (
      <div className="space-y-4">
        {visibleReturnedList.map((p) =>
          renderCard(
            p,
            p.author_id === user?.id ? (
              <>
                <button
                  onClick={() => actions.handleResubmit(p.id)}
                  className="px-3 py-1.5 text-xs bg-gray-800 text-white hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
                >
                  再次提交
                </button>
                <button
                  onClick={() => actions.handleDelete(p.id, p.title)}
                  className="px-3 py-1.5 text-xs border border-red-300 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
                >
                  删除
                </button>
              </>
            ) : undefined,
          ),
        )}
      </div>
    );
  };

  return (
    <div className="p-6 max-w-5xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
          题目{" "}
          {!isGuest && (
            <span className="text-sm font-normal text-gray-400 dark:text-gray-500">
              ({filteredProblems.length})
            </span>
          )}
        </h1>
        {!isGuest && canSubmit && !showSubmit && (
          <button
            onClick={() => setShowSubmit(true)}
            className="px-4 py-2 bg-gray-800 text-white text-sm font-medium hover:bg-gray-700 dark:bg-gray-700 dark:hover:bg-gray-600"
          >
            投稿题目
          </button>
        )}
        {showSubmit && (
          <button
            onClick={() => setShowSubmit(false)}
            className="px-4 py-2 border border-gray-300 text-gray-600 text-sm hover:bg-gray-100 dark:border-gray-700 dark:text-gray-300 dark:hover:bg-gray-800"
          >
            返回列表
          </button>
        )}
      </div>

      {showSubmit ? (
        <SubmitProblemForm
          canSubmit={canSubmit}
          teamStatus={user?.team_status}
          contests={contests}
          difficulties={difficulties as any}
          formTitle={submitForm.formTitle}
          contestMode={submitForm.contestMode}
          selectedContestId={submitForm.selectedContestId}
          customContest={submitForm.customContest}
          formDifficulty={submitForm.formDifficulty}
          formContent={submitForm.formContent}
          formSolution={submitForm.formSolution}
          formRemark={submitForm.formRemark}
          submitted={submitForm.submitted}
          formError={submitForm.formError}
          setFormTitle={submitForm.setFormTitle}
          setContestMode={submitForm.setContestMode}
          setSelectedContestId={submitForm.setSelectedContestId}
          setCustomContest={submitForm.setCustomContest}
          setFormDifficulty={submitForm.setFormDifficulty}
          setFormContent={submitForm.setFormContent}
          setFormSolution={submitForm.setFormSolution}
          setFormRemark={submitForm.setFormRemark}
          onSubmit={submitForm.handleSubmitProblem}
        />
      ) : (
        <>
          {/* Tabs */}
          <div className="flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`px-4 py-2.5 text-sm font-medium border-b-2 transition-colors ${
                  activeTab === tab.id
                    ? "border-gray-800 text-gray-900 dark:border-gray-100 dark:text-gray-100"
                    : "border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-100"
                }`}
              >
                {tab.label}
                {tab.count !== undefined && (
                  <span
                    className={`mg-tab-count ${
                      activeTab === tab.id
                        ? "bg-gray-800 text-white dark:bg-gray-600"
                        : "bg-gray-200 text-gray-600 dark:bg-gray-700 dark:text-gray-300"
                    }`}
                  >
                    {tab.count}
                  </span>
                )}
              </button>
            ))}
          </div>

          {/* Tab content */}
          {activeTab === "list" && renderProblemList()}
          {activeTab === "mine" && renderMyProblems()}
          {activeTab === "pending" && renderPending()}
          {activeTab === "approved" && renderApproved()}
          {activeTab === "published" && renderPublished()}
          {activeTab === "returned" && renderReturned()}
        </>
      )}

      {/* Reason dialog for return/reject/reply */}
      {actions.reasonDialog && actions.reasonDialog.open && (
        <ReasonDialog
          dialog={actions.reasonDialog}
          reasonText={actions.reasonText}
          onReasonTextChange={actions.setReasonText}
          onCancel={() => actions.setReasonDialog(null)}
          onConfirm={actions.handleReviewWithReason}
        />
      )}
    </div>
  );
}
