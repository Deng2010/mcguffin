/**
 * Team Members Plugin — 成员管理页面
 *
 * 以插件形式重写团队成员页，演示：
 * - definePlugin() 注册路由
 * - SDK 数据 API（pluginUserList / pluginUserMe）
 * - 与主应用共享 auth store 和 apiFetch
 */
import React from "react";
import { definePlugin } from "../sdk";

const plugin = definePlugin(
  {
    id: "team-members",
    name: "团队成员",
    version: "1.0.0",
    description: "团队成员列表、角色管理、入队审批",
    author: "mcguffin",
    routes: [
      {
        path: "/plugins/team",
        label: "团队",
        icon: "👥",
        nav_placement: "main",
        required_permission: "view_team",
      },
    ],
  },
  // 代码分割：页面组件按需加载
  React.lazy(() => import("./TeamMembersPage")),
);

export default plugin;
