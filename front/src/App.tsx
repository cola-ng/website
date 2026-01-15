import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { AuthProvider, useAuth } from './lib/auth'
import { AuthorizePage } from './pages/AuthorizePage'
import { HomePage } from './pages/HomePage'
import { MePage } from './pages/MePage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/auth" element={<AuthorizePage />} />
      <Route path="/app" element={token ? <HomePage /> : <Navigate to="/" replace />} />
      <Route path="/me" element={token ? <MePage /> : <Navigate to="/" replace />} />
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
