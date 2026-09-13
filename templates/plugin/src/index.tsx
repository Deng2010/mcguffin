/**
 * 插件入口 —— 打包后即 zip 包内的 `index.js`（plugin.json 的 entry）。
 *
 * 入口只做一件事：通过宿主的全局 SDK **自注册**自己的路由 / 插槽。
 * 注意：这里刻意不 `import "react"`（运行时也不会真的 import），
 * React 一律从 `window.__MCGUFFIN_SDK__` 取用，保证与宿主是同一份实例。
 */
const { definePlugin, React } = window.__MCGUFFIN_SDK__;

/** 插件 id：必须与 plugin.json 的 id 一致（插件数据按 id 隔离）。 */
const PLUGIN_ID = "my-plugin";

definePlugin({
  id: PLUGIN_ID,
  name: "我的插件",
  version: "1.0.0",
  description: "McGuffin 插件模板",
  // 与 plugin.json 的 permissions_needed 保持一致；
  // 首次安装按声明授予，之后调整权限要由管理员在后台勾选。
  permissions_needed: ["storage"],
  routes: [
    {
      path: "/plugins/my",
      label: "我的插件",
      icon: "🔌",
      nav_placement: "main", // main = 主导航 | admin = 管理后台 | hidden = 只注册路由
      // 按需加载页面：动态 import 会产出同目录下的 chunk，打包脚本会一起塞进 zip。
      component: React.lazy(() => import("./Page")),
    },
  ],
});

export {};
