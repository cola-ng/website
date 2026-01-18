import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { AuthProvider, useAuth } from './lib/auth'
import { AuthorizePage } from './pages/AuthorizePage'
import { DictPage } from './pages/DictPage'
import { HomePage } from './pages/HomePage'
import { MePage } from './pages/MePage'
import { FeedbackPage } from './pages/FeedbackPage'
import { TermsPage } from './pages/TermsPage'
import { PrivacyPage } from './pages/PrivacyPage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<DictPage />} />
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
