/**
 * 全局 `window.__MCGUFFIN_SDK__` 声明。
 *
 * 插件入口 ESM 通过它取用宿主能力：
 *   const { definePlugin, React } = window.__MCGUFFIN_SDK__;
 *
 * 类型主体在 `./plugin-sdk`（规范声明，需与 `templates/plugin/types/` 副本逐字节一致）。
 * 宿主侧赋值在 `web/src/main.tsx`，由本声明做类型检查（不匹配即报错）。
 */
import type { McGuffinPluginSdk } from "./plugin-sdk";

declare global {
  interface Window {
    __MCGUFFIN_SDK__: McGuffinPluginSdk;
  }
}

export {};
