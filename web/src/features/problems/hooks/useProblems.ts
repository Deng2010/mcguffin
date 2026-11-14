import { useEffect, useMemo, useState } from "react";
import { getProblems } from "../../../services/problem.service";
import type { ProblemListItem } from "../../../types";

export function useProblems(canApprove: boolean) {
  const [problems, setProblems] = useState<ProblemListItem[]>([]);
  const [loading, setLoading] = useState(true);

  const loadProblems = () => {
    getProblems(canApprove)
      .then(setProblems)
      .catch(() => setProblems([]))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadProblems();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [canApprove]);

  return { problems, loading, loadProblems };
}

// 按状态拆分的列表（基于已加载的问题列表）
// 归属判断一律使用 author_id：显示名称（author_name）只是快照，用户改名后
// 不应再作为「我的题目」的依据。
export function useProblemLists(problems: ProblemListItem[], userId?: string) {
  return useMemo(() => {
    const myProblems =
      userId != null ? problems.filter((p) => p.author_id === userId) : [];
    const pendingList = problems.filter((p) => p.status === "pending");
    const ownPendingList = pendingList.filter(
      (p) => userId != null && p.author_id === userId,
    );
    const approvedList = problems.filter((p) => p.status === "approved");
    const publishedList = problems.filter((p) => p.status === "published");
    const returnedList = problems.filter((p) => p.status === "returned");
    const ownReturnedList = returnedList.filter(
      (p) => userId != null && p.author_id === userId,
    );
    return {
      myProblems,
      pendingList,
      ownPendingList,
      approvedList,
      publishedList,
      returnedList,
      ownReturnedList,
    };
  }, [problems, userId]);
}

// Client-side filtering
export function useProblemFilters(
  problems: ProblemListItem[],
  searchText: string,
  filterDifficulty: string,
  filterAuthor: string,
) {
  return useMemo(() => {
    const q = searchText.toLowerCase().trim();
    const a = filterAuthor.toLowerCase().trim();
    return problems.filter((p) => {
      if (q && !p.title.toLowerCase().includes(q)) return false;
      if (filterDifficulty && p.difficulty !== filterDifficulty) return false;
      if (a && !p.author_name.toLowerCase().includes(a)) return false;
      return true;
    });
  }, [problems, searchText, filterDifficulty, filterAuthor]);
}
