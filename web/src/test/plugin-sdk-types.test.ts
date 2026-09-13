import { describe, it, expect } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

// 插件 SDK 的类型声明有两份：主仓库的规范声明，以及插件模板里的副本。
// 插件是独立 repo，副本只能靠 `cp` 同步，因此必须有测试防止两份漂移。
//
// 规范文件：web/src/plugins/sdk/plugin-sdk.d.ts
// 模板副本：templates/plugin/types/mcguffin-plugin-sdk.d.ts
//
// 测试通常在 `web/` 下运行（`bun run test`），但也支持从仓库根运行，
// 因此这里从工作目录向上找仓库根，而不是依赖固定的工作目录。
const MARKER = "web/src/plugins/sdk/plugin-sdk.d.ts";

function findRepoRoot(): string {
  for (const candidate of [".", "..", "../.."]) {
    const dir = resolve(candidate);
    if (existsSync(join(dir, MARKER))) return dir;
  }
  throw new Error(
    `找不到仓库根目录（缺少 ${MARKER}）：请在 web/ 或仓库根目录下运行测试`,
  );
}

const REPO_ROOT = findRepoRoot();
const CANONICAL_PATH = join(REPO_ROOT, MARKER);
const TEMPLATE_PATH = join(
  REPO_ROOT,
  "templates/plugin/types/mcguffin-plugin-sdk.d.ts",
);
const DATA_PATH = join(REPO_ROOT, "web/src/plugins/sdk/data.ts");
const HOOKS_PATH = join(REPO_ROOT, "web/src/plugins/sdk/hooks.ts");

const SYNC_HINT =
  "插件模板里的 SDK 类型副本与主仓库不一致，请执行：\n" +
  "  cp web/src/plugins/sdk/plugin-sdk.d.ts templates/plugin/types/mcguffin-plugin-sdk.d.ts";

function read(path: string): string {
  expect(existsSync(path), `文件不存在：${path}`).toBe(true);
  return readFileSync(path, "utf8");
}

/** 收集一个 SDK 模块的全部运行时导出名（`export function` / `export const`）。 */
function runtimeExports(source: string): string[] {
  const names = new Set<string>();
  for (const match of source.matchAll(
    /^export (?:async )?function ([A-Za-z0-9_]+)/gm,
  )) {
    names.add(match[1]);
  }
  for (const match of source.matchAll(/^export const ([A-Za-z0-9_]+)/gm)) {
    names.add(match[1]);
  }
  return [...names];
}

describe("插件 SDK 类型声明与模板副本一致", () => {
  it("两份 plugin-sdk.d.ts 逐字节相同", () => {
    const canonical = read(CANONICAL_PATH);
    const templateCopy = read(TEMPLATE_PATH);

    expect(templateCopy, SYNC_HINT).toBe(canonical);
  });

  it("规范声明覆盖 data.ts / hooks.ts 的全部运行时导出", () => {
    const canonical = read(CANONICAL_PATH);
    const exports = [
      ...runtimeExports(read(DATA_PATH)),
      ...runtimeExports(read(HOOKS_PATH)),
    ];

    // 兜底：正则失效时不至于「零断言通过」
    expect(exports.length).toBeGreaterThan(20);

    const missing = exports.filter(
      (name) => !new RegExp(`\\b${name}\\b`).test(canonical),
    );
    expect(
      missing,
      `以下 SDK 成员没有写进 web/src/plugins/sdk/plugin-sdk.d.ts：${missing.join(", ")}\n` +
        "（window.__MCGUFFIN_SDK__ 的成员必须逐个列出，否则插件拿不到类型）",
    ).toEqual([]);
  });
});
