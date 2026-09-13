/**
 * `react-dom` 垫片（由 `web/index.html` 的 import map 映射）。
 *
 * 插件页面渲染在宿主的 React 树里，通常不需要 react-dom；但 UI 库常用
 * `createPortal`（弹窗 / 浮层）与 `flushSync`，所以一并提供，
 * 数据来源是宿主暴露的 `window.__MCGUFFIN_SDK__.ReactDOM`。
 *
 * 注意：`react-dom/client`（createRoot）**不在**映射范围内 ——
 * 插件不应自行创建 React 根，页面组件由宿主挂载。
 */
const ReactDOM = window.__MCGUFFIN_SDK__.ReactDOM;

export default ReactDOM;

export const {
  createPortal,
  flushSync,
  findDOMNode,
  hydrate,
  render,
  unmountComponentAtNode,
  unstable_batchedUpdates,
  version,
} = ReactDOM;
