import * as React from 'react'

import type { User } from './api'

type AuthState = {
  token: string | null
  user: User | null
  setAuth: (token: string, user: User) => void
  clear: () => void
}

const AuthContext = React.createContext<AuthState | null>(null)

const TOKEN_KEY = 'colang.token'
const USER_KEY = 'colang.user'

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [token, setToken] = React.useState<string | null>(() =>
    localStorage.getItem(TOKEN_KEY)
  )
  const [user, setUser] = React.useState<User | null>(() => {
    const raw = localStorage.getItem(USER_KEY)
    if (!raw) return null
    try {
      return JSON.parse(raw) as User
    } catch {
      return null
    }
  })

  const setAuth = React.useCallback((nextToken: string, nextUser: User) => {
    localStorage.setItem(TOKEN_KEY, nextToken)
    localStorage.setItem(USER_KEY, JSON.stringify(nextUser))
    setToken(nextToken)
    setUser(nextUser)
  }, [])

  const clear = React.useCallback(() => {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
    setToken(null)
    setUser(null)
  }, [])

  const value = React.useMemo<AuthState>(
    () => ({ token, user, setAuth, clear }),
    [token, user, setAuth, clear]
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth() {
  const ctx = React.useContext(AuthContext)
  if (!ctx) throw new Error('AuthProvider missing')
  return ctx
}

