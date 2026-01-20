import { Link } from 'react-router-dom'
import { CheckCircle, Home, BookOpen, MessageSquare, RotateCcw, Theater, Mic } from 'lucide-react'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'
import { Button } from '../components/ui/button'
import { useAuth } from '../lib/auth'

export function AuthSuccessPage() {
  const { user } = useAuth()

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <div className="mx-auto flex min-h-screen max-w-4xl items-center justify-center p-6">
        <Card className="w-full max-w-2xl">
          <CardHeader className="text-center pb-2">
            <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
              <CheckCircle className="h-10 w-10 text-green-600" />
            </div>
            <CardTitle className="text-2xl">登录成功</CardTitle>
            <CardDescription className="text-base">
              欢迎回来，{user?.name || user?.email || '用户'}！
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="text-center text-sm text-muted-foreground">
              您已成功登录开朗英语。现在可以开始您的英语学习之旅了！
            </div>

            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
              <Link to="/" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-orange-100">
                    <Home className="h-5 w-5 text-orange-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">首页</span>
                </div>
              </Link>

              <Link to="/conversation" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-blue-100">
                    <MessageSquare className="h-5 w-5 text-blue-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">日常唠嗑</span>
                </div>
              </Link>

              <Link to="/review" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-green-100">
                    <RotateCcw className="h-5 w-5 text-green-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">温故知新</span>
                </div>
              </Link>

              <Link to="/scenes" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-purple-100">
                    <Theater className="h-5 w-5 text-purple-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">场景中心</span>
                </div>
              </Link>

              <Link to="/reading" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-red-100">
                    <Mic className="h-5 w-5 text-red-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">大声跟读</span>
                </div>
              </Link>

              <Link to="/dict" className="block">
                <div className="flex flex-col items-center gap-2 rounded-lg border bg-white p-4 transition-all hover:border-orange-300 hover:shadow-md">
                  <div className="flex h-10 w-10 items-center justify-center rounded-full bg-amber-100">
                    <BookOpen className="h-5 w-5 text-amber-600" />
                  </div>
                  <span className="text-sm font-medium text-gray-700">词典查询</span>
                </div>
              </Link>
            </div>

            <div className="flex justify-center gap-3 pt-2">
              <Button asChild>
                <Link to="/">进入首页</Link>
              </Button>
              <Button asChild variant="outline">
                <Link to="/me">账户设置</Link>
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
