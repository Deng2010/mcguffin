import { Link } from 'react-router-dom'

export default function NotFoundPage() {
  return (
    <div className="flex flex-col items-center justify-center py-24 px-6 text-center">
      <div className="text-8xl font-bold text-gray-200 dark:text-gray-800 mb-4">404</div>
      <h1 className="text-xl font-semibold text-gray-700 dark:text-gray-200 mb-2">页面不存在</h1>
      <p className="text-gray-500 dark:text-gray-400 mb-8">您访问的页面未找到或已被移除</p>
      <Link
        to="/"
        className="px-6 py-2.5 bg-gray-800 text-white text-sm border border-gray-900 dark:bg-gray-700 dark:border-gray-600 hover:bg-gray-700 dark:hover:bg-gray-600"
      >
        返回首页
      </Link>
    </div>
  )
}
