import { useParams } from 'react-router-dom'

export default function PlaceholderComponent() {
  const { pluginId } = useParams<{ pluginId: string }>()

  return (
    <div className="max-w-2xl mx-auto px-6 py-12">
      <h1 className="text-2xl font-bold text-gray-800 dark:text-gray-100 mb-4">
        插件: {pluginId}
      </h1>
      <p className="text-gray-600 dark:text-gray-300">
        此插件已注册但没有自定义前端组件。后端 API 可通过 /api/v1/plugins/{pluginId} 访问。
      </p>
    </div>
  )
}
