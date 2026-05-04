import { describe, it, expect, beforeEach } from 'vitest'
import { getToken, setToken, clearToken } from '../api'

describe('Token Management', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('returns null when no token is set', () => {
    expect(getToken()).toBeNull()
  })

  it('stores and retrieves a token', () => {
    setToken('my-test-token')
    expect(getToken()).toBe('my-test-token')
  })

  it('overwrites an existing token', () => {
    setToken('token-1')
    setToken('token-2')
    expect(getToken()).toBe('token-2')
  })

  it('clears the token', () => {
    setToken('temp-token')
    expect(getToken()).toBe('temp-token')
    clearToken()
    expect(getToken()).toBeNull()
  })

  it('handles empty string token', () => {
    setToken('')
    expect(getToken()).toBe('')
    clearToken()
    expect(getToken()).toBeNull()
  })
})
