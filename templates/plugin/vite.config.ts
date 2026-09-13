import { defineConfig } from "vite";

/**
 * McGuffin 插件构建配置。
 *
 * 产物契约（见 docs/admin/plugins.md）：
 *   - 单入口 ESM，入口文件名固定为 `index.js`（与 plugin.json 的 `entry` 对应）；
 *   - React / ReactDOM 由宿主提供，**不打包进插件**（否则会出现多份 React 实例，
 *     hooks / context 全部失效）。插件里可以直接写
 *       import { useState } from "react";
 *     以及 JSX —— 宿主 web/index.html 的 import map 会把 `react` /
 *     `react/jsx-runtime` / `react/jsx-dev-runtime` / `react-dom` 映射到
 *     /plugin-sdk/*.js 垫片，垫片再转发到宿主的同一份实例；
 *   - 因此下面 external 里的这几个裸模块名是**故意留着的**，它们由 import map 解析，
 *     不要打进产物，也不要 alias 成别的东西；
 *   - 页面组件用动态 `import()` 时，产物是该入口同目录下的 chunk 文件
 *     （`Page-<hash>.js`），打包脚本会把 dist/ 下所有文件一起塞进 zip。
 */
export default defineConfig({
  build: {
    target: "esnext",
    lib: {
      entry: "src/index.tsx",
      formats: ["es"],
      fileName: () => "index.js",
    },
    rollupOptions: {
      // 绝不打包 React：宿主已有一份，运行时按 import map 解析到宿主实例。
      external: [
        "react",
        "react-dom",
        "react/jsx-runtime",
        "react/jsx-dev-runtime",
      ],
    },
  },
});
