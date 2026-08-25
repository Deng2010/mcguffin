import type { JoinRequest, TeamMember } from "../types";
import type { ActionResult } from "./admin.service";
import { apiFetch } from "./api";

export async function getMembers(): Promise<TeamMember[]> {
  return apiFetch<TeamMember[]>("/team/members");
}

export async function getRequests(): Promise<JoinRequest[]> {
  return apiFetch<JoinRequest[]>("/team/requests");
}

export async function reviewRequest(
  requestId: string,
  action: "approve" | "reject",
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/team/review/${requestId}/${action}`, {
    method: "POST",
  });
}

export async function changeMemberRole(
  userId: string,
  role: string,
): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/team/members/role/${userId}`, {
    method: "POST",
    body: JSON.stringify({ role }),
  });
}

export async function removeMember(userId: string): Promise<ActionResult> {
  return apiFetch<ActionResult>(`/team/members/remove/${userId}`, {
    method: "POST",
  });
}

export async function applyToJoin(reason: string): Promise<ActionResult> {
  return apiFetch<ActionResult>("/team/apply", {
    method: "POST",
    body: JSON.stringify({ reason }),
  });
}
