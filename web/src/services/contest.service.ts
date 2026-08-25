import { apiFetch } from "./api";
import type { ActionResult } from "./admin.service";

export interface ContestPayload {
  name: string;
  description?: string;
  start_time?: string | null;
  end_time?: string | null;
  link?: string | null;
}

export interface Contest {
  id: string;
  name: string;
  description?: string;
  status: string;
  start_time?: string | null;
  end_time?: string | null;
  link?: string | null;
  created_at?: string;
  updated_at?: string;
}

export async function getContests(): Promise<Contest[]> {
  return apiFetch<Contest[]>("/contests");
}

export async function createContest(
  body: ContestPayload,
): Promise<Contest & ActionResult> {
  return apiFetch<Contest & ActionResult>("/contests", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function deleteContest(id: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/contests/${id}`, { method: "DELETE" });
}

export async function getContestProblems(
  id: string,
): Promise<Record<string, any>[]> {
  return apiFetch<Record<string, any>[]>(`/contests/${id}/problems`);
}

export async function updateContest(
  id: string,
  body: ContestPayload,
): Promise<Contest & ActionResult> {
  return apiFetch<Contest & ActionResult>(`/contests/${id}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
}

export async function setProblemOrder(
  id: string,
  problemIds: string[],
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/contests/${id}/problem-order`, {
    method: "POST",
    body: JSON.stringify({ problem_ids: problemIds }),
  });
}

export async function setContestStatus(
  id: string,
  status: string,
  link?: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/contests/${id}/status`, {
    method: "POST",
    body: JSON.stringify({ status, link }),
  });
}
