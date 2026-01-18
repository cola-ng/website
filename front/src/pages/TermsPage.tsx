import { Header } from '../components/Header'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function TermsPage() {
  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />
      <main className="mx-auto max-w-4xl p-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-lg">服务条款</CardTitle>
            <CardDescription className="text-sm">简短、友好的规则总结</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-xs text-muted-foreground">
            <p>使用 CoLang，即表示您同意负责任地使用本服务并遵守适用法律。</p>
            <p>您对提交的内容和保持账户安全负责。</p>
            <p>我们可能会偶尔更新这些条款。继续使用即表示您接受变更。</p>
            <p>如果您对这些条款有疑问，请联系支持部门。</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
