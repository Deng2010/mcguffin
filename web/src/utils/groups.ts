import type { DiscussionReply } from '../types'

/** 将扁平的回复列表按 parent_id 分组为顶级回复→子回复树 */
export function groupReplies(replies: DiscussionReply[]) {
  const topLevel: DiscussionReply[] = []
  const childrenMap: Record<string, DiscussionReply[]> = {}
  for (const r of replies) {
    if (r.parent_id) {
      if (!childrenMap[r.parent_id]) childrenMap[r.parent_id] = []
      childrenMap[r.parent_id].push(r)
    } else {
      topLevel.push(r)
    }
  }
  return { topLevel, childrenMap }
}
