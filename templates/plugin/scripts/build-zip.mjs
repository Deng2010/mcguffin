#!/usr/bin/env node
/**
 * 构建 + 打包：`npm run zip`
 *
 * 1. `tsc --noEmit` 类型检查
 * 2. `vite build`   产出 `dist/index.js`（及按需加载的 chunk）
 * 3. 把 `plugin.json` 与 `dist/` 下的全部文件打成 `dist/<id>.zip`
 *    - zip 内布局：`plugin.json` 在根目录，其余文件保持 `dist/` 下的相对路径
 *    - zip 文件名固定为 `<id>.zip`（id 取自 plugin.json），便于用
 *      `https://github.com/<owner>/<repo>/releases/latest/download/<id>.zip` 这类稳定地址安装
 */
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import AdmZip from "adm-zip";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const distDir = join(root, "dist");

/** 用 node 直接执行 node_modules 里的 CLI，避免 shell / PATH 差异。 */
function run(label, bin, args) {
  const binPath = join(root, "node_modules", bin);
  if (!existsSync(binPath)) {
    console.error(`✗ 找不到 ${bin}，请先在仓库根目录执行 npm install`);
    process.exit(1);
  }
  const res = spawnSync(process.execPath, [binPath, ...args], {
    cwd: root,
    stdio: "inherit",
  });
  if (res.status !== 0) {
    console.error(`✗ ${label} 失败（exit ${res.status ?? "?"}）`);
    process.exit(res.status ?? 1);
  }
}

run("tsc --noEmit", "typescript/bin/tsc", ["--noEmit"]);
run("vite build", "vite/bin/vite.js", ["build"]);

// ── 读取并校验 plugin.json ──
const manifestPath = join(root, "plugin.json");
if (!existsSync(manifestPath)) {
  console.error("✗ 缺少 plugin.json（插件清单，必需）");
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const id = String(manifest.id ?? "").trim();
if (!/^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/.test(id)) {
  console.error(`✗ plugin.json 的 id 不合法：${JSON.stringify(manifest.id)}`);
  process.exit(1);
}
const entry = String(manifest.entry ?? "index.js");
if (!existsSync(join(distDir, entry))) {
  console.error(`✗ 找不到入口产物 dist/${entry}（与 plugin.json 的 entry 不一致？）`);
  process.exit(1);
}

// ── 收集 dist/ 下的文件（相对路径，zip 内统一用 /）──
function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      out.push(...walk(full));
    } else if (!name.endsWith(".zip")) {
      out.push(relative(distDir, full).split(sep).join("/"));
    }
  }
  return out;
}
const files = walk(distDir).sort();

// ── 兜底检查：产物里是否残留宿主 import map 之外的裸模块名 ──
// 宿主 web/index.html 的 import map 只映射了下面这几个（指向 /plugin-sdk/*.js 垫片），
// 其余裸模块名浏览器一律解析不了（没有 node_modules 可查，也没有别名的 import map）。
const MAPPED_SPECIFIERS = new Set([
  "react",
  "react/jsx-runtime",
  "react/jsx-dev-runtime",
  "react-dom",
]);
const offenders = [];
const bareImports = new Set();
for (const rel of files) {
  if (!rel.endsWith(".js")) continue;
  const source = readFileSync(join(distDir, rel), "utf8");
  for (const match of source.matchAll(
    /(?:from|import)\s*["']([^"'./][^"']*)["']/g,
  )) {
    const specifier = match[1];
    if (MAPPED_SPECIFIERS.has(specifier)) continue;
    offenders.push(rel);
    bareImports.add(specifier);
  }
}
if (offenders.length > 0) {
  console.warn(
    `\n⚠ 产物里出现了宿主未映射的裸模块名 import：${[...bareImports].join(", ")}\n` +
      `  文件：${[...new Set(offenders)].join(", ")}\n` +
      `  宿主 import map 只映射了 ${[...MAPPED_SPECIFIERS].join(" / ")}，\n` +
      `  其它裸模块名浏览器解析不了，插件加载时会报 "Failed to resolve module specifier"。\n` +
      "  第三方库请保持默认打包（不要加进 rollupOptions.external），或改成相对路径导入。\n",
  );
}

// ── 打包 ──
const zip = new AdmZip();
zip.addFile("plugin.json", readFileSync(manifestPath));
for (const rel of files) {
  zip.addFile(rel, readFileSync(join(distDir, rel)));
}
const zipPath = join(distDir, `${id}.zip`);
zip.writeZip(zipPath);

const sizeKb = (statSync(zipPath).size / 1024).toFixed(1);
console.log(
  `\n✓ 已生成 ${relative(root, zipPath)}（${sizeKb} KiB，共 ${files.length + 1} 个文件）`,
);
console.log("  安装：管理后台 → 插件管理 → 「上传 .zip 文件」");
console.log(
  `  或「从 URL 安装」：https://github.com/<owner>/<repo>/releases/latest/download/${id}.zip`,
);
