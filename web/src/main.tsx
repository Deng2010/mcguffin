import { StrictMode } from "react";
import * as React from "react";
import * as ReactDOM from "react-dom";
import { createRoot } from "react-dom/client";
import "./index.css";
import App from "./App.tsx";
import { definePlugin } from "./plugins/sdk/definePlugin";
import PluginSlots from "./plugins/sdk/PluginSlots";
import {
  PluginProvider,
  usePluginContext,
} from "./plugins/sdk/PluginContext";
import * as pluginData from "./plugins/sdk/data";
import * as pluginHooks from "./plugins/sdk/hooks";

// 暴露给 ZIP 安装的远程插件：插件入口（ESM）通过
//   const { definePlugin, React, ... } = window.__MCGUFFIN_SDK__
// 获取 SDK 能力，避免打包进各自的 React 副本。
// 类型由 plugins/sdk/global.d.ts + plugins/sdk/plugin-sdk.d.ts 提供：
// 下面这个赋值会逐成员做类型检查，SDK 增删成员必须同步更新规范声明。
// ReactDOM 供 web/public/plugin-sdk/react-dom.js 垫片使用（createPortal 等）。
window.__MCGUFFIN_SDK__ = {
  React,
  ReactDOM,
  definePlugin,
  PluginSlots,
  PluginProvider,
  usePluginContext,
  ...pluginData,
  ...pluginHooks,
};

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
