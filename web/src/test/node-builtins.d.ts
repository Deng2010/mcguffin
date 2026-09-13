/**
 * 最小 Node 内置模块声明（仅供测试使用）。
 *
 * `web/` 的依赖清单里没有 `@types/node`（不为此新增依赖），但个别测试需要用
 * `node:fs` 读取文件做一致性校验。这里只声明用到的 API，
 * **不要**在浏览器侧代码里 import 这些模块。
 */
declare module "node:fs" {
  /** 读取文件；`encoding` 为 "utf8" 时返回字符串。 */
  export function readFileSync(path: string, encoding: "utf8"): string;
  /** 判断路径是否存在。 */
  export function existsSync(path: string): boolean;
}

declare module "node:path" {
  /** 拼接路径片段。 */
  export function join(...parts: string[]): string;
  /** 解析为绝对路径（相对路径以进程工作目录为基准）。 */
  export function resolve(...parts: string[]): string;
}
