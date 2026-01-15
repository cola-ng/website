import { Link } from 'react-router-dom'

import { Button } from '../components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card'

export function PrivacyPage() {
  return (
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="mx-auto flex max-w-4xl items-center justify-between px-6 py-4">
          <div>
            <div className="text-sm font-semibold">Privacy Policy</div>
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
            <CardTitle>Privacy</CardTitle>
            <CardDescription>We respect your privacy and keep things minimal.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 text-sm text-muted-foreground">
            <p>We only collect the data needed to provide the service and improve your learning experience.</p>
            <p>Your account details and learning records are stored securely and never sold.</p>
            <p>You can request deletion of your account data at any time by contacting support.</p>
            <p>This policy may change as the product evolves. We will notify you of material updates.</p>
          </CardContent>
        </Card>
      </main>
    </div>
  )
}
