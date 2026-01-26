import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { AuthProvider, useAuth } from './lib/auth'
import { AuthorizePage } from './pages/AuthorizePage'
import { DictPage } from './pages/DictPage'
import { HomePage } from './pages/HomePage'
import { MePage } from './pages/MePage'
import { FeedbackPage } from './pages/FeedbackPage'
import { TermsPage } from './pages/TermsPage'
import { PrivacyPage } from './pages/PrivacyPage'
import { LandingPage } from './pages/LandingPage'
import { ChatPage } from './pages/ChatPage'
import { ReviewPage } from './pages/ReviewPage'
import { ReviewSessionPage } from './pages/ReviewSessionPage'
import { StagesPage } from './pages/StagesPage'
import { StageDetailPage } from './pages/StageDetailPage'
import { ReadingPage } from './pages/ReadingPage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<LandingPage />} />
      <Route path="/dict" element={<DictPage />} />
      <Route path="/dict/:word" element={<DictPage />} />
      <Route path="/chat" element={<ChatPage />} />
      <Route path="/chat/:chatId" element={<ChatPage />} />
      <Route path="/review" element={<ReviewPage />} />
      <Route path="/review/session" element={<ReviewSessionPage />} />
      <Route path="/stages" element={<StagesPage />} />
      <Route path="/stages/:id" element={<StageDetailPage />} />
      <Route path="/read" element={<ReadingPage />} />
      <Route path="/login" element={<HomePage />} />
      <Route path="/auth" element={<AuthorizePage />} />
      <Route path="/me" element={token ? <MePage /> : <Navigate to="/" replace />} />
      <Route path="/feedback" element={<FeedbackPage />} />
      <Route path="/terms" element={<TermsPage />} />
      <Route path="/privacy" element={<PrivacyPage />} />
    </Routes>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <AppRoutes />
      </BrowserRouter>
    </AuthProvider>
  )
}
