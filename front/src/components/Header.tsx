import { Link, useLocation } from 'react-router-dom'
import { User, LogOut, Settings, MessageSquare, RotateCcw, Theater, Mic, BookOpen, Menu, Award, Flame, Star, Trophy, Zap } from 'lucide-react'
import { useState, useEffect } from 'react'

import { Button } from './ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from './ui/dropdown-menu'
import { useAuth } from '../lib/auth'
import { cn } from '../lib/utils'
import { getUserProfileSummary, type UserProfileSummary } from '../lib/api'

interface NavItem {
  label: string
  path: string
  icon: React.ReactNode
}

const navItems: NavItem[] = [
  { label: '日常唠嗑', path: '/conversation', icon: <MessageSquare className="h-4 w-4" /> },
  { label: '温故知新', path: '/review', icon: <RotateCcw className="h-4 w-4" /> },
  { label: '角色扮演', path: '/stages', icon: <Theater className="h-4 w-4" /> },
  { label: '大声跟读', path: '/reading', icon: <Mic className="h-4 w-4" /> },
  { label: '词典查询', path: '/dict', icon: <BookOpen className="h-4 w-4" /> },
]

// Rarity colors for achievements
const rarityColors: Record<string, string> = {
  common: 'bg-gray-100 text-gray-600 border-gray-200',
  uncommon: 'bg-green-50 text-green-600 border-green-200',
  rare: 'bg-blue-50 text-blue-600 border-blue-200',
  epic: 'bg-purple-50 text-purple-600 border-purple-200',
  legendary: 'bg-amber-50 text-amber-600 border-amber-200',
}

