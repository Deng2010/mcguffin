/**
 * `react/jsx-runtime` 垫片 —— JSX 的**生产**自动运行时（由 import map 映射）。
 *
 * `tsconfig.json` 的 `jsx: "react-jsx"` 会让编译器产出具名导入：
 *   import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
 * 该裸模块名在浏览器里只能靠 import map 解析，这里把它接到宿主的 React 上。
 *
 * 签名与 React 官方运行时一致：`jsx(type, props, key)`，
 * `key` 单独传入时需要合回 props（React.createElement 从 config 里读 key）。
 */
const React = window.__MCGUFFIN_SDK__.React;

export const Fragment = React.Fragment;

function createElement(type, props, key) {
  if (key === undefined) return React.createElement(type, props);
  return React.createElement(type, { ...props, key });
}

export function jsx(type, props, key) {
  return createElement(type, props, key);
}

/** 多个静态子节点时编译器用 `jsxs`（与 `jsx` 行为一致）。 */
export const jsxs = jsx;
