import type { Discussion, DiscussionEmoji, DiscussionTag, PostDetail } from '../types'
import { apiFetch } from './api'

export interface CreatePostPayload {
  title: string
  content: string
  tags?: string[]
  emoji?: string | null
  pinned?: boolean
  team_only?: boolean
  status?: string
  visible_to?: string[]
  editable_by?: string[]
}

export interface ReplyPayload {
  content: string
  parent_id?: string | null
  reply_to?: string | null
}

export interface CommunityPostsResponse {
  posts: Discussion[]
  total: number
}

export async function getCommunityPosts(
  page: number,
  limit: number,
  tag?: string
): Promise<CommunityPostsResponse> {
  const params = new URLSearchParams()
  params.append('page', String(page))
  params.append('limit', String(limit))
  if (tag) params.append('tag', tag)
  return apiFetch<CommunityPostsResponse>(`/community/posts?${params.toString()}`)
}

export async function getAnnouncements(): Promise<Discussion[]> {
  return apiFetch<Discussion[]>('/announcements')
}

export async function createPost(body: CreatePostPayload): Promise<PostDetail> {
  return apiFetch<PostDetail>('/posts', {
    method: 'POST',
    body: JSON.stringify(body),
  })
}

export async function getPost(id: string): Promise<PostDetail> {
  return apiFetch<PostDetail>(`/posts/${id}`)
}

export async function updatePost(id: string, body: CreatePostPayload): Promise<PostDetail> {
  return apiFetch<PostDetail>(`/posts/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  })
}

export async function deletePost(id: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/posts/${id}`, { method: 'DELETE' })
}

export async function replyToPost(id: string, body: ReplyPayload): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/posts/${id}/reply`, {
    method: 'POST',
    body: JSON.stringify(body),
  })
}

export async function deleteReply(postId: string, replyId: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/posts/${postId}/reply/${replyId}`, { method: 'DELETE' })
}

export async function reactToPost(id: string, emoji: string): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/posts/${id}/react`, {
    method: 'POST',
    body: JSON.stringify({ emoji }),
  })
}

export async function reactToReply(
  postId: string,
  replyId: string,
  emoji: string
): Promise<Record<string, any>> {
  return apiFetch<Record<string, any>>(`/posts/${postId}/reply/${replyId}/react`, {
    method: 'POST',
    body: JSON.stringify({ emoji }),
  })
}

export async function getTags(): Promise<DiscussionTag[]> {
  return apiFetch<DiscussionTag[]>('/posts/tags')
}

export async function getEmojis(): Promise<DiscussionEmoji[]> {
  return apiFetch<DiscussionEmoji[]>('/posts/emojis')
}
