import { Link } from 'react-router-dom'

export function Footer() {
  return (
    <footer className="mt-8 py-6 text-center text-sm text-gray-500">
      <p>© {new Date().getFullYear()} 开朗英语. All rights reserved.</p>
      <div className="mt-2 space-x-4">
        <Link to="/terms" className="hover:text-gray-700 hover:underline">
          服务条款
        </Link>
        <Link to="/privacy" className="hover:text-gray-700 hover:underline">
          隐私政策
        </Link>
      </div>
    </footer>
  )
}
