import { Header } from '../components/Header'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function PrivacyPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <main className="mx-auto max-w-4xl p-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-lg">隐私政策</CardTitle>
            <CardDescription className="text-sm">我们尊重您的隐私，保持极简原则</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-xs text-muted-foreground">
            <p>我们只收集提供服务所需的数据，以改善您的学习体验。</p>
            <p>您的账户详情和学习记录安全存储，绝不会出售。</p>
            <p>您可以随时联系支持部门请求删除您的账户数据。</p>
            <p>本政策可能会随着产品的发展而变化。对于重大更新，我们会通知您。</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
