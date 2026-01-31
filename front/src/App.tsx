import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { AuthProvider, useAuth } from './lib/auth'
import { ToastProvider } from './contexts/ToastContext'
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
import { FavoritesPage } from './pages/FavoritesPage'
import { AdminPage } from './pages/AdminPage'
import { AdminUserDetailPage } from './pages/AdminUserDetailPage'
import { AdminDictPage } from './pages/AdminDictPage'
import { AdminFinancePage } from './pages/AdminFinancePage'
import { AdminTaxonomyPage } from './pages/AdminTaxonomyPage'
import { AdminChatScenarioPage } from './pages/AdminChatScenarioPage'
import { AdminReadPage } from './pages/AdminReadPage'
import { SubscriptionPage } from './pages/SubscriptionPage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<LandingPage />} />
      <Route path="/dict" element={<DictPage />} />
      <Route path="/dict/:word" element={<DictPage />} />
      <Route path="/translate" element={<DictPage />} />
      <Route path="/chat" element={<ChatPage />} />
      <Route path="/chat/:chatId" element={<ChatPage />} />
      <Route path="/review" element={<ReviewPage />} />
      <Route path="/review/session" element={<ReviewSessionPage />} />
      <Route path="/review/session/:type" element={<ReviewSessionPage />} />
      <Route path="/stages" element={<StagesPage />} />
      <Route path="/stages/:id" element={<StageDetailPage />} />
      <Route path="/read" element={<ReadingPage />} />
      <Route path="/login" element={<HomePage />} />
      <Route path="/auth" element={<AuthorizePage />} />
      <Route path="/me" element={token ? <MePage /> : <Navigate to="/" replace />} />
      <Route path="/admin" element={<AdminPage />} />
      <Route path="/admin/users" element={<AdminPage />} />
      <Route path="/admin/users/:userId" element={<AdminUserDetailPage />} />
      <Route path="/admin/finance" element={<AdminFinancePage />} />
      <Route path="/admin/finance/orders" element={<AdminFinancePage />} />
      <Route path="/admin/finance/subscriptions" element={<AdminFinancePage />} />
      <Route path="/admin/dict" element={<AdminDictPage />} />
      <Route path="/admin/dict/:wordId" element={<AdminDictPage />} />
      <Route path="/admin/taxonomy" element={<AdminTaxonomyPage />} />
      <Route path="/admin/taxonomy/domains/:domainId" element={<AdminTaxonomyPage />} />
      <Route path="/admin/taxonomy/categories/:categoryId" element={<AdminTaxonomyPage />} />
      <Route path="/admin/chat-scenarios" element={<AdminChatScenarioPage />} />
      <Route path="/admin/read" element={<AdminReadPage />} />
      <Route path="/subscription" element={<SubscriptionPage />} />
      <Route path="/favorites" element={<FavoritesPage />} />
      <Route path="/feedback" element={<FeedbackPage />} />
      <Route path="/terms" element={<TermsPage />} />
      <Route path="/privacy" element={<PrivacyPage />} />
    </Routes>
  )
}

export default function App() {
  return (
    <AuthProvider>
      <ToastProvider>
        <BrowserRouter>
          <AppRoutes />
        </BrowserRouter>
      </ToastProvider>
    </AuthProvider>
  )
}
