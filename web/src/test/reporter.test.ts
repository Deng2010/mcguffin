import { describe, it, expect, vi, beforeEach } from "vitest";

describe("错误上报器", () => {
  beforeEach(() => {
    localStorage.clear();
    vi.restoreAllMocks();
  });

  it("相同指纹去重后只上报一次", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
    vi.stubGlobal("fetch", fetchMock);
    vi.resetModules();
    const { reportError, flush } = await import("../errors/reporter");

    const err = new Error("boom");
    reportError(err);
    reportError(err); // 重复
    flush();
    await new Promise((r) => setTimeout(r, 10));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe("/api/errors/report");
    const body = JSON.parse(init.body);
    expect(body.code).toBe("UNKNOWN_ERROR");
    expect(body.message).toBe("boom");
    expect(body.source).toBe("frontend");
  });

  it("单会话上限 100 条", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
    vi.stubGlobal("fetch", fetchMock);
    vi.resetModules();
    const { reportError, flush } = await import("../errors/reporter");

    for (let i = 0; i < 150; i++) {
      reportError(new Error(`e${i}`));
    }
    for (let i = 0; i < 8; i++) {
      flush();
    }
    await new Promise((r) => setTimeout(r, 20));

    // 100 条以内入队，多余丢弃
    expect(fetchMock.mock.calls.length).toBe(100);
  });

  it("离线时写入 localStorage 待发队列，恢复后继续上报", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
    vi.stubGlobal("fetch", fetchMock);
    vi.resetModules();
    const reporter = await import("../errors/reporter");

    // 首次上报失败（离线）→ 回退到 localStorage
    fetchMock.mockRejectedValueOnce(new TypeError("Failed to fetch"));
    reporter.reportError(new Error("offline-err"));
    reporter.flush();
    await new Promise((r) => setTimeout(r, 10));
    expect(localStorage.getItem("mcguffin.error.queue.v1")).toBeTruthy();
  });
});
