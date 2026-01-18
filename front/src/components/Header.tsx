import { Link } from 'react-router-dom'
import { Button } from './ui/button'
import { Sun } from 'lucide-react'
import { useAuth } from '../lib/auth'

interface HeaderProps {
  showDictLink?: boolean
}

export function Header({ showDictLink = true }: HeaderProps) {
  const { token, user, clear } = useAuth()

  return (
    <header className="border-b bg-white">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-3">
        <div className="flex items-center gap-3">
          <div className="h-8 w-8 rounded-lg bg-gradient-to-br from-amber-400 to-orange-500 flex items-center justify-center">
            <Sun className="w-5 h-5 text-white" />
          </div>
          <div>
            <div className="text-sm font-bold leading-tight text-gray-900">
              开朗英语
            </div>
            <div className="text-xs text-gray-500 leading-tight">
              CoLang English Coach
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {token ? (
            <>
              {showDictLink && (
                <Button asChild variant="outline" size="sm">
                  <Link to="/dict">词典</Link>
                </Button>
              )}
              <div className="text-right px-3">
                <div className="text-xs font-medium leading-tight text-gray-900">
                  {user?.name || user?.email}
                </div>
                <div className="text-xs text-gray-500 leading-tight">
                  已登录
                </div>
              </div>
              <Button variant="outline" size="sm" onClick={clear}>
                登出
              </Button>
            </>
          ) : (
            <>
              {showDictLink && (
                <Button asChild variant="outline" size="sm">
                  <Link to="/dict">词典</Link>
                </Button>
              )}
              <Button asChild size="sm">
                <Link to="/">登录</Link>
              </Button>
            </>
          )}
        </div>
      </div>
    </header>
  )
}
