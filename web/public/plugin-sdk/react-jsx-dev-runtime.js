/**
 * `react/jsx-dev-runtime` 垫片 —— JSX 的**开发**自动运行时（由 import map 映射）。
 *
 * 生产构建走 `react/jsx-runtime`，但 `vite build --watch` 之外的开发链路
 * （如 tsx/vitest 直接跑插件源码）可能会请求 dev runtime，这里一并接住。
 * `jsxDEV(type, props, key, isStaticChildren, source, self)` 的额外参数
 * 只用于调试信息，宿主不需要，忽略即可。
 */
const React = window.__MCGUFFIN_SDK__.React;

export const Fragment = React.Fragment;

export function jsxDEV(type, props, key) {
  if (key === undefined) return React.createElement(type, props);
  return React.createElement(type, { ...props, key });
}
