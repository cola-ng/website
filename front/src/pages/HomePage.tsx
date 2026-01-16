import { ChatPanel } from '../widgets/ChatPanel'
import { ProfilePanel } from '../widgets/ProfilePanel'
import { RecordsPanel } from '../widgets/RecordsPanel'
import { AuthCard } from '../widgets/AuthCard'
import { Button } from '../components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import { Link } from 'react-router-dom'

export function HomePage() {
  const { token, user, clear } = useAuth()

  if (!token) {
    return (
      <div className="min-h-screen bg-background">
        <div className="mx-auto flex min-h-screen max-w-6xl items-center justify-center p-6">
          <div className="w-full max-w-md">
            <AuthCard />
            <div className="mt-4 flex justify-center">
              <Button asChild variant="outline">
                <Link to="/dict">Dictionary</Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="border-b">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 rounded-lg bg-primary" />
            <div>
              <div className="text-sm font-semibold leading-tight">
                CoLang English Coach
              </div>
              <div className="text-xs text-muted-foreground leading-tight">
                Practice speaking without pressure
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <div className="text-right">
              <div className="text-sm font-medium leading-tight">
                {user?.name || user?.email}
              </div>
              <div className="text-xs text-muted-foreground leading-tight">
                Logged in
              </div>
            </div>
            <Button asChild variant="outline">
              <Link to="/dict">Dictionary</Link>
            </Button>
            <Button variant="outline" onClick={clear}>
              Log out
            </Button>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl p-6">
        <Tabs defaultValue="chat">
          <TabsList>
            <TabsTrigger value="chat">Chat</TabsTrigger>
            <TabsTrigger value="records">Records</TabsTrigger>
            <TabsTrigger value="profile">Profile</TabsTrigger>
          </TabsList>
          <TabsContent value="chat">
            <ChatPanel />
          </TabsContent>
          <TabsContent value="records">
            <RecordsPanel />
          </TabsContent>
          <TabsContent value="profile">
            <ProfilePanel />
          </TabsContent>
        </Tabs>
      </main>
    </div>
  )
}
