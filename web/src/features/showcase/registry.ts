import type { ComponentType } from "react";
import type {
  ShowcaseComponentConfig,
  ShowcaseComponentProps,
  ShowcaseComponentType,
  ShowcaseLayout,
} from "./types";
import { SHOWCASE_LAYOUT_SCHEMA_VERSION, SHOWCASE_GRID_COLUMNS } from "./types";
import IntroComponent from "./components/IntroComponent";
import AnnouncementsComponent from "./components/AnnouncementsComponent";
import ProblemsComponent from "./components/ProblemsComponent";
import ContestsComponent from "./components/ContestsComponent";

// ============================================================
// 展板组件注册表
//
// 每个内置组件在此声明：
//   - label / description   展示与管理面板中的元信息
//   - defaultSettings       默认设置
//   - fields                设置面板字段（schema 驱动，自动生成编辑器）
//   - component             渲染实现
//
// 「新增一个展板组件」= 实现展示组件 → 在此注册 → 无需改动后端。
// 详见 docs/guide/showcase-components.md。
// ============================================================

export type SettingField =
  | {
      kind: "number";
      key: string;
      label: string;
      min?: number;
      max?: number;
      step?: number;
    }
  | { kind: "boolean"; key: string; label: string }
  | {
      kind: "ids";
      key: string;
      label: string;
      /** 候选项数据源（ctx.problems / ctx.contests） */
      source: "problems" | "contests";
    };

export interface ShowcaseComponentDef {
  type: ShowcaseComponentType;
  label: string;
  description: string;
  defaultSettings: () => Record<string, unknown>;
  fields: SettingField[];
  component: ComponentType<ShowcaseComponentProps>;
}

export const SHOWCASE_COMPONENT_DEFS: Record<
  ShowcaseComponentType,
  ShowcaseComponentDef
> = {
  intro: {
    type: "intro",
    label: "团队简介",
    description: "站点名称与团队介绍（Markdown）",
    defaultSettings: () => ({}),
    fields: [],
    component: IntroComponent,
  },
  announcements: {
    type: "announcements",
    label: "公告",
    description: "置顶公告与最新公告",
    defaultSettings: () => ({ showCount: 3 }),
    fields: [
      {
        kind: "number",
        key: "showCount",
        label: "展示条数",
        min: 1,
        max: 20,
        step: 1,
      },
    ],
    component: AnnouncementsComponent,
  },
  problems: {
    type: "problems",
    label: "公开题目",
    description: "已发布题目列表，可按题选题",
    defaultSettings: () => ({ selectedIds: [], showDifficulty: true }),
    fields: [
      {
        kind: "ids",
        key: "selectedIds",
        label: "展示题目（不选 = 全部展示）",
        source: "problems",
      },
      { kind: "boolean", key: "showDifficulty", label: "显示难度徽标" },
    ],
    component: ProblemsComponent,
  },
  contests: {
    type: "contests",
    label: "比赛",
    description: "公开比赛列表，可嵌套展示比赛题目",
    defaultSettings: () => ({
      selectedIds: [],
      showDescription: true,
      showProblems: true,
    }),
    fields: [
      {
        kind: "ids",
        key: "selectedIds",
        label: "展示比赛（不选 = 全部展示）",
        source: "contests",
      },
      { kind: "boolean", key: "showDescription", label: "显示比赛简介" },
      { kind: "boolean", key: "showProblems", label: "显示比赛题目" },
    ],
    component: ContestsComponent,
  },
};

// ============== 布局构建与归一化 ==============

/**
 * 默认布局：四个内置组件按 简介 → 公告 → 题目 → 比赛 排列，整行宽度。
 * 旧版 showcase_problem_ids / showcase_contest_ids 作为题目/比赛组件的
 * 默认选中项迁移进来（空数组 = 全部展示，语义与旧版一致）。
 */
export function createDefaultLayout(
  legacyProblemIds: string[],
  legacyContestIds: string[],
): ShowcaseLayout {
  const seed: [ShowcaseComponentType, Record<string, unknown>][] = [
    ["intro", {}],
    ["announcements", { showCount: 3 }],
    ["problems", { selectedIds: legacyProblemIds, showDifficulty: true }],
    [
      "contests",
      { selectedIds: legacyContestIds, showDescription: true, showProblems: true },
    ],
  ];
  return {
    schema_version: SHOWCASE_LAYOUT_SCHEMA_VERSION,
    components: seed.map(([type, settings], order) => ({
      id: type,
      type,
      enabled: true,
      settings,
      size: { width: SHOWCASE_GRID_COLUMNS },
      position: { order },
    })),
  };
}

