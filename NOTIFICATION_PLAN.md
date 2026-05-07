# 网页通知（小铃铛）

## 目标
在 McGuffin 前端导航栏添加铃铛图标，显示未读通知数量，点击展示通知列表。当题目审核、建议状态变更等事件发生时，自动通知相关用户。

## 触发事件
- **题目审核通过/拒绝** → 通知出题人
- **题目发布** → 通知出题人  
- **建议状态变更**（resolved/closed）→ 通知建议作者
- **建议被回复** → 通知建议作者

## 实现方案
- 后端：`Notification` 数据模型 + `POST /api/notifications/read/:id` 标记已读 + `GET /api/notifications` 获取通知
- 前端：`NotificationContext` 轮询 + Navbar 铃铛图标 + 下拉通知面板
