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
import { ConversationPage } from './pages/ConversationPage'
import { ReviewPage } from './pages/ReviewPage'
import { ReviewSessionPage } from './pages/ReviewSessionPage'
import { ScenesPage } from './pages/ScenesPage'
import { SceneDetailPage } from './pages/SceneDetailPage'
import { ReadingPage } from './pages/ReadingPage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<LandingPage />} />
      <Route path="/dict" element={<DictPage />} />
      <Route path="/conversation" element={<ConversationPage />} />
      <Route path="/review" element={<ReviewPage />} />
      <Route path="/review/session" element={<ReviewSessionPage />} />
      <Route path="/scenes" element={<ScenesPage />} />
      <Route path="/scenes/:id" element={<SceneDetailPage />} />
      <Route path="/reading" element={<ReadingPage />} />
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
