/**
 * Team Members Plugin — 成员管理页面
 *
 * 以插件形式重写团队成员页，演示：
 * - definePlugin() 注册路由 + 声明权限
 * - SDK 数据 API（pluginUserMe / pluginUserList）
 * - 后端根据插件申请的权限鉴权
 *
 * 权限说明：
 *   storage      — 不需要（本插件不存数据）
 *   read:team    — 列出团队成员
 *   write:team   — 审批入队、踢人（需要 manage_team 主权限配合）
 *   notify       — 不需要
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
    permissions_needed: [
      "read:team", // 列出团队成员
      "write:team", // 审批/移除（需 manage_team 主权限）
      "read:users", // 查看用户资料
    ],
    routes: [
      {
        path: "/plugins/team",
        label: "团队",
        nav_placement: "main",
        required_permission: "view_team",
      },
    ],
  },
  React.lazy(() => import("./TeamMembersPage")),
);

export default plugin;
