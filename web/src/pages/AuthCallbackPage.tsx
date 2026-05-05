import { useEffect } from 'react'
import { useNavigate, useLocation } from 'react-router-dom'
import { setToken, apiFetch } from '../api'
import type { User } from '../types'

export default function AuthCallbackPage() {
  const navigate = useNavigate()
  const location = useLocation()

  useEffect(() => {
    const params = new URLSearchParams(location.search)
    const token = params.get('token')
    const error = params.get('error')

    if (token) {
      setToken(token)
      apiFetch<User>('/user/me')
        .then(() => navigate('/', { replace: true }))
        .catch(() => navigate('/login', { replace: true }))
    } else if (error) {
      navigate(`/login?error=${error}`, { replace: true })
    } else {
      navigate('/login', { replace: true })
    }
  }, [location, navigate])

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-100 dark:bg-gray-950">
      <p className="text-gray-500 dark:text-gray-400">登录中...</p>
    </div>
  )
}
