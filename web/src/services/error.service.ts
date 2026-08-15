// ============== 错误中心 API ==============

import { apiFetch } from "./api";

export type ErrorReportStatus = "open" | "investigating" | "resolved" | "ignored";

export interface ErrorReportRow {
  id: string;
  ts: string;
  user_id: string | null;
  source: string;
  code: string;
  message: string;
  hint: string;
  suggestion: string;
  stack: string;
  url: string;
  route: string;
  method: string;
  http_status: number | null;
  ua: string;
  plugin_id: string;
  count: number;
  status: ErrorReportStatus;
  resolved_by: string | null;
  resolved_at: string | null;
  first_seen: string;
  last_seen: string;
}

export interface ErrorListResponse {
  success: boolean;
  errors: ErrorReportRow[];
  total: number;
}

export function fetchErrorReports(params?: {
  status?: string;
  code?: string;
  source?: string;
}): Promise<ErrorListResponse> {
  const qs = new URLSearchParams();
  if (params?.status) qs.set("status", params.status);
  if (params?.code) qs.set("code", params.code);
  if (params?.source) qs.set("source", params.source);
  const query = qs.toString();
  return apiFetch<ErrorListResponse>(`/errors${query ? `?${query}` : ""}`);
}

export function updateErrorStatus(
  id: string,
  status: ErrorReportStatus,
): Promise<{ success: boolean; message: string }> {
  return apiFetch(`/errors/${encodeURIComponent(id)}`, {
    method: "PATCH",
    body: JSON.stringify({ status }),
  });
}

export function deleteError(
  id: string,
): Promise<{ success: boolean; message: string }> {
  return apiFetch(`/errors/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export function clearErrors(): Promise<{ success: boolean; message: string }> {
  return apiFetch("/errors", { method: "DELETE" });
}
