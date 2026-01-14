import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { AuthProvider, useAuth } from './lib/auth'
import { DesktopAuthorizePage } from './pages/DesktopAuthorizePage'
import { HomePage } from './pages/HomePage'

function AppRoutes() {
  const { token } = useAuth()
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/desktop/authorize" element={<DesktopAuthorizePage />} />
      <Route path="/app" element={token ? <HomePage /> : <Navigate to="/" replace />} />
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
