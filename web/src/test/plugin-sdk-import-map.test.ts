import { describe, it, expect } from "vitest";
import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

// 插件运行时 import map 的一致性守卫：
//
//   web/index.html 的 <script type="importmap"> 把 `react` / `react/jsx-runtime`
//   / `react/jsx-dev-runtime` / `react-dom` 映射到 web/public/plugin-sdk/*.js 垫片，
//   垫片再转发到宿主暴露的 window.__MCGUFFIN_SDK__（同一份 React 实例）。
//
// 这类「配置 + 静态文件 + 宿主全局」三处联动的约定最容易悄悄漂移，故在此断言：
//   1. 映射的每个 URL 都真实存在，且位于 public/plugin-sdk/ 下；
//   2. 每个垫片确实引用 window.__MCGUFFIN_SDK__（而不是自带一份 React）；
//   3. 必需的裸模块名（react / jsx-runtime / jsx-dev-runtime / react-dom）都已映射；
//   4. main.tsx 上挂载的每个 SDK 成员都写进了规范类型声明。
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
const INDEX_HTML = join(REPO_ROOT, "web/index.html");
const PUBLIC_DIR = join(REPO_ROOT, "web/public");
const MAIN_TSX = join(REPO_ROOT, "web/src/main.tsx");
const CANONICAL_DTS = join(REPO_ROOT, MARKER);

function read(path: string): string {
  expect(existsSync(path), `文件不存在：${path}`).toBe(true);
  return readFileSync(path, "utf8");
}

/** 取出 index.html 里 import map 的 imports 映射。 */
function readImportMap(): Record<string, string> {
  const html = read(INDEX_HTML);
  const block = html.match(
    /<script\s+type="importmap"\s*>([\s\S]*?)<\/script>/,
  );
  expect(block, "web/index.html 缺少 import map（<script type=\"importmap\">）").not.toBeNull();
  const parsed = JSON.parse(block![1]) as { imports?: Record<string, string> };
  expect(parsed.imports, "import map 缺少 imports 字段").toBeTruthy();
  return parsed.imports!;
}

/** main.tsx 里 `window.__MCGUFFIN_SDK__ = { ... }` 的直接成员名（忽略展开）。 */
function directSdkMembers(): string[] {
  const source = read(MAIN_TSX);
  const block = source.match(/window\.__MCGUFFIN_SDK__\s*=\s*\{([\s\S]*?)\n\};/);
  expect(block, "main.tsx 里找不到 window.__MCGUFFIN_SDK__ 赋值").not.toBeNull();
  return block![1]
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("...") && !line.startsWith("//"))
    .map((line) => line.replace(/,$/, "").split(":")[0].trim())
    .filter((name) => /^[A-Za-z0-9_]+$/.test(name));
}

describe("插件 import map 与 React 垫片", () => {
  const imports = readImportMap();

  it("映射了全部必需的裸模块名", () => {
    for (const specifier of [
      "react",
      "react/jsx-runtime",
      "react/jsx-dev-runtime",
      "react-dom",
    ]) {
      expect(
        imports[specifier],
        `import map 缺少 "${specifier}" 映射：插件产物里的裸模块名会解析失败`,
      ).toBeTruthy();
    }
  });

  it("每个映射都指向 public/plugin-sdk/ 下真实存在的垫片", () => {
    for (const [specifier, url] of Object.entries(imports)) {
      expect(url, `"${specifier}" 应映射到 /plugin-sdk/*.js`).toMatch(
        /^\/plugin-sdk\/[A-Za-z0-9._-]+\.js$/,
      );
      const file = join(PUBLIC_DIR, url.replace(/^\//, ""));
      expect(
        existsSync(file),
        `"${specifier}" 映射到 ${url}，但 ${file} 不存在`,
      ).toBe(true);
    }
  });

  it("垫片一律转发到宿主实例，不自带 React", () => {
    for (const url of new Set(Object.values(imports))) {
      const source = read(join(PUBLIC_DIR, url.replace(/^\//, "")));
      expect(
        source.includes("window.__MCGUFFIN_SDK__"),
        `${url} 没有引用 window.__MCGUFFIN_SDK__：垫片必须复用宿主那份 React`,
      ).toBe(true);
      expect(
        /^\s*import\s/m.test(source),
        `${url} 出现了 import：public 静态垫片不能解析裸模块名`,
      ).toBe(false);
    }
  });

  it("main.tsx 的直接 SDK 成员都在规范类型声明里", () => {
    const members = directSdkMembers();
    // 兜底：正则失效时不至于「零断言通过」
    expect(members.length).toBeGreaterThanOrEqual(5);

    const dts = read(CANONICAL_DTS);
    const missing = members.filter(
      (name) => !new RegExp(`\\b${name}\\b`).test(dts),
    );
    expect(
      missing,
      `以下成员挂在 window.__MCGUFFIN_SDK__ 上，但没有写进 ${MARKER}：${missing.join(", ")}`,
    ).toEqual([]);
  });
});
