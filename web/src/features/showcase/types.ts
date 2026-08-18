import type { Announcement } from "../../types";
import type { DifficultyInfo } from "../../hooks/useDifficulties";

// ============================================================
// 展板（Showcase）组件化布局 —— 数据模型
//
// 布局是一个「有序组件列表」，每个组件携带：
//   - settings  组件设置（结构由 registry 中的组件定义声明）
//   - size      大小（宽度 = 栅格列数 1..4，高度 = 最小 px）
//   - position  位置（流式顺序 order；未来可扩展显式行列）
//
// 该布局整体在后端以 opaque JSON 持久化（meta 表 showcase_layout，
// 经 /site/info 与 /admin/showcase/layout 读写），schema 由前端
// 定义并归一化，新增组件类型无需改动后端。
// ============================================================

export const SHOWCASE_LAYOUT_SCHEMA_VERSION = 1;

/** 展板栅格总列数（lg 及以上屏幕生效，小屏自动单列堆叠） */
export const SHOWCASE_GRID_COLUMNS = 4;

export type ShowcaseComponentType =
  | "intro"
  | "announcements"
  | "problems"
  | "contests";

export interface ShowcaseComponentSize {
  /** 宽度（栅格列数，1..4；4 = 整行） */
  width: number;
  /** 最小高度（px）；0 / 缺省 = 内容自适应 */
  height?: number;
}

export interface ShowcaseComponentPosition {
  /** 展板流式顺序（与 components 数组序号一致） */
  order: number;
}

export interface ShowcaseComponentConfig {
  /** 组件实例 id（内置组件 = type；未来扩展组件可用任意唯一字符串） */
  id: string;
  type: ShowcaseComponentType;
  /** 是否在展板中展示 */
  enabled: boolean;
  /** 组件设置（结构由 registry 的组件定义声明，加载时归一化） */
  settings: Record<string, unknown>;
  size: ShowcaseComponentSize;
  position: ShowcaseComponentPosition;
}

export interface ShowcaseLayout {
  schema_version: number;
  components: ShowcaseComponentConfig[];
}

// ============== 展板数据项（接口返回列表中的最小字段） ==============

export interface ShowcaseProblem {
  id: string;
  title: string;
  author_name: string;
  contest: string;
  contest_id?: string | null;
  difficulty: string;
  status: string;
  created_at: string;
  link?: string | null;
}

export interface ShowcaseContest {
  id: string;
  name: string;
  start_time: string;
  end_time: string;
  description: string;
  created_by: string;
  created_at: string;
  status: string;
  link?: string | null;
  problem_order: string[];
}

// ============== 组件上下文（页面 → 组件的共享数据契约） ==============

/** 组件需要的站点信息最小子集（来自 siteStore，结构兼容） */
export interface ShowcaseSiteInfo {
  name: string;
  description: string;
  timezone?: string;
  server_time?: number;
}

export interface ShowcaseContext {
  siteInfo: ShowcaseSiteInfo | null;
  announcements: Announcement[];
  /** 已发布的题目（展示前再按组件设置过滤） */
  problems: ShowcaseProblem[];
  /** 公开的比赛 */
  contests: ShowcaseContest[];
  difficultyMap: Map<string, DifficultyInfo>;
  /** 管理员（edit_showcase） */
  isAdmin: boolean;
  /** 是否有权进入管理后台（用于公告空态引导） */
  canAccessAdmin: boolean;
  getServerNow: () => number | null;
}

export interface ShowcaseComponentProps {
  config: ShowcaseComponentConfig;
  ctx: ShowcaseContext;
}