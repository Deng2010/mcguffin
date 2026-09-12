import { describe, it, expect, beforeEach } from "vitest";
import { getToken, setToken, clearToken } from "../services/api";

describe("Token Management", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns null when no token is set", () => {
    expect(getToken()).toBeNull();
  });

  it("stores and retrieves a token", () => {
    setToken("my-test-token");
    expect(getToken()).toBe("my-test-token");
  });

  it("overwrites an existing token", () => {
    setToken("token-1");
    setToken("token-2");
    expect(getToken()).toBe("token-2");
  });

  it("clears the token", () => {
    setToken("temp-token");
    expect(getToken()).toBe("temp-token");
    clearToken();
    expect(getToken()).toBeNull();
  });

  it("handles empty string token", () => {
    setToken("");
    expect(getToken()).toBe("");
    clearToken();
    expect(getToken()).toBeNull();
  });
});

// 0.3.1 → 0.4.0 改了 localStorage 的 token key。
// 这些测试锁定「旧会话不被静默丢弃」这一契约，以及新 key 的优先级。
describe("Legacy token key migration (auth_token → mcguffin_token)", () => {
  const NEW_KEY = "mcguffin_token";
  const OLD_KEY = "auth_token";

  beforeEach(() => {
    localStorage.clear();
  });

  it("migrates a legacy token to the new key", () => {
    localStorage.setItem(OLD_KEY, "legacy-token");

    expect(getToken()).toBe("legacy-token");
    // 迁移后旧 key 必须被清掉，否则会留下两份凭据
    expect(localStorage.getItem(NEW_KEY)).toBe("legacy-token");
    expect(localStorage.getItem(OLD_KEY)).toBeNull();
  });

  it("never lets a stale legacy token override a newer session", () => {
    setToken("fresh-token");
    localStorage.setItem(OLD_KEY, "stale-token");

    expect(getToken()).toBe("fresh-token");
    // 新 key 优先时不应触碰遗留值，避免覆盖判断被污染
    expect(localStorage.getItem(OLD_KEY)).toBe("stale-token");
  });

  it("clears both keys on logout so the legacy token cannot resurface", () => {
    localStorage.setItem(OLD_KEY, "legacy-token");
    setToken("fresh-token");

    clearToken();

    expect(getToken()).toBeNull();
    expect(localStorage.getItem(OLD_KEY)).toBeNull();
  });

  it("returns null when neither key is present", () => {
    expect(getToken()).toBeNull();
  });
});

// 插件 SDK 曾经绕过 services 层直接读 "auth_token" 硬编码字符串，
// 导致 token key 改名后插件的文件 API 与 ZIP 安装静默发出 Bearer null。
// 这条测试锁死「凭据只有 api.ts 一个出口」的约定。
describe("Credential key contract", () => {
  it("plugin SDK reads the token through getToken(), not a literal key", async () => {
    const dataSource = await import("../plugins/sdk/data.ts?raw");
    const pluginServiceSource = await import("../services/plugin.service.ts?raw");

    for (const [name, source] of [
      ["plugins/sdk/data.ts", dataSource.default],
      ["services/plugin.service.ts", pluginServiceSource.default],
    ] as const) {
      expect(source, `${name} 不应再出现遗留 token key`).not.toContain(
        '"auth_token"',
      );
      expect(source, `${name} 不应直接读 localStorage 凭据`).not.toMatch(
        /localStorage\.getItem\(\s*["'](?:auth_token|mcguffin_token)["']\s*\)/,
      );
    }
  });
});
