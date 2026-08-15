/**
 * 榜榜糖核心纯逻辑 — 无 React 依赖，便于推理与复用
 *
 * 约定：
 *  - 所有"天 / 周"均以北京时间（UTC+8）计算，与需求"每周一 0:00 北京时间"一致
 *  - KV 存储约定见 CandyPage.tsx 顶部的 namespace 常量
 */

/** 北京时间相对 UTC 的偏移（毫秒） */
export const BJ_OFFSET_MS = 8 * 60 * 60 * 1000;

// ── 概率参数（集中可调） ──

/** 糖数为 0 时的基础成功率 */
export const SUCCESS_BASE = 0.6;
/** 每多 1 颗糖，成功率衰减系数 */
export const SUCCESS_DECAY = 0.8;
/** 成功率下限，避免糖数很高时永远失败 */
export const SUCCESS_FLOOR = 0.05;

/** 每个用户每天可点出的糖次数上限 */
export const DAILY_LIMIT = 10;

/** 冠军两侧的棒棒糖 emoji */
export const CHAMPION_EMOJI = "🍭";

// ── 北京时间工具 ──

/** 当前北京时间对应的 Date（仅用于取 UTC 年月日/周，勿用于本地展示） */
export function bjDate(now: Date = new Date()): Date {
  return new Date(now.getTime() + BJ_OFFSET_MS);
}

/** 北京时间日期 key，形如 2026-02-02（用于每日次数计数，按天自然过期） */
export function bjDateKey(now: Date = new Date()): string {
  return bjDate(now).toISOString().slice(0, 10);
}

/**
 * ISO 周计算（给定日期取其 UTC 字段，见 bjDate 的平移语义）
 * 返回形如 "2026-W06" 的周标识
 */
export function isoWeekKey(date: Date): string {
  const d = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  // 将周对齐到"周一为一周开始"的 ISO 语义
  const dayNum = d.getUTCDay() || 7; // 周日=7，周一=1 ... 周六=6
  d.setUTCDate(d.getUTCDate() + 4 - dayNum); // 移到本周四
  const yearStart = new Date(Date.UTC(d.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((d.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
  return `${d.getUTCFullYear()}-W${String(week).padStart(2, "0")}`;
}

/** 当前"北京时间周"标识，形如 2026-W06 */
export function bjWeekKey(now: Date = new Date()): string {
  return isoWeekKey(bjDate(now));
}

// ── 点糖概率 ──

/**
 * 给已有 candies 颗糖的成员点糖的成功率。
 * 随糖数递减：p = base × decay^candies，且不小于 floor。
 */
export function successProbability(candies: number): number {
  const c = Math.max(0, candies);
  const p = SUCCESS_BASE * Math.pow(SUCCESS_DECAY, c);
  return Math.max(SUCCESS_FLOOR, Math.min(1, p));
}

/** 成功率格式化（用于界面展示，如 "48%"） */
export function formatProbability(p: number): string {
  return `${Math.round(p * 100)}%`;
}

// ── 周结算 ──

/** 结算所需的数据读接口（由调用方注入，便于测试与解耦） */
export interface CandyReader {
  getCandies(userId: string): Promise<number>;
}

/** 结算所需的数据写接口（由调用方注入） */
export interface CandyWriter {
  setCandies(userId: string, n: number): Promise<void>;
  setChampions(userIds: string[]): Promise<void>;
  setLastSettled(weekKey: string): Promise<void>;
}

export interface SettleResult {
  /** 本次是否真正执行了结算（false = 本周已结算，幂等跳过） */
  settled: boolean;
  /** 上周冠军 userId 列表（并列全部包含；无人有糖则为空数组） */
  champions: string[];
}

/**
 * 懒结算：若 lastSettled 不是当前周，则执行结算。
 * 结算 = 找糖数最高的成员（并列全取）→ 写冠军 → 全员糖数清零。
 *
 * 并发说明：多人同时打开页面可能重复触发。为缩小"重复结算/冠军算错"窗口，
 * 先写入 lastSettled（占位锁）再读糖数结算；此后其他打开页面的用户会因
 * lastSettled 已更新而直接跳过。读-算-写仍非原子，理论上极小概率交错下
 * 冠军结果以最后一次写入为准（内部工具可接受，数据幂等）。
 */
export async function settleIfDue(
  lastSettled: string | null,
  currentWeek: string,
  memberIds: string[],
  reader: CandyReader,
  writer: CandyWriter,
): Promise<SettleResult> {
  if (lastSettled === currentWeek || memberIds.length === 0) {
    return { settled: false, champions: [] };
  }

  // 先占位：记录本周已结算，作为简易锁，防止并发重复结算；
  // 若后续任一步失败，catch 中回滚占位锁，使重试可再次结算。
  await writer.setLastSettled(currentWeek);

  try {
    const scores = new Map<string, number>();
    let max = 0;
    for (const id of memberIds) {
      const c = await reader.getCandies(id);
      scores.set(id, c);
      if (c > max) max = c;
    }

    const champions =
      max > 0
        ? [...scores.entries()].filter(([, c]) => c === max).map(([id]) => id)
        : [];

    for (const id of memberIds) {
      await writer.setCandies(id, 0);
    }

    // 仅在有冠军时写 champion_ids：并发交错下若读到被清零后的全 0 数据，
    // 不覆盖他人刚写入的真实冠军；若整周确无人有糖，旧冠军会继续展示，
    // 直至某周产生新冠军（语义可接受）
    if (champions.length > 0) {
      await writer.setChampions(champions);
    }

    return { settled: true, champions };
  } catch (e) {
    // 尽力回滚占位锁：网络短暂抖动后可重试重新结算；
    // 若回滚本身也失败（网络持续不可用），本周结算将被跳过，属极端情况下的可接受降级
    await writer.setLastSettled(lastSettled ?? "").catch(() => {});
    throw e;
  }
}
