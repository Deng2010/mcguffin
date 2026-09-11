import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// 某些环境会全局导出 NODE_ENV=production，导致 React 解析到生产构建
// （`act(...) is not supported in production builds of React`）。
// 测试强制使用 test 环境。
process.env.NODE_ENV = "test";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    env: {
      NODE_ENV: "test",
    },
  },
});
