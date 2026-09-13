/**
 * 宿主为插件提供的 `react` 垫片（由 `web/index.html` 的 import map 映射）。
 *
 * 插件产物里的 `import { useState } from "react"`（JSX 编译产物里的
 * `react/jsx-runtime` 同理）会解析到这个文件，再转发到宿主通过
 * `window.__MCGUFFIN_SDK__.React` 暴露的**同一份** React 实例 ——
 * 插件因此既能用 JSX 与具名导入，又不会打包出第二份 React。
 *
 * ⚠️ 本文件是 public 静态资源，不参与主应用构建：
 *   - 不要在这里 import 任何东西（浏览器无法解析裸模块名）；
 *   - 新增导出时，React 侧必须真实存在同名成员，否则导出 `undefined`。
 */
const React = window.__MCGUFFIN_SDK__.React;

export default React;

export const {
  Children,
  Component,
  Fragment,
  Profiler,
  PureComponent,
  StrictMode,
  Suspense,
  cloneElement,
  createContext,
  createElement,
  createFactory,
  createRef,
  forwardRef,
  isValidElement,
  lazy,
  memo,
  startTransition,
  version,
  // Hooks
  useCallback,
  useContext,
  useDebugValue,
  useDeferredValue,
  useEffect,
  useId,
  useImperativeHandle,
  useInsertionEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  useSyncExternalStore,
  useTransition,
} = React;

/** React 18.3+ 才有 `act`；缺失时为 undefined（仅测试环境用得到）。 */
export const act = React.act;
