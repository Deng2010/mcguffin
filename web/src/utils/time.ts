/** 将 ISO 时间字符串格式化为相对时间或日期 */
export function formatTime(dateStr: string): string {
  const d = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 60) return `${mins}分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}小时前`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}天前`;
  return d.toLocaleDateString("zh-CN");
}

const DEFAULT_TZ = "UTC+8";

/** 解析形如 "UTC+8"、"UTC+08:00"、"UTC-5:30" 等时区为偏移分钟数。
 *  空值自动回退到 UTC+8；无法解析时按 0（UTC）处理。 */
export function timezoneToOffsetMinutes(timezone?: string | null): number {
  const tz = (timezone || DEFAULT_TZ).trim();
  const m = tz.match(/^UTC\s*([+-])(\d{1,2})(?::?(\d{1,2}))?$/i);
  if (!m) return 0;
  const sign = m[1] === "-" ? -1 : 1;
  const hours = parseInt(m[2], 10) || 0;
  const minutes = parseInt(m[3] || "0", 10) || 0;
  if (hours > 14 || minutes > 59) return 0;
  return sign * (hours * 60 + minutes);
}

const pad2 = (n: number) => String(n).padStart(2, "0");

/** 将编辑器中的日期 + 时 + 分组合为存储字符串 "YYYY-MM-DD HH:mm"。 */
export function buildContestTime(
  date: string,
  hour: number,
  minute: number,
): string {
  return `${date} ${pad2(hour)}:${pad2(minute)}`;
}

/** 解析比赛时间字符串，返回站点时区下的 {date, hour, minute}。
 *  兼容 "YYYY-MM-DD HH:mm" 与 ISO 8601（含 Z / 偏移）两种格式。 */
export function splitContestTime(
  value: string | null | undefined,
  timezone?: string | null,
): { date: string; hour: number; minute: number } {
  const offsetMinutes = timezoneToOffsetMinutes(timezone);
  const naive = (value || "").trim();

  // 本地日期时间格式：时间按站点时区解释，直接拆分即可。
  const naiveMatch = naive.match(/^(\d{4})-(\d{2})-(\d{2})[ T](\d{1,2}):(\d{2})$/);
  if (naiveMatch) {
    return {
      date: `${naiveMatch[1]}-${naiveMatch[2]}-${naiveMatch[3]}`,
      hour: parseInt(naiveMatch[4], 10),
      minute: parseInt(naiveMatch[5], 10),
    };
  }

  // ISO 8601 或其他可被 Date 解析的字符串：转换为站点时区后拆分。
  const parsed = new Date(naive);
  if (!Number.isNaN(parsed.getTime())) {
    const shifted = new Date(parsed.getTime() + offsetMinutes * 60000);
    return {
      date: `${shifted.getUTCFullYear()}-${pad2(shifted.getUTCMonth() + 1)}-${pad2(shifted.getUTCDate())}`,
      hour: shifted.getUTCHours(),
      minute: shifted.getUTCMinutes(),
    };
  }

  const now = new Date();
  return {
    date: `${now.getFullYear()}-${pad2(now.getMonth() + 1)}-${pad2(now.getDate())}`,
    hour: 0,
    minute: 0,
  };
}

/** 将比赛时间字符串转换为服务器可比对的 Unix 毫秒时间戳。
 *  "YYYY-MM-DD HH:mm" 按站点时区解释；ISO 字符串按自身偏移解释。 */
export function contestTimeToMs(
  value: string | null | undefined,
  timezone?: string | null,
): number | null {
  const naive = (value || "").trim();
  if (!naive) return null;

  const naiveMatch = naive.match(/^(\d{4})-(\d{2})-(\d{2})[ T](\d{1,2}):(\d{2})$/);
  if (naiveMatch) {
    const offsetMinutes = timezoneToOffsetMinutes(timezone);
    const y = parseInt(naiveMatch[1], 10);
    const mo = parseInt(naiveMatch[2], 10);
    const d = parseInt(naiveMatch[3], 10);
    const h = parseInt(naiveMatch[4], 10);
    const mi = parseInt(naiveMatch[5], 10);
    // 手工用 UTC 构造，再减去站点时区偏移（固定偏移时区无需处理 DST）。
    return Date.UTC(y, mo - 1, d, h, mi) - offsetMinutes * 60000;
  }

  const parsed = new Date(naive);
  return Number.isNaN(parsed.getTime()) ? null : parsed.getTime();
}

/** 根据服务器当前时间判定比赛状态（未开始/进行中/已结束）。 */
export function contestStatus(
  start: string | null | undefined,
  end: string | null | undefined,
  serverNowMs: number | null,
  timezone?: string | null,
): "not_started" | "running" | "ended" {
  const now = serverNowMs ?? Date.now();
  const s = contestTimeToMs(start, timezone);
  const e = contestTimeToMs(end, timezone);
  if (s === null || e === null) return "not_started";
  if (now < s) return "not_started";
  if (now > e) return "ended";
  return "running";
}
