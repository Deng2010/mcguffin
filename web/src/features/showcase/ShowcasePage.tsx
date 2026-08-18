import { useState, useEffect } from "react";
import { useAuthStore } from "../../stores/authStore";
import { useSiteStore } from "../../stores/siteStore";
import { useDifficulties } from "../../hooks/useDifficulties";
import { useToast } from "../../errors/ToastContext";
import { apiFetch } from "../../services/api";
import type { Announcement } from "../../types";
import {
  createDefaultLayout,
  normalizeShowcaseLayout,
} from "./registry";
import ShowcaseBoard from "./ShowcaseBoard";
import ShowcaseSettingsPanel from "./ShowcaseSettingsPanel";
import type {
  ShowcaseContext,
  ShowcaseContest,
  ShowcaseLayout,
  ShowcaseProblem,
} from "./types";

/**
 * 展板页面：数据加载 + 布局状态管理。
 *
 * 页面不再关心「怎么展示」，只负责：
 *  1. 拉取题目 / 比赛 / 公告等数据并组装 ShowcaseContext；
 *  2. 维护 ShowcaseLayout（来自 siteInfo.showcase_layout，缺失时按旧版
 *     showcase_problem_ids / showcase_contest_ids 迁移默认布局）；
 *  3. 渲染 ShowcaseBoard（公开视图）与 ShowcaseSettingsPanel（管理视图）。
 *
 * 展板组件的架构 / 数据模型 / 扩展方式见 docs/guide/showcase-components.md。
 */
export default function ShowcasePage() {
  const { hasPermission } = useAuthStore();
  const toast = useToast();
  const {
    siteInfo,
    refresh: refreshSite,
    getServerNow,
    saveShowcaseLayout,
  } = useSiteStore();
  const { difficultyMap } = useDifficulties();

  const [allContests, setAllContests] = useState<ShowcaseContest[]>([]);
  const [allProblems, setAllProblems] = useState<ShowcaseProblem[]>([]);
  const [announcements, setAnnouncements] = useState<Announcement[]>([]);
  const [loading, setLoading] = useState(true);

  // —— 展板布局状态 ——
  const [layout, setLayout] = useState<ShowcaseLayout>(() =>
    createDefaultLayout([], []),
  );
  const [layoutReady, setLayoutReady] = useState(false);
  // —— 展板管理状态 ——
  const [showcaseMode, setShowcaseMode] = useState(false);
  const [showcaseMsg, setShowcaseMsg] = useState("");
  const [showcaseSaving, setShowcaseSaving] = useState(false);

  const isAdmin = hasPermission("edit_showcase");

  useEffect(() => {
    Promise.all([
      apiFetch<ShowcaseContest[]>("/contests").catch(() => [] as ShowcaseContest[]),
      apiFetch<ShowcaseProblem[]>("/problems").catch(() => [] as ShowcaseProblem[]),
      apiFetch<Announcement[]>("/announcements").catch(() => [] as Announcement[]),
    ])
      .then(([c, p, a]) => {
        setAllContests(c);
        setAllProblems(p);
        setAnnouncements(a);
      })
      .finally(() => setLoading(false));
  }, []);

  // siteInfo 就绪后（重新）归一化布局；非管理态下外部变化自动同步
  useEffect(() => {
    if (!siteInfo) return;
    setLayout(
      normalizeShowcaseLayout(
        siteInfo.showcase_layout ?? null,
        siteInfo.showcase_problem_ids ?? [],
        siteInfo.showcase_contest_ids ?? [],
      ),
    );
    setLayoutReady(true);
  }, [siteInfo]);

  const openShowcaseManage = () => {
    setShowcaseMode(true);
    setShowcaseMsg("");
  };

  const cancelShowcaseManage = () => {
    setShowcaseMode(false);
    if (siteInfo) {
      setLayout(
        normalizeShowcaseLayout(
          siteInfo.showcase_layout ?? null,
          siteInfo.showcase_problem_ids ?? [],
          siteInfo.showcase_contest_ids ?? [],
        ),
      );
    }
  };

  const handleSaveLayout = async () => {
    setShowcaseSaving(true);
    setShowcaseMsg("");
    const res = await saveShowcaseLayout(layout);
    if (!res.success) {
      setShowcaseMsg(`保存失败: ${res.message}`);
      setShowcaseSaving(false);
      toast.error(res.message);
      return;
    }
    setShowcaseMsg(res.message);
    setShowcaseMode(false);
    refreshSite();
    setShowcaseSaving(false);
  };

  // 展板展示 ONLY 公开比赛与已发布题目
  const contests = allContests.filter((c) => c.status === "public");
  const problems = allProblems.filter((p) => p.status === "published");

  const ctx: ShowcaseContext = {
    siteInfo,
    announcements,
    problems,
    contests,
    difficultyMap,
    isAdmin,
    canAccessAdmin: hasPermission("access_admin"),
    getServerNow,
  };

  if (loading)
    return (
      <div className="p-6 text-center py-12 text-gray-400 dark:text-gray-500">
        加载中...
      </div>
    );

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* 管理入口 */}
      {isAdmin && !showcaseMode && (
        <div className="flex justify-end">
          <button
            onClick={openShowcaseManage}
            className="text-xs px-3 py-1 border border-gray-300 dark:border-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800"
          >
            展板管理
          </button>
        </div>
      )}

      {/* 管理面板（编辑中实时预览下方展板） */}
      {showcaseMode && layoutReady && (
        <ShowcaseSettingsPanel
          layout={layout}
          onChange={setLayout}
          ctx={ctx}
          saving={showcaseSaving}
          msg={showcaseMsg}
          onSave={handleSaveLayout}
          onCancel={cancelShowcaseManage}
        />
      )}

      {/* 展板（组件化栅格） */}
      {layoutReady && <ShowcaseBoard layout={layout} ctx={ctx} />}
    </div>
  );
}