/**
 * 榜榜糖 (Lollipop Rank) — 团队成员点糖插件
 *
 * 功能：
 *  - 给除自己外的团队成员点糖（🍭），类似点赞
 *  - 点糖成功率随该成员已有糖数递减，失败则清空其糖数
 *  - 每个用户每天最多点出 10 次糖（北京时间按天计数）
 *  - 每周一 0:00（北京时间）懒结算：最高糖数者成为冠军，
 *    插件页面用户名两侧展示 🍭（并列冠军全部展示），糖数清零进入新一周
 */
import React from "react";
import { definePlugin } from "../sdk";

const plugin = definePlugin(
  {
    id: "lollipop-rank",
    name: "榜榜糖",
    version: "1.0.0",
    description: "给团队成员点糖，每周结算冠军；糖越多成功率越低，失败清零",
    author: "mcguffin",
    permissions_needed: [
      "storage", // KV 存储：糖数 / 每日次数 / 结算状态
      "read:team", // 读取团队成员列表
    ],
    routes: [
      {
        path: "/plugins/lollipop",
        label: "榜榜糖",
        icon: "🍭",
        nav_placement: "main",
        required_permission: "view_team",
      },
    ],
  },
  React.lazy(() => import("./CandyPage")),
);

export default plugin;
