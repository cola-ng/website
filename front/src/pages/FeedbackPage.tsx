import { Link } from 'react-router-dom'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function FeedbackPage() {
  return (
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="mx-auto flex max-w-4xl items-center justify-between px-6 py-4">
          <div>
            <div className="text-sm font-semibold">Feedback</div>
            <div className="text-xs text-muted-foreground">Tell us what to improve.</div>
          </div>
          <Button variant="outline" asChild>
            <Link to="/">Back</Link>
          </Button>
        </div>
      </header>
      <main className="mx-auto max-w-4xl p-6">
        <Card>
          <CardHeader>
            <CardTitle>We value your feedback</CardTitle>
            <CardDescription>
              Share ideas, report bugs, or request features. We read every message.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3 text-sm text-muted-foreground">
            <p>For now, please reach out through the support channel you were given during onboarding.</p>
            <p>We are working on an in-app feedback form. Thanks for your patience!</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
