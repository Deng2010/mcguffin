import { Outlet } from 'react-router-dom'
import { useSiteStore } from '../../stores/siteStore'
import Navbar from '../../components/Navbar'

export function Footer() {
  const siteInfo = useSiteStore(s => s.siteInfo)
  const version = siteInfo?.version || '0.1.0'
  return (
    <footer className="border-t border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 mt-12 py-4 px-6">
      <div className="max-w-6xl mx-auto text-center text-xs text-gray-400 dark:text-gray-500">
        Powered by{' '}
        <a
          href="https://github.com/Deng2010/mcguffin"
          target="_blank"
          rel="noopener noreferrer"
          className="text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 underline"
        >
          McGuffin
        </a>{' '}
        v{version}
      </div>
    </footer>
  )
}

/** Main site layout: Navbar + content + Footer */
export default function MainLayout() {
  return (
    <div className="min-h-screen bg-gray-100 dark:bg-gray-950 flex flex-col">
      <Navbar />
      <div className="flex-1">
        <Outlet />
      </div>
      <Footer />
    </div>
  )
}
