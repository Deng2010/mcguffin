interface Tab {
  id: string
  label: string
  count?: number
}

interface TabsProps {
  tabs: Tab[]
  activeTab: string
  onChange: (id: string) => void
  scrollable?: boolean
  wrap?: boolean
  className?: string
}

export default function Tabs({
  tabs,
  activeTab,
  onChange,
  scrollable = false,
  wrap = false,
  className = '',
}: TabsProps) {
  const overflow = scrollable ? 'overflow-x-auto' : wrap ? 'flex-wrap' : ''

  return (
    <div className={`flex items-center gap-1 border-b border-gray-300 dark:border-gray-700 mb-6 ${overflow} ${className}`}>
      {tabs.map((tab) => {
        const active = activeTab === tab.id
        return (
          <button
            key={tab.id}
            onClick={() => onChange(tab.id)}
            className={`px-4 py-2.5 text-sm font-medium border-b-2 whitespace-nowrap transition-colors ${
              active
                ? 'border-gray-800 dark:border-gray-100 text-gray-900 dark:text-gray-100'
                : 'border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-100'
            }`}
          >
            {tab.label}
            {tab.count !== undefined && (
              <span
                className={`ml-1.5 px-1.5 py-0.5 text-xs rounded ${
                  active
                    ? 'bg-gray-800 dark:bg-gray-600 text-white'
                    : 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400'
                }`}
              >
                {tab.count}
              </span>
            )}
          </button>
        )
      })}
    </div>
  )
}
