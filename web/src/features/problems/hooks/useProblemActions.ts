import { useState } from "react";
import {
  claimProblem,
  deleteProblem,
  resubmitProblem,
  reviewProblem,
  setProblemContest,
  setProblemVisibility,
  unclaimProblem,
} from "../../../services/problem.service";
import { useToast } from "../../../errors/ToastContext";
import { errorMessage } from "../../../errors/normalize";

type ReviewAction = "approve" | "reply" | "publish" | "return" | "unpublish";

export function useProblemActions(loadProblems: () => void) {
  const toast = useToast();
  // Reason dialog for return/reply
  const [reasonDialog, setReasonDialog] = useState<{
    open: boolean;
    problemId: string;
    action: string;
  } | null>(null);
  const [reasonText, setReasonText] = useState("");

  const handleClaim = async (problemId: string) => {
    try {
      const res = await claimProblem(problemId);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      loadProblems();
    } catch (err) {
      toast.error(`认领失败: ${errorMessage(err)}`);
    }
  };

  const handleUnclaim = async (problemId: string) => {
    try {
      const res = await unclaimProblem(problemId);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      loadProblems();
    } catch (err) {
      toast.error(`取消认领失败: ${errorMessage(err)}`);
    }
  };

  const handleReview = async (
    problemId: string,
    action: string,
    reason?: string,
  ) => {
    try {
      const res = await reviewProblem(problemId, action as ReviewAction, reason);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      loadProblems();
    } catch (err) {
      toast.error(`操作失败: ${errorMessage(err)}`);
    }
  };

  // Open reason dialog for return/reject/reply
  const openReasonDialog = (problemId: string, action: string) => {
    setReasonDialog({ open: true, problemId, action });
    setReasonText("");
  };

  const handleResubmit = async (problemId: string) => {
    try {
      const res = await resubmitProblem(problemId);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      toast.success("已重新提交");
      loadProblems();
    } catch (err) {
      toast.error(`再次提交失败: ${errorMessage(err)}`);
    }
  };

  // Submit with reason
  const handleReviewWithReason = async () => {
    if (!reasonDialog) return;
    const { problemId, action } = reasonDialog;
    const trimmed = reasonText.trim();
    if (action === "reject" && trimmed.length < 10) {
      toast.error("退回理由不能少于 10 个字");
      return;
    }
    await handleReview(problemId, action, trimmed || undefined);
    setReasonDialog(null);
    setReasonText("");
  };

  const handleSetVisibility = async (
    problemId: string,
    visibilityMap: Record<string, string[]>,
  ) => {
    const ids = visibilityMap[problemId] || [];
    try {
      const res = await setProblemVisibility(problemId, ids);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      toast.success("可见性已更新");
    } catch (err) {
      toast.error(`设置失败: ${errorMessage(err)}`);
    }
  };

  const handleSetContest = async (problemId: string, contestId: string) => {
    try {
      const res = await setProblemContest(problemId, contestId);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      loadProblems();
    } catch (err) {
      toast.error(`设置失败: ${errorMessage(err)}`);
    }
  };

  const handleDelete = async (problemId: string, title: string) => {
    if (!window.confirm(`确定要永久删除题目「${title}」吗？此操作不可撤销。`))
      return;
    try {
      const res = await deleteProblem(problemId);
      if (!res.success) {
        toast.error(res.message);
        return;
      }
      loadProblems();
    } catch (err) {
      toast.error(`删除失败: ${errorMessage(err)}`);
    }
  };

  return {
    reasonDialog,
    setReasonDialog,
    reasonText,
    setReasonText,
    openReasonDialog,
    handleReviewWithReason,
    handleClaim,
    handleUnclaim,
    handleReview,
    handleResubmit,
    handleSetVisibility,
    handleSetContest,
    handleDelete,
  };
}
