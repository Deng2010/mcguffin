import type {
  AdminPendingProblem,
  ProblemDetail,
  ProblemListItem,
  SubmitProblemPayload,
} from "../types";
import type { ActionResult } from "./admin.service";
import { apiFetch } from "./api";

export interface TeamMemberOption {
  user_id: string;
  name: string;
}

export async function getProblems(all?: boolean): Promise<ProblemListItem[]> {
  const path = all ? "/problems?all=true" : "/problems";
  return apiFetch<ProblemListItem[]>(path);
}

export async function getProblemDetail(id: string): Promise<ProblemDetail> {
  return apiFetch<ProblemDetail>(`/problems/detail/${id}`);
}

export async function getAdminMembers(): Promise<TeamMemberOption[]> {
  return apiFetch<TeamMemberOption[]>("/problems/admin/members");
}

export async function createProblem(
  body: SubmitProblemPayload,
): Promise<ActionResult> {
  return apiFetch<ActionResult>("/problems", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function updateProblem(
  id: string,
  body: Partial<SubmitProblemPayload> & Record<string, any>,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/${id}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
}

export async function deleteProblem(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/${id}`, { method: "DELETE" });
}

export async function claimProblem(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/claim/${id}`, {
    method: "POST",
  });
}

export async function unclaimProblem(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/unclaim/${id}`, {
    method: "POST",
  });
}

export async function reviewProblem(
  id: string,
  action: "approve" | "reply" | "publish" | "return" | "unpublish",
  reason?: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/review/${id}/${action}`, {
    method: "POST",
    body: reason ? JSON.stringify({ reason }) : undefined,
  });
}

export async function setProblemVisibility(
  id: string,
  userIds: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/visibility/${id}`, {
    method: "POST",
    body: JSON.stringify({ visible_to: userIds }),
  });
}

export async function setProblemContest(
  id: string,
  contestId: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/contest/${id}`, {
    method: "POST",
    body: JSON.stringify({ contest_id: contestId }),
  });
}

export async function resubmitProblem(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/${id}/resubmit`, {
    method: "POST",
  });
}

export async function submitVerifierComment(
  id: string,
  content: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/verifier-comment/${id}`, {
    method: "POST",
    body: JSON.stringify({ content }),
  });
}

export async function submitVerifierSolution(
  id: string,
  solution: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/problems/verifier-solution/${id}`, {
    method: "POST",
    body: JSON.stringify({ solution }),
  });
}
