import { Crown, Construction } from 'lucide-react'

import { Header } from '../components/Header'
import { Footer } from '../components/Footer'

export function SubscriptionPage() {
  return (
    <div className="min-h-screen bg-gray-50 flex flex-col">
      <Header />

      <main className="flex-1 max-w-4xl mx-auto px-4 py-8">
        <div className="mb-8 text-center">
          <h1 className="text-3xl font-bold text-gray-900 flex items-center justify-center gap-2">
            <Crown className="h-8 w-8 text-amber-500" />
            订阅会员
          </h1>
          <p className="text-gray-500 mt-2">解锁全部功能，无限学习</p>
        </div>

        <div className="flex flex-col items-center justify-center py-20">
          <Construction className="h-16 w-16 text-amber-400 mb-6" />
          <h2 className="text-xl font-semibold text-gray-700 mb-2">订阅功能尚未开放</h2>
          <p className="text-gray-500 text-center max-w-md">
            订阅功能正在筹备中，敬请期待。如有疑问请联系 <a href="mailto:contact@cola.ng" className="text-amber-600 hover:underline">contact@cola.ng</a>
          </p>
        </div>
      </main>

      <Footer />
    </div>
  )
}
