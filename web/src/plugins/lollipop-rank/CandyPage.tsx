/**
 * 榜榜糖 — 插件页面
 *
 * 数据存储（KV，namespace 隔离）：
 *  - candies / {userId}        → 当前周糖数（数字字符串）
 *  - attempts / {日期}:{userId} → 该用户当天已用点糖次数（KV 读-改-写，按天自然过期）
 *  - week / last_settled       → 已结算的周标识（如 2026-W06，幂等）
 *  - week / champion_ids       → 上周冠军 userId 列表（JSON 数组）
 */
import { useCallback, useEffect, useState } from "react";
import {
  getPluginData,
  setPluginData,
  pluginUserMe,
  pluginUserList,
  type PluginTeamMember,
  type PluginUserInfo,
} from "../sdk";
import {
  bjDateKey,
  bjWeekKey,
  CHAMPION_EMOJI,
  DAILY_LIMIT,
  formatProbability,
  settleIfDue,
  successProbability,
  type CandyReader,
  type CandyWriter,
} from "./logic";

const PLUGIN_ID = "lollipop-rank";

const NS_CANDIES = "candies";
const NS_ATTEMPTS = "attempts";
const NS_WEEK = "week";
const KEY_LAST_SETTLED = "last_settled";
const KEY_CHAMPIONS = "champion_ids";

interface Feed {
  kind: "success" | "fail" | "error";
  text: string;
}

async function readCandies(userId: string): Promise<number> {
  const raw = await getPluginData(PLUGIN_ID, NS_CANDIES, userId);
  const n = parseInt(raw, 10);
  return Number.isFinite(n) && n > 0 ? n : 0;
}

