import { apiFetch } from "./api";
import type { ShowcaseLayout } from "../features/showcase/types";

export interface SiteInfo {
  name: string;
  description: string;
  version: string;
  title?: string;
  timezone?: string;
  server_time?: number;
  showcase?: {
    problems: Record<string, any>[];
    contests: Record<string, any>[];
  };
  /** 展板组件化布局（可能为 null = 未配置，前端回退默认布局） */
  showcase_layout?: ShowcaseLayout | null;
}

export interface ShowcasePayload {
  problem_ids: string[];
  contest_ids: string[];
}

export async function getSiteInfo(): Promise<SiteInfo> {
  return apiFetch<SiteInfo>("/site/info");
}

export async function updateSiteDescription(
  description: string,
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>("/site/description", {
    method: "PUT",
    body: JSON.stringify({ description }),
  });
}

export async function updateShowcase(
  problemIds: string[],
  contestIds: string[],
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>("/admin/showcase", {
    method: "PUT",
    body: JSON.stringify({ problem_ids: problemIds, contest_ids: contestIds }),
  });
}

export async function updateShowcaseLayout(
  layout: ShowcaseLayout,
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>("/admin/showcase/layout", {
    method: "PUT",
    body: JSON.stringify({ layout }),
  });
}