/** 取宽度（1..4 收拢到合法范围）并补齐高度字段 */
function normalizeSize(raw: unknown): ShowcaseComponentConfig["size"] {
  const s =
    raw && typeof raw === "object"
      ? (raw as Record<string, unknown>)
      : ({} as Record<string, unknown>);
  const width =
    typeof s.width === "number"
      ? Math.min(SHOWCASE_GRID_COLUMNS, Math.max(1, Math.floor(s.width)))
      : SHOWCASE_GRID_COLUMNS;
  const height =
    typeof s.height === "number" && s.height >= 0 ? s.height : undefined;
  return height ? { width, height } : { width };
}

/**
 * 把后端返回（或本地编辑、或旧版数据）的任意布局归一化为合法布局：
 *  - 缺失字段补默认值；
 *  - 未知组件类型保留（供未来扩展向前兼容，渲染时跳过）；
 *  - 内置组件缺失时补默认；
 *  - 按 order 排序并重刷序号。
 */
export function normalizeShowcaseLayout(
  raw: unknown,
  legacyProblemIds: string[],
  legacyContestIds: string[],
): ShowcaseLayout {
  const idx = (v: unknown, i: number, fallback: number) =>
    v && typeof v === "object" && typeof (v as Record<string, unknown>).order === "number"
      ? ((v as Record<string, unknown>).order as number)
      : fallback;

  const fallback = createDefaultLayout(legacyProblemIds, legacyContestIds);
  if (!raw || typeof raw !== "object") return fallback;
  const rawComponents = (raw as Record<string, unknown>).components;
  if (!Array.isArray(rawComponents)) return fallback;

  const seen = new Set<string>();
  const components: ShowcaseComponentConfig[] = [];

  rawComponents.forEach((c, i) => {
    if (!c || typeof c !== "object") return;
    const rec = c as Record<string, unknown>;
    const type = typeof rec.type === "string" ? (rec.type as ShowcaseComponentType) : null;
    const id =
      typeof rec.id === "string" && rec.id ? rec.id : type;
    if (!type || !id || seen.has(id)) return;
    seen.add(id);

    const def = SHOWCASE_COMPONENT_DEFS[type];
    const settings: Record<string, unknown> = def ? def.defaultSettings() : {};
    if (rec.settings && typeof rec.settings === "object") {
      for (const [k, v] of Object.entries(rec.settings as Record<string, unknown>)) {
        settings[k] = v;
      }
    }

    components.push({
      id,
      type,
      enabled: rec.enabled !== false,
      settings,
      size: normalizeSize(rec.size),
      position: { order: idx(rec.position, i, i) },
    });
  });

  // 内置组件缺失时补默认（向前兼容旧布局）
  for (const def of Object.values(SHOWCASE_COMPONENT_DEFS)) {
    if (!components.some((c) => c.id === def.type)) {
      components.push({
        id: def.type,
        type: def.type,
        enabled: true,
        settings: def.defaultSettings(),
        size: { width: SHOWCASE_GRID_COLUMNS },
        position: { order: components.length },
      });
    }
  }

  components.sort((a, b) => a.position.order - b.position.order);
  components.forEach((c, i) => {
    c.position.order = i;
  });

  return { schema_version: SHOWCASE_LAYOUT_SCHEMA_VERSION, components };
}

// ============== 获取类型化设置的小工具 ==============

export function getAnnouncementsSettings(config: ShowcaseComponentConfig): {
  showCount: number;
} {
  const s = config.settings;
  return {
    showCount:
      typeof s.showCount === "number" && s.showCount >= 1
        ? Math.floor(s.showCount)
        : 3,
  };
}

export function getProblemsSettings(config: ShowcaseComponentConfig): {
  selectedIds: string[];
  showDifficulty: boolean;
} {
  const s = config.settings;
  return {
    selectedIds: Array.isArray(s.selectedIds)
      ? (s.selectedIds as string[]).filter((x) => typeof x === "string")
      : [],
    showDifficulty: s.showDifficulty !== false,
  };
}

export function getContestsSettings(config: ShowcaseComponentConfig): {
  selectedIds: string[];
  showDescription: boolean;
  showProblems: boolean;
} {
  const s = config.settings;
  return {
    selectedIds: Array.isArray(s.selectedIds)
      ? (s.selectedIds as string[]).filter((x) => typeof x === "string")
      : [],
    showDescription: s.showDescription !== false,
    showProblems: s.showProblems !== false,
  };
}