function safeJsonArray(raw: string): string[] {
  try {
    const v = JSON.parse(raw);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

export default function CandyPage() {
  const [me, setMe] = useState<PluginUserInfo | null>(null);
  const [members, setMembers] = useState<PluginTeamMember[]>([]);
  const [candies, setCandies] = useState<Map<string, number>>(new Map());
  const [attemptsToday, setAttemptsToday] = useState(0);
  const [champions, setChampions] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [feed, setFeed] = useState<Feed | null>(null);

  // ── 初始加载：用户信息 + 成员 + 懒结算 + 各成员糖数 + 今日已用次数 ──

  const loadAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [meInfo, teamRes] = await Promise.all([
        pluginUserMe(PLUGIN_ID),
        pluginUserList(PLUGIN_ID),
      ]);
      setMe(meInfo);
      const memberList = teamRes.members ?? [];
      setMembers(memberList);
      const ids = memberList.map((m) => m.user_id);

      const reader: CandyReader = { getCandies: readCandies };
      const writer: CandyWriter = {
        setCandies: (id, n) =>
          setPluginData(PLUGIN_ID, NS_CANDIES, id, String(n)),
        setChampions: (list) =>
          setPluginData(PLUGIN_ID, NS_WEEK, KEY_CHAMPIONS, JSON.stringify(list)),
        setLastSettled: (week) =>
          setPluginData(PLUGIN_ID, NS_WEEK, KEY_LAST_SETTLED, week),
      };

      const lastSettledRaw = await getPluginData(
        PLUGIN_ID,
        NS_WEEK,
        KEY_LAST_SETTLED,
      );
      const lastSettled = lastSettledRaw || null;
      const currentWeek = bjWeekKey();

      const { champions: settledChampions } = await settleIfDue(
        lastSettled,
        currentWeek,
        ids,
        reader,
        writer,
      );

      // 冠军：刚结算则用返回值，本周已结算则从存储读取
      let champ: string[] = settledChampions;
      if (lastSettled === currentWeek) {
        const rawChamp = await getPluginData(PLUGIN_ID, NS_WEEK, KEY_CHAMPIONS);
        champ = rawChamp ? safeJsonArray(rawChamp) : [];
      }
      setChampions(champ);

      // 各成员当前周糖数
      const c = new Map<string, number>();
      for (const id of ids) {
        c.set(id, await readCandies(id));
      }
      setCandies(c);

      // 今日已用次数（按北京时间）
      const usedRaw = await getPluginData(
        PLUGIN_ID,
        NS_ATTEMPTS,
        `${bjDateKey()}:${meInfo.id}`,
      );
      setAttemptsToday(usedRaw ? parseInt(usedRaw, 10) || 0 : 0);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  // ── 点糖 ──

  const handleCandy = useCallback(
    async (member: PluginTeamMember) => {
      if (busy || !me || me.team_status !== "joined") return;
      setBusy(true);
      setFeed(null);
      const attemptsKey = `${bjDateKey()}:${me.id}`;
      try {
        // 1. 占一次今日额度（后端无原子计数器端点，用 KV 读-改-写；
        //    页面内 busy 锁串行化单页面点击，多 Tab/多设备并发为内部工具可接受边界）
        const usedRaw = await getPluginData(PLUGIN_ID, NS_ATTEMPTS, attemptsKey);
        const used = (usedRaw ? parseInt(usedRaw, 10) || 0 : 0) + 1;
        if (used > DAILY_LIMIT) {
          setAttemptsToday(DAILY_LIMIT);
          setFeed({
            kind: "error",
            text: `今日点糖次数已用完（${DAILY_LIMIT}/${DAILY_LIMIT}），明天再来吧`,
          });
          return;
        }
        await setPluginData(PLUGIN_ID, NS_ATTEMPTS, attemptsKey, String(used));
        setAttemptsToday(used);

        // 2. 读当前糖数并判定概率（随糖数递减，失败清零）
        const cur = candies.get(member.user_id) ?? 0;
        const ok = Math.random() < successProbability(cur);

        if (ok) {
          const next = cur + 1;
          await setPluginData(PLUGIN_ID, NS_CANDIES, member.user_id, String(next));
          setCandies((prev) => new Map(prev).set(member.user_id, next));
          setFeed({
            kind: "success",
            text: `🍭 点糖成功！${member.display_name} 的糖数 +1（共 ${next} 颗）`,
          });
        } else {
          await setPluginData(PLUGIN_ID, NS_CANDIES, member.user_id, "0");
          setCandies((prev) => new Map(prev).set(member.user_id, 0));
          setFeed({
            kind: "fail",
            text: `💥 点糖失败…${member.display_name} 的糖数被清空了`,
          });
        }
      } catch (e) {
        // 3. 异常时回滚额度，避免白扣次数（KV 读-改-写回滚）
        const curRaw = await getPluginData(PLUGIN_ID, NS_ATTEMPTS, attemptsKey).catch(
          () => "",
        );
        const cur = curRaw ? parseInt(curRaw, 10) || 0 : 0;
        if (cur > 0) {
          await setPluginData(PLUGIN_ID, NS_ATTEMPTS, attemptsKey, String(cur - 1)).catch(
            () => {},
          );
          setAttemptsToday(Math.max(0, cur - 1));
        }
        setFeed({ kind: "error", text: `操作失败：${String(e)}` });
      } finally {
        setBusy(false);
      }
    },
    [busy, me, candies],
  );

  // ── 派生数据 ──

  const remaining = Math.max(0, DAILY_LIMIT - attemptsToday);
  const ranked = members
    .filter((m) => m.user_id !== me?.id)
    .sort(
      (a, b) =>
        (candies.get(b.user_id) ?? 0) - (candies.get(a.user_id) ?? 0) ||
        a.display_name.localeCompare(b.display_name, "zh"),
    );
  const nameOf = (userId: string) =>
    members.find((m) => m.user_id === userId)?.display_name ?? userId;
  const isJoined = me?.team_status === "joined";

  // ── 加载 / 错误 ──

  if (loading) {
    return (
      <div className="p-6 text-center text-gray-400 dark:text-gray-500 py-12">
        加载榜榜糖数据...
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6 max-w-5xl mx-auto">
        <div className="p-6 bg-red-50 dark:bg-red-900/30 border border-red-300 dark:border-red-800 text-red-700 dark:text-red-300">
          <h2 className="text-lg font-semibold mb-2">加载失败</h2>
          <p className="text-sm mb-4">{error}</p>
          <button
            onClick={loadAll}
            className="px-4 py-1.5 text-sm border border-red-300 dark:border-red-800 hover:bg-red-100 dark:hover:bg-red-900/20"
          >
            重试
          </button>
        </div>
      </div>
    );
  }

  // ── 渲染 ──

  return (
    <div className="p-6 max-w-5xl mx-auto">
      <div className="flex items-center gap-3 mb-6">
        <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
          🍭 榜榜糖
        </h1>
        <span className="text-xs px-2 py-0.5 bg-pink-100 dark:bg-pink-900/30 text-pink-600 dark:text-pink-400">
          插件
        </span>
      </div>

      {/* 状态区 */}
      <div className="mb-6 p-4 bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700 flex flex-wrap items-center gap-x-6 gap-y-2">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          今日剩余点糖次数：
          <span
            className={`font-bold ml-1 ${
              remaining > 0
                ? "text-pink-600 dark:text-pink-400"
                : "text-red-500 dark:text-red-400"
            }`}
          >
            {remaining} / {DAILY_LIMIT}
          </span>
        </p>
        <p className="text-sm text-gray-600 dark:text-gray-400">
          本周：
          <span className="font-medium text-gray-800 dark:text-gray-200 ml-1">
            {bjWeekKey()}
          </span>
        </p>
        <p className="text-sm text-gray-400 dark:text-gray-500 ml-auto">
          北京时间 {bjDateKey()} · 每周一 0:00 结算
        </p>
      </div>

      {/* 上周冠军横幅 */}
      {champions.length > 0 && (
        <div className="mb-6 p-4 bg-pink-50 dark:bg-pink-900/30 border border-pink-300 dark:border-pink-800">
          <p className="text-pink-700 dark:text-pink-300 font-medium">
            🏆 上周冠军：
            {champions.map((id, i) => (
              <span key={id}>
                {i > 0 && "、"}
                <span className="font-bold">
                  {CHAMPION_EMOJI} {nameOf(id)} {CHAMPION_EMOJI}
                </span>
              </span>
            ))}
          </p>
        </div>
      )}

      {/* 反馈消息 */}
      {feed && (
        <div
          className={`mb-6 p-3 text-sm border ${
            feed.kind === "success"
              ? "bg-green-50 dark:bg-green-900/30 border-green-300 dark:border-green-800 text-green-700 dark:text-green-300"
              : feed.kind === "fail"
                ? "bg-amber-50 dark:bg-amber-900/30 border-amber-300 dark:border-amber-800 text-amber-700 dark:text-amber-300"
                : "bg-red-50 dark:bg-red-900/30 border-red-300 dark:border-red-800 text-red-700 dark:text-red-300"
          }`}
        >
          {feed.text}
        </div>
      )}

      {/* 非成员提示 */}
      {me && !isJoined && (
        <div className="mb-6 p-4 bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800">
          <p className="text-blue-700 dark:text-blue-300">
            仅团队成员可以参与点糖，请联系管理员申请加入团队。
          </p>
        </div>
      )}

      {/* 成员榜 */}
      <div className="space-y-2">
        {ranked.length === 0 ? (
          <div className="text-center py-12 text-gray-400 dark:text-gray-500">
            {members.length === 0 ? "暂无团队成员" : "团队里只有你自己，去找更多队友吧"}
          </div>
        ) : (
          ranked.map((m) => {
            const c = candies.get(m.user_id) ?? 0;
            const p = successProbability(c);
            const isChamp = champions.includes(m.user_id);
            const disabled = busy || !isJoined || remaining <= 0;
            return (
              <div
                key={m.user_id}
                className="flex items-center justify-between p-4 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 shadow"
              >
                <div className="flex items-center gap-3 min-w-0">
                  {m.avatar_url ? (
                    <img
                      src={m.avatar_url}
                      alt=""
                      className="w-10 h-10 rounded-full object-cover shrink-0"
                      onError={(e) => {
                        (e.target as HTMLImageElement).style.display = "none";
                      }}
                    />
                  ) : (
                    <div className="w-10 h-10 bg-gray-300 dark:bg-gray-700 rounded-full flex items-center justify-center text-gray-600 dark:text-gray-400 font-bold text-sm shrink-0">
                      {m.display_name?.charAt(0) || "?"}
                    </div>
                  )}
                  <div className="min-w-0">
                    <div className="font-medium text-gray-800 dark:text-gray-100 flex items-center gap-1">
                      {isChamp && (
                        <span className="shrink-0">{CHAMPION_EMOJI}</span>
                      )}
                      <span className="truncate">{m.display_name}</span>
                      {isChamp && (
                        <span className="shrink-0">{CHAMPION_EMOJI}</span>
                      )}
                    </div>
                    <div className="text-sm text-gray-500 dark:text-gray-400">
                      {m.username} · 🍬 {c} 颗糖
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3 shrink-0 ml-4">
                  <span className="text-sm text-gray-400 dark:text-gray-500 w-24 text-right">
                    成功率 {formatProbability(p)}
                  </span>
                  <button
                    disabled={disabled}
                    onClick={() => handleCandy(m)}
                    title={
                      !isJoined
                        ? "仅团队成员可点糖"
                        : remaining <= 0
                          ? "今日次数已用完"
                          : `当前成功率 ${formatProbability(p)}`
                    }
                    className="px-4 py-1.5 text-sm bg-pink-500 text-white hover:bg-pink-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                  >
                    🍭 点糖
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      <p className="mt-6 text-xs text-gray-400 dark:text-gray-500">
        共 {members.length} 名团队成员 · 糖数越多成功率越低，失败会清空糖数
      </p>
    </div>
  );
}
