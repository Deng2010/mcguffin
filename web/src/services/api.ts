// ============== Token Management ==============

const TOKEN_KEY='***'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY)
}

// ============== API Error ==============

export class ApiError extends Error {
  status: number
  responseText: string

  constructor(status: number, responseText: string, message?: string) {
    super(message ?? `请求失败 (${status})`)
    this.status = status
    this.responseText = responseText
  }
}

// ============== API Client ==============

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options?.headers as Record<string, string> || {}),
  }
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }
  const res = await fetch(`/api${path}`, { ...options, headers })
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    console.error(`API error ${res.status}:`, text);
    throw new ApiError(res.status, text);
  }
  return res.json()
}
