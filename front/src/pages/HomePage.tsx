import { ChatPanel } from '../widgets/ChatPanel'
import { ProfilePanel } from '../widgets/ProfilePanel'
import { RecordsPanel } from '../widgets/RecordsPanel'
import { AuthCard } from '../widgets/AuthCard'
import { Header } from '../components/Header'
import { Button } from '../components/ui/button'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../components/ui/tabs'
import { useAuth } from '../lib/auth'
import { Link } from 'react-router-dom'

export function HomePage() {
  const { token } = useAuth()

  if (!token) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
        <Header />
        <div className="mx-auto flex min-h-[calc(100vh-60px)] max-w-6xl items-center justify-center p-4">
          <div className="w-full max-w-md">
            <AuthCard />
            <div className="mt-3 flex justify-center">
              <Button asChild variant="outline" size="sm">
                <Link to="/dict">词典</Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-orange-50 via-amber-50 to-yellow-50">
      <Header />

      <main className="mx-auto max-w-6xl p-4">
        <Tabs defaultValue="chat">
          <TabsList className="mb-3">
            <TabsTrigger value="chat">对话</TabsTrigger>
            <TabsTrigger value="records">记录</TabsTrigger>
            <TabsTrigger value="profile">个人资料</TabsTrigger>
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
