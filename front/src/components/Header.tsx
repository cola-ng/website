import { Link, useLocation } from 'react-router-dom'
import { User, LogOut, Settings, Home, MessageSquare, RotateCcw, Theater, Mic, BookOpen, Menu } from 'lucide-react'
import { useState } from 'react'

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
import { cn } from '../lib/utils'

interface NavItem {
  label: string
  path: string
  icon: React.ReactNode
}

const navItems: NavItem[] = [
  { label: '首页', path: '/', icon: <Home className="h-4 w-4" /> },
  { label: '日常唠嗑', path: '/conversation', icon: <MessageSquare className="h-4 w-4" /> },
  { label: '复习巩固', path: '/review', icon: <RotateCcw className="h-4 w-4" /> },
  { label: '场景中心', path: '/scenes', icon: <Theater className="h-4 w-4" /> },
  { label: '大声跟读', path: '/reading', icon: <Mic className="h-4 w-4" /> },
  { label: '词典查询', path: '/dict', icon: <BookOpen className="h-4 w-4" /> },
]

export function Header() {
  const { token, user, clear } = useAuth()
  const location = useLocation()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)

  const getLoginUrl = () => {
    const currentPath = location.pathname + location.search
    if (currentPath === '/login' || currentPath.startsWith('/login?')) {
      return '/login'
    }
    return `/login?redirectTo=${encodeURIComponent(currentPath)}`
  }

  const loginUrl = getLoginUrl()

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/'
    }
    return location.pathname.startsWith(path)
  }

  return (
    <header className="border-b bg-white sticky top-0 z-50">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-2">
        {/* Logo */}
        <Link to="/" className="flex items-center gap-2 shrink-0">
          <div className="h-8 w-8 flex items-center justify-center">
            <img src="/colang-logo.svg" alt="Colang" className="h-8 w-8" />
          </div>
          <span className="text-base font-bold text-gray-900 hidden sm:block">
            开朗英语
          </span>
        </Link>

        {/* Desktop Navigation */}
        <nav className="hidden md:flex items-center gap-1">
          {navItems.map((item) => (
            <Link
              key={item.path}
              to={item.path}
              className={cn(
                "flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition-colors",
                isActive(item.path)
                  ? "bg-orange-100 text-orange-700"
                  : "text-gray-600 hover:text-gray-900 hover:bg-gray-100"
              )}
            >
              {item.icon}
              <span>{item.label}</span>
            </Link>
          ))}
        </nav>

        {/* Right side: User menu */}
        <div className="flex items-center gap-2">
          {/* Mobile menu button */}
          <Button
            variant="ghost"
            size="sm"
            className="md:hidden"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            <Menu className="h-5 w-5" />
          </Button>

          {token ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" size="sm" className="gap-2">
                  <User className="h-4 w-4" />
                  <span className="max-w-[80px] truncate hidden sm:block">
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

      {/* Mobile Navigation */}
      {mobileMenuOpen && (
        <nav className="md:hidden border-t bg-white px-4 py-2">
          <div className="grid grid-cols-3 gap-2">
            {navItems.map((item) => (
              <Link
                key={item.path}
                to={item.path}
                onClick={() => setMobileMenuOpen(false)}
                className={cn(
                  "flex flex-col items-center gap-1 px-2 py-2 rounded-md text-xs font-medium transition-colors",
                  isActive(item.path)
                    ? "bg-orange-100 text-orange-700"
                    : "text-gray-600 hover:text-gray-900 hover:bg-gray-100"
                )}
              >
                {item.icon}
                <span>{item.label}</span>
              </Link>
            ))}
          </div>
        </nav>
      )}
    </header>
  )
}
