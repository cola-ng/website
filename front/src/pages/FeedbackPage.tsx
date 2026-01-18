import { Link } from 'react-router-dom'

import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function FeedbackPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 via-indigo-50 to-purple-50">
      <Header />
      <main className="mx-auto max-w-4xl p-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-lg">我们重视您的反馈</CardTitle>
            <CardDescription className="text-sm">
              分享想法、报告错误或请求功能。我们会阅读每一条信息。
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2 text-xs text-muted-foreground">
            <p>目前，请通过入职时获得的支持渠道联系我们。</p>
            <p>我们正在开发应用内反馈表单。感谢您的耐心等待！</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
