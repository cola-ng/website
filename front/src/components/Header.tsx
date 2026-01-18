import { Link, useLocation } from 'react-router-dom'
import { User, LogOut, Settings } from 'lucide-react'

import { Button } from './ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from './ui/dropdown-menu'
import { useAuth } from '../lib/auth'

interface HeaderProps {
  showDictLink?: boolean
}

export function Header({ showDictLink = true }: HeaderProps) {
  const { token, user, clear } = useAuth()
  const location = useLocation()
  
  const getLoginUrl = () => {
    const currentPath = location.pathname + location.search
    if (currentPath === '/login' || currentPath.startsWith('/login?')) {
      return '/login'
    }
    return `/login?redirectTo=${encodeURIComponent(currentPath)}`
  }
  
  const loginUrl = getLoginUrl()

  return (
    <header className="border-b bg-white">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-3">
        <div className="flex items-center gap-3">
          <div className="h-8 w-8 flex items-center justify-center">
            <img src="/colang-logo.svg" alt="Colang" className="h-8 w-8" />
          </div>
          <div>
            <div className="text-base font-bold leading-tight text-gray-900">
              开朗英语
            </div>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {showDictLink && (
            <Button asChild variant="outline" size="sm">
              <Link to="/dict">词典</Link>
            </Button>
          )}
          {token ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="gap-2">
                  <User className="h-4 w-4" />
                  <span className="max-w-[120px] truncate">
                    {user?.name || user?.email}
                  </span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuLabel>
                  <div className="flex flex-col space-y-1">
                    <p className="text-sm font-medium leading-none">
                      {user?.name || '用户'}
                    </p>
                    <p className="text-xs leading-none text-muted-foreground">
                      {user?.email}
                    </p>
                  </div>
                </DropdownMenuLabel>
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild>
                  <Link to="/me" className="cursor-pointer">
                    <Settings className="mr-2 h-4 w-4" />
                    <span>账户设置</span>
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={clear} className="cursor-pointer text-destructive focus:text-destructive">
                  <LogOut className="mr-2 h-4 w-4" />
                  <span>登出</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : (
            <Button asChild size="sm">
              <Link to={loginUrl}>登录</Link>
            </Button>
          )}
        </div>
      </div>
    </header>
  )
}
