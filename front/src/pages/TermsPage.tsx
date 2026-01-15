import { Link } from 'react-router-dom'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function TermsPage() {
  return (
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="mx-auto flex max-w-4xl items-center justify-between px-6 py-4">
          <div>
            <div className="text-sm font-semibold">Terms of Service</div>
            <div className="text-xs text-muted-foreground">Last updated: Jan 15, 2026</div>
          </div>
          <Button variant="outline" asChild>
            <Link to="/">Back</Link>
          </Button>
        </div>
      </header>
      <main className="mx-auto max-w-4xl p-6">
        <Card>
          <CardHeader>
            <CardTitle>Terms</CardTitle>
            <CardDescription>Short, friendly summary of the rules.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-muted-foreground">
            <p>By using CoLang, you agree to use the service responsibly and comply with applicable laws.</p>
            <p>You are responsible for the content you submit and for keeping your account secure.</p>
            <p>We may update these terms occasionally. Continued use means you accept the changes.</p>
            <p>Contact support if you have questions about these terms.</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