export function Header() {
  const { token, user, clear } = useAuth()
  const location = useLocation()
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false)
  const [profile, setProfile] = useState<UserProfileSummary | null>(null)

  // Fetch user profile summary when logged in
  useEffect(() => {
    if (token) {
      getUserProfileSummary(token)
        .then(setProfile)
        .catch(() => setProfile(null))
    } else {
      setProfile(null)
    }
  }, [token])

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
    <header className="border-b bg-white sticky top-0 z-50 shadow-sm">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-4 py-2">
        {/* Logo */}
        <Link to="/" className="flex items-center gap-2 shrink-0 group">
          <div className="h-9 w-9 flex items-center justify-center transition-transform group-hover:scale-105">
            <img src="/colang-logo.svg" alt="Colang" className="h-9 w-9" />
          </div>
          <span className="text-lg font-bold bg-gradient-to-r from-orange-600 to-amber-500 bg-clip-text text-transparent hidden sm:block">
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
                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200",
                isActive(item.path)
                  ? "bg-gradient-to-r from-orange-500 to-amber-500 text-white shadow-sm"
                  : "text-gray-600 hover:text-orange-600 hover:bg-orange-50"
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
            size="icon"
            className="md:hidden h-8 w-8"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            <Menu className="h-5 w-5" />
          </Button>

          {token ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline" className="gap-2 h-9 pl-1 pr-3 rounded-full border-gray-200 hover:border-orange-300 hover:bg-orange-50">
                  <div className="h-7 w-7 rounded-full bg-gradient-to-br from-orange-500 to-amber-400 flex items-center justify-center ring-2 ring-white">
                    <User className="h-4 w-4 text-white" />
                  </div>
                  <span className="max-w-[80px] truncate hidden sm:block text-sm font-medium text-gray-700">
                    {user?.name || user?.email}
                  </span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-80 p-0 bg-white border shadow-xl">
                {/* User Info Header */}
                <div className="p-4 bg-gradient-to-br from-orange-50 to-amber-50 border-b">
                  <div className="flex items-center gap-3">
                    <div className="h-14 w-14 rounded-full bg-gradient-to-br from-orange-500 to-amber-400 flex items-center justify-center shadow-lg ring-4 ring-white">
                      <User className="h-7 w-7 text-white" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <p className="text-base font-bold text-gray-900 truncate">
                        {user?.name || '用户'}
                      </p>
                      <p className="text-sm text-gray-500 truncate">
                        {user?.email}
                      </p>
                    </div>
                  </div>
                </div>

                {/* Rank & XP Section */}
                {profile && (
                  <div className="p-4 border-b">
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <div
                          className="h-8 w-8 rounded-full flex items-center justify-center"
                          style={{ backgroundColor: profile.rank?.color || '#9CA3AF' }}
                        >
                          <Trophy className="h-4 w-4 text-white" />
                        </div>
                        <div>
                          <p className="text-sm font-bold" style={{ color: profile.rank?.color || '#374151' }}>
                            {profile.rank?.name_zh || '新手'}
                          </p>
                          <p className="text-xs text-gray-500">Lv.{profile.rank?.level || 1}</p>
                        </div>
                      </div>
                      <div className="text-right">
                        <div className="flex items-center gap-1">
                          <Zap className="h-4 w-4 text-amber-500" />
                          <span className="text-sm font-bold text-gray-900">{profile.total_xp}</span>
                          <span className="text-xs text-gray-500">XP</span>
                        </div>
                        {profile.current_streak_days > 0 && (
                          <div className="flex items-center gap-1 mt-0.5">
                            <Flame className="h-3 w-3 text-orange-500" />
                            <span className="text-xs text-orange-600 font-medium">
                              {profile.current_streak_days}天连续
                            </span>
                          </div>
                        )}
                      </div>
                    </div>

                    {/* XP Progress Bar */}
                    {profile.next_rank && (
                      <div className="space-y-1">
                        <div className="flex justify-between text-xs text-gray-500">
                          <span>距离 {profile.next_rank.name_zh}</span>
                          <span>{profile.xp_to_next_rank} XP</span>
                        </div>
                        <div className="h-2 bg-gray-100 rounded-full overflow-hidden">
                          <div
                            className="h-full bg-gradient-to-r from-orange-400 to-amber-400 rounded-full transition-all"
                            style={{
                              width: `${Math.min(100, ((profile.total_xp - (profile.rank?.min_xp || 0)) / (profile.next_rank.min_xp - (profile.rank?.min_xp || 0))) * 100)}%`
                            }}
                          />
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Achievements Section */}
                {profile && profile.recent_achievements.length > 0 && (
                  <div className="p-4 border-b">
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <Award className="h-4 w-4 text-amber-500" />
                        <span className="text-sm font-semibold text-gray-700">成就勋章</span>
                      </div>
                      <span className="text-xs text-gray-500">
                        {profile.completed_achievements}/{profile.total_achievements}
                      </span>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {profile.recent_achievements.slice(0, 4).map((achievement) => (
                        <div
                          key={achievement.code}
                          className={cn(
                            "px-2 py-1 rounded-full border text-xs font-medium flex items-center gap-1",
                            rarityColors[achievement.rarity] || rarityColors.common
                          )}
                          title={achievement.name_zh}
                        >
                          <Star className="h-3 w-3" />
                          <span className="truncate max-w-[80px]">{achievement.name_zh}</span>
                        </div>
                      ))}
                      {profile.completed_achievements > 4 && (
                        <div className="px-2 py-1 rounded-full bg-gray-100 text-gray-500 text-xs font-medium">
                          +{profile.completed_achievements - 4}
                        </div>
                      )}
                    </div>
                  </div>
                )}

                {/* Quick Stats */}
                {profile && (
                  <div className="grid grid-cols-3 gap-2 p-4 border-b">
                    <div className="text-center p-2 bg-orange-50 rounded-lg">
                      <p className="text-lg font-bold text-orange-600">{profile.current_streak_days}</p>
                      <p className="text-xs text-gray-500">连续天数</p>
                    </div>
                    <div className="text-center p-2 bg-green-50 rounded-lg">
                      <p className="text-lg font-bold text-green-600">{profile.completed_achievements}</p>
                      <p className="text-xs text-gray-500">已获成就</p>
                    </div>
                    <div className="text-center p-2 bg-blue-50 rounded-lg">
                      <p className="text-lg font-bold text-blue-600">{profile.rank?.level || 1}</p>
                      <p className="text-xs text-gray-500">当前等级</p>
                    </div>
                  </div>
                )}

                {/* Menu Items */}
                <div className="p-2">
                  <DropdownMenuItem asChild>
                    <Link to="/me" className="cursor-pointer flex items-center gap-2 px-3 py-2 rounded-lg">
                      <Settings className="h-4 w-4" />
                      <span>账户设置</span>
                    </Link>
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    onClick={clear}
                    className="cursor-pointer flex items-center gap-2 px-3 py-2 rounded-lg text-red-600 focus:text-red-600 focus:bg-red-50"
                  >
                    <LogOut className="h-4 w-4" />
                    <span>登出</span>
                  </DropdownMenuItem>
                </div>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : (
            <Button asChild className="h-8 px-4 text-sm rounded-lg bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600">
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
                  "flex flex-col items-center gap-1 px-2 py-2 rounded-lg text-xs font-medium transition-all",
                  isActive(item.path)
                    ? "bg-gradient-to-r from-orange-500 to-amber-500 text-white shadow-sm"
                    : "text-gray-600 hover:text-orange-600 hover:bg-orange-50"
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
