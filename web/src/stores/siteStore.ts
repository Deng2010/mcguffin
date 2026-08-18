import { create } from "zustand";
import { apiFetch } from "../services/api";
import { updateSiteDescription, updateShowcaseLayout } from "../services/site.service";
import type { ShowcaseLayout } from "../features/showcase/types";

// ============== SiteInfo Type ==============

export interface SiteInfo {
  name: string;
  version: string;
  description: string;
  title: string;
  difficulty_order: string[];
  showcase_problem_ids: string[];
  showcase_contest_ids: string[];
  /** 展板组件化布局（可能为 null = 未配置，前端回退默认布局） */
  showcase_layout?: ShowcaseLayout | null;
  timezone: string;
  server_time: number;
}

// ============== State & Actions ==============

interface SiteState {
  siteInfo: SiteInfo | null;
  /** 记录最近一次拿到 server_time 的客户端时间，用于估算当前服务器时间 */
  serverTimeFetchedAt: number;
  getServerNow: () => number | null;
  updateDescription: (
    description: string,
  ) => Promise<{ success: boolean; message: string }>;
  /** 保存展板组件化布局（PUT /admin/showcase/layout） */
  saveShowcaseLayout: (
    layout: ShowcaseLayout,
  ) => Promise<{ success: boolean; message: string }>;
  refresh: () => Promise<void>;
}

const DEFAULT_SITE_INFO: SiteInfo = {
  name: "McGuffin",
  version: "0.1.0",
  description: "",
  title: "McGuffin",
  difficulty_order: [],
  showcase_problem_ids: [],
  showcase_contest_ids: [],
  showcase_layout: null,
  timezone: "UTC+8",
  server_time: Date.now(),
};

export const DEFAULT_TIMEZONE = "UTC+8";

export const useSiteStore = create<SiteState>()((set, get) => ({
  siteInfo: null,
  serverTimeFetchedAt: 0,

  getServerNow: () => {
    const { siteInfo, serverTimeFetchedAt } = get();
    if (!siteInfo || !serverTimeFetchedAt) return null;
    return siteInfo.server_time + (Date.now() - serverTimeFetchedAt);
  },

  refresh: async () => {
    try {
      const info = await apiFetch<SiteInfo>("/site/info");
      set({ siteInfo: info, serverTimeFetchedAt: Date.now() });
      document.title = info.title || "McGuffin";
    } catch {
      set({
        siteInfo: DEFAULT_SITE_INFO,
        serverTimeFetchedAt: Date.now(),
      });
      document.title = "McGuffin";
    }
  },

  updateDescription: async (description) => {
    try {
      const res = await updateSiteDescription(description);
      const { siteInfo } = get();
      if (res.success && siteInfo) {
        set({ siteInfo: { ...siteInfo, description: res.description } });
      }
      return { success: res.success, message: res.message };
    } catch (err) {
      return { success: false, message: `请求失败: ${err}` };
    }
  },

  saveShowcaseLayout: async (layout) => {
    try {
      const res = await updateShowcaseLayout(layout);
      const { siteInfo } = get();
      if (res.success && siteInfo) {
        set({ siteInfo: { ...siteInfo, showcase_layout: layout } });
      }
      return { success: res.success, message: res.message };
    } catch (err) {
      return { success: false, message: `请求失败: ${err}` };
    }
  },
}));

// ============== Initialization ==============

export function initSite() {
  useSiteStore.getState().refresh();
}