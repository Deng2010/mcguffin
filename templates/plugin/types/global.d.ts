/**
 * 全局 `window.__MCGUFFIN_SDK__` 声明（宿主在 `web/src/main.tsx` 注入）。
 *
 * 与主仓库 `web/src/plugins/sdk/global.d.ts` 对应，只是类型主体的文件名不同。
 */
import type { McGuffinPluginSdk } from "./mcguffin-plugin-sdk";

declare global {
  interface Window {
    __MCGUFFIN_SDK__: McGuffinPluginSdk;
  }
}

export {};